use std::{
    collections::VecDeque,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard};
use shipyard::{
    AllStoragesViewMut, EntitiesViewMut, EntityId, Get, IntoWorkload, Unique, UniqueViewMut, View,
    ViewMut, World,
};

use crate::{
    config::WorldConfig,
    ecs::{
        components::{
            behavior::Behavior,
            guid::Guid,
            melee::Melee,
            movement::{Movement, MovementKind},
            nearby_players::NearbyPlayers,
            powers::Powers,
            quest_actor::QuestActor,
            spell_cast::SpellCast,
            threat_list::ThreatList,
            unit::Unit,
        },
        resources::DeltaTime,
        systems::{behavior, combat, inventory, melee, movement, powers, spell, unwind, updates},
    },
    entities::{
        creature::Creature, game_object::GameObject, internal_values::WrappedInternalValues,
        object_guid::ObjectGuid, player::Player, position::WorldPosition,
    },
    protocol::{
        self,
        opcodes::Opcode,
        packets::{MovementInfo, SmsgCreateObject},
        server::ServerMessage,
    },
    repositories::{
        character::CharacterRecord, creature::CreatureSpawnDbRecord,
        game_object::GameObjectSpawnDbRecord,
    },
    session::{session_holder::WrappedSessionHolder, world_session::WorldSession},
    shared::constants::{HighGuidType, NpcFlags, WeaponAttackType},
    SessionHolder,
};

use super::{
    entity_manager::{EntityManager, WrappedEntityManager},
    map_manager::MapKey,
    packet_broadcaster::{PacketBroadcaster, WrappedPacketBroadcaster},
    spatial_grid::{SpatialGrid, WrappedSpatialGrid},
    spell_effect_handler::WrappedSpellEffectHandler,
    terrain_manager::{TerrainManager, WrappedTerrainManager},
    world_context::{WorldContext, WrappedWorldContext},
};

pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;

pub struct Map {
    key: MapKey,
    world: Arc<ReentrantMutex<World>>,
    world_context: Arc<WorldContext>,
    session_holder: Arc<SessionHolder<ObjectGuid>>,
    entity_manager: Arc<EntityManager>,
    terrain_manager: Arc<TerrainManager>,
    spatial_grid: Arc<SpatialGrid>,
    packet_broadcaster: Arc<PacketBroadcaster>,
    visibility_distance: f32,
}

impl Map {
    pub fn new(
        key: MapKey,
        world_context: Arc<WorldContext>,
        terrain_manager: Arc<TerrainManager>,
        creature_spawns: Vec<CreatureSpawnDbRecord>,
        game_object_spawns: Vec<GameObjectSpawnDbRecord>,
        config: Arc<WorldConfig>,
    ) -> Arc<Map> {
        // TODO: pass this as a parameter?
        let visibility_distance = DEFAULT_VISIBILITY_DISTANCE;

        let session_holder = Arc::new(SessionHolder::new());
        let entity_manager = Arc::new(EntityManager::new());
        let spatial_grid = Arc::new(SpatialGrid::new(
            session_holder.clone(),
            entity_manager.clone(),
            world_context.clone(),
            visibility_distance,
        ));
        let packet_broadcaster = Arc::new(PacketBroadcaster::new(
            spatial_grid.clone(),
            entity_manager.clone(),
            visibility_distance,
        ));

        let map_record = world_context
            .data_store
            .get_map_record(key.map_id)
            .expect(
                format!(
                    "unable to find map record for map id {}, cannot instanciate map",
                    key.map_id
                )
                .as_str(),
            )
            .clone();

        let world = World::new();
        world.add_unique(DeltaTime::default());
        world.add_unique(WrappedSpellEffectHandler(
            world_context.spell_effect_handler.clone(),
        ));
        world.add_unique(WrappedWorldContext(world_context.clone()));
        world.add_unique(WrappedSpatialGrid(spatial_grid.clone()));
        world.add_unique(WrappedEntityManager(entity_manager.clone()));
        world.add_unique(VisibilityDistance(visibility_distance));
        world.add_unique(WrappedPacketBroadcaster(packet_broadcaster.clone()));
        world.add_unique(WrappedTerrainManager(terrain_manager.clone()));
        world.add_unique(WrappedSessionHolder(session_holder.clone()));
        world.add_unique(map_record);

        let workload = || {
            (
                unwind::unwind_creatures,
                updates::update_player_environment, // Must be before regenerate_powers
                movement::update_movement,
                combat::update_combat_state,
                behavior::tick,
                powers::regenerate_powers,
                combat::select_target,
                melee::attempt_melee_attack,
                spell::update_spell,
                updates::send_entity_update,
                inventory::send_inventory_update,
            )
                .into_workload()
        };
        world.add_workload(workload);

        let world = Arc::new(ReentrantMutex::new(world));

        let map = Map {
            key,
            world: world.clone(),
            world_context: world_context.clone(),
            session_holder: session_holder.clone(),
            entity_manager: entity_manager.clone(),
            terrain_manager,
            spatial_grid: spatial_grid.clone(),
            packet_broadcaster,
            visibility_distance,
        };

        for spawn in creature_spawns {
            let guid = ObjectGuid::with_entry(HighGuidType::Unit, spawn.entry, spawn.guid);

            let position = WorldPosition {
                map_key: key,
                zone: 1, // FIXME: calculate from position and terrain
                x: spawn.position_x,
                y: spawn.position_y,
                z: spawn.position_z,
                o: spawn.orientation,
            };

            let creature = Creature::from_spawn(&spawn, world_context.clone())
                .expect("unable to build InternalValues for creature from DB spawn");

            map.add_creature(&guid, creature, &position);
        }

        for spawn in game_object_spawns {
            let guid = ObjectGuid::with_entry(HighGuidType::Gameobject, spawn.entry, spawn.guid);

            let position = WorldPosition {
                map_key: key,
                zone: 1, // FIXME: calculate from position and terrain
                x: spawn.position_x,
                y: spawn.position_y,
                z: spawn.position_z,
                o: spawn.orientation,
            };

            let game_object = GameObject::from_spawn(&spawn, world_context.clone())
                .expect("unable to build InternalValues for game object from DB spawn");

            map.add_game_object(&guid, game_object, &position);
        }

        let map = Arc::new(map);

        map.world.lock().add_unique(HasPlayers(false));

        let map_id = key.map_id;
        let target_tick_time = config.world.game.target_tick_time_ms;
        thread::Builder::new()
            .name(format!("Map {map_id}"))
            .spawn(move || {
                let mut time = Instant::now();
                let mut update_times: VecDeque<u128> = VecDeque::with_capacity(200);
                let mut last_update_time_print = Instant::now();

                loop {
                    let tick_start_time = Instant::now();
                    let elapsed_since_last_tick = tick_start_time.duration_since(time);
                    time = tick_start_time;

                    {
                        let world_guard = world.lock();
                        world_guard.run(
                            |mut dt: UniqueViewMut<DeltaTime>,
                             mut hp: UniqueViewMut<HasPlayers>| {
                                // Update the delta time
                                *dt = DeltaTime(elapsed_since_last_tick);
                                // Update whether the map has players
                                *hp = HasPlayers(
                                    !world_guard.borrow::<View<Player>>().unwrap().is_empty(),
                                );
                            },
                        );
                        world_guard.run_workload(workload).unwrap();
                    }

                    let tick_duration = Instant::now().duration_since(tick_start_time);
                    if update_times.len() == 200 {
                        update_times.pop_front();
                    }

                    update_times.push_back(tick_duration.as_millis());

                    if tick_start_time.duration_since(last_update_time_print)
                        > Duration::from_secs(10)
                    {
                        let mean_tick_time =
                            update_times.iter().sum::<u128>() / update_times.len() as u128;
                        info!("Mean tick time on map {}: {mean_tick_time}", map_id);
                        last_update_time_print = tick_start_time;
                    }

                    thread::sleep(
                        Duration::from_millis(target_tick_time).saturating_sub(tick_duration),
                    );
                }
            })
            .unwrap();

        map
    }

    pub fn world(&self) -> ReentrantMutexGuard<World> {
        self.world.lock()
    }

    pub fn id(&self) -> u32 {
        self.key.map_id
    }

    pub fn lookup_entity_ecs(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.entity_manager.lookup(guid)
    }

    pub fn add_player_on_login(&self, session: Arc<WorldSession>, char_data: &CharacterRecord) {
        let player_guid = ObjectGuid::from_raw(char_data.guid).unwrap();

        if let Some(previous_session) = self
            .session_holder
            .insert_session(player_guid, session.clone())
        {
            warn!(
                "session from account {} was already on map {}",
                previous_session.account_id, self.key
            );
        }

        let player_entity_id = self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_powers: ViewMut<Powers>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>,
             mut vm_player: ViewMut<Player>,
             mut vm_movement: ViewMut<Movement>,
             (mut vm_spell, mut vm_nearby_players): (
                ViewMut<SpellCast>,
                ViewMut<NearbyPlayers>,
            )| {
                let player = Player::load_from_db(
                    session.account_id,
                    char_data.guid,
                    self.world_context.clone(),
                    session.clone(),
                );

                session.send_initial_spells(&player);
                session.send_initial_action_buttons(&player);
                session.send_initial_reputations(&player);

                let base_health_mana_record = self
                    .world_context
                    .data_store
                    .get_player_base_health_mana(char_data.class, char_data.level as u32)
                    .expect("unable to retrieve base health/mana for this class/level combination");

                let main_hand_attack_time = player.base_attack_time(
                    WeaponAttackType::MainHand,
                    self.world_context.data_store.clone(),
                );
                let off_hand_attack_time = player.base_attack_time(
                    WeaponAttackType::OffHand,
                    self.world_context.data_store.clone(),
                );
                let ranged_attack_time = player.base_attack_time(
                    WeaponAttackType::Ranged,
                    self.world_context.data_store.clone(),
                );

                let main_hand_base_damage = player.base_damage(
                    WeaponAttackType::MainHand,
                    self.world_context.data_store.clone(),
                );

                player.calculate_mana_regen();

                let entity_id = entities.add_entity(
                    (
                        &mut vm_guid,
                        &mut vm_powers,
                        &mut vm_melee,
                        &mut vm_unit,
                        &mut vm_wpos,
                        &mut vm_int_vals,
                        &mut vm_player,
                        &mut vm_movement,
                        &mut vm_spell,
                        &mut vm_nearby_players,
                    ),
                    (
                        Guid::new(player_guid, player.internal_values.clone()),
                        Powers::new(
                            player.internal_values.clone(),
                            base_health_mana_record.base_health,
                            base_health_mana_record.base_mana,
                        ),
                        Melee::new(
                            player.internal_values.clone(),
                            main_hand_base_damage.min(), // FIXME: still wildly inaccurate
                            main_hand_base_damage.max(), // FIXME: still wildly inaccurate
                            false,
                            [
                                main_hand_attack_time,
                                off_hand_attack_time,
                                ranged_attack_time,
                            ],
                        ),
                        Unit::new(
                            player.internal_values.clone(),
                            self.world_context.data_store.clone(),
                        ),
                        WorldPosition {
                            map_key: self.key,
                            zone: self
                                .terrain_manager
                                .get_area_id(char_data.position.x, char_data.position.y)
                                .unwrap_or(0),
                            x: char_data.position.x,
                            y: char_data.position.y,
                            z: char_data.position.z,
                            o: char_data.position.o,
                        },
                        WrappedInternalValues(player.internal_values.clone()),
                        player,
                        Movement::new(MovementKind::PlayerControlled),
                        SpellCast::new(),
                        NearbyPlayers::new(), // Player is always nearby a player
                    ),
                );

                self.entity_manager.insert(player_guid, entity_id);

                session.set_player_entity_id(entity_id);

                let movement = vm_movement.get(entity_id).ok().map(|m| {
                    m.build_update(
                        self.world_context.clone(),
                        &char_data.position.as_position(),
                    )
                });

                let player = vm_player.get(entity_id).unwrap();
                player.internal_values.write().reset_dirty();
                let smsg_create_object = player.build_create_object(movement, true);

                session.create_entity(&player_guid, smsg_create_object);

                entity_id
            },
        );

        self.spatial_grid
            .insert(char_data.position.as_position(), player_entity_id);

        {
            let entities_around: Vec<EntityId> = self.spatial_grid.search_ids_around_position(
                &char_data.position.as_position(),
                self.visibility_distance,
                true,
                Some(&player_entity_id),
            );
            for other_entity_id in entities_around {
                let other_entity_guid = self
                    .world()
                    .run(|v_guid: View<Guid>| v_guid[other_entity_id].0);
                // Broadcast the new player to nearby players
                let other_session = self.session_holder.get_session(&other_entity_guid);
                if let Some(other_session) = other_session {
                    let smsg_create_object: SmsgCreateObject;
                    {
                        let new_player_entity_id = session.player_entity_id().unwrap();
                        let world_guard = self.world.lock();
                        let (v_movement, v_player) = world_guard
                            .borrow::<(View<Movement>, View<Player>)>()
                            .unwrap();

                        let movement = v_movement.get(new_player_entity_id).ok().map(|m| {
                            m.build_update(
                                self.world_context.clone(),
                                &char_data.position.as_position(),
                            )
                        });

                        smsg_create_object = v_player
                            .get(new_player_entity_id)
                            .unwrap()
                            .build_create_object(movement, false);
                    }

                    other_session.create_entity(&player_guid, smsg_create_object);
                }

                // Make creatures aware of the player's presence
                self.world()
                    .run(|mut vm_nearby_players: ViewMut<NearbyPlayers>| {
                        NearbyPlayers::increment(other_entity_id, &mut vm_nearby_players);
                    });

                // Send nearby entities to the new player
                let smsg_create_object: Option<SmsgCreateObject>;
                {
                    let world_guard = self.world.lock();

                    let (v_movement, v_player, v_creature, v_game_object, v_wpos) = world_guard
                        .borrow::<(
                            View<Movement>,
                            View<Player>,
                            View<Creature>,
                            View<GameObject>,
                            View<WorldPosition>,
                        )>()
                        .unwrap();

                    let movement = v_movement.get(other_entity_id).ok().map(|m| {
                        m.build_update(
                            self.world_context.clone(),
                            &v_wpos[other_entity_id].as_position(),
                        )
                    });

                    if let Ok(player) = v_player.get(other_entity_id) {
                        smsg_create_object = Some(player.build_create_object(movement, false));
                    } else if let Ok(creature) = v_creature.get(other_entity_id) {
                        smsg_create_object = Some(creature.build_create_object(movement));
                    } else if let Ok(game_object) = v_game_object.get(other_entity_id) {
                        smsg_create_object =
                            Some(game_object.build_create_object_for(&v_player[player_entity_id]));
                    } else {
                        unreachable!("cannot generate SMSG_CREATE_OBJECT for this entity type");
                    }
                }

                if let Some(smsg) = smsg_create_object {
                    session.create_entity(&other_entity_guid, smsg);
                } else {
                    warn!(
                        "add_player_on_login: unable to generate a SmsgCreateObject for guid {:?}",
                        other_entity_id
                    );
                }
            }
        }
    }

    pub fn remove_player(&self, player_guid: &ObjectGuid) {
        let maybe_player_entity_id =
            self.world
                .lock()
                .run(|mut all_storages: AllStoragesViewMut| {
                    if let Some(entity_id) = self.entity_manager.remove(player_guid) {
                        all_storages.delete_entity(entity_id);
                        Some(entity_id)
                    } else {
                        error!(
                            "attempt to remove player {} who is not on map",
                            player_guid.counter()
                        );
                        None
                    }
                });

        if let Some(player_entity_id) = maybe_player_entity_id {
            let other_sessions = self.spatial_grid.sessions_nearby_entity(
                &player_entity_id,
                self.visibility_distance,
                false,
                false,
            );
            for other_session in other_sessions {
                other_session.destroy_entity(player_guid);
            }

            self.spatial_grid.delete(&player_entity_id);
        }

        self.session_holder.remove_session(player_guid);
    }

    pub fn add_creature(
        &self,
        creature_guid: &ObjectGuid,
        creature: Creature,
        wpos: &WorldPosition,
    ) {
        let creature_template = self
            .world_context
            .data_store
            .get_creature_template(creature.entry)
            .expect("creature template not found when adding to map");

        let creature_base_damage = self
            .world_context
            .data_store
            .get_creature_base_attributes(creature_template.unit_class, creature.real_level())
            .map(|attrs| {
                attrs.damage(
                    creature_template.expansion,
                    creature_template.damage_multiplier,
                )
            })
            .expect("creature base attributes not found");

        let creature_faction_template = self
            .world_context
            .data_store
            .get_faction_template_record(creature.template.faction_template_id);

        let creature_entity_id = self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_powers: ViewMut<Powers>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>,
             mut vm_creature: ViewMut<Creature>,
             mut vm_movement: ViewMut<Movement>,
             (mut vm_spell, mut vm_quest_actor, mut vm_behavior, mut vm_threat_list): (
                ViewMut<SpellCast>,
                ViewMut<QuestActor>,
                ViewMut<Behavior>,
                ViewMut<ThreatList>,
            )| {
                let entity_id = entities.add_entity(
                    (
                        &mut vm_guid,
                        &mut vm_powers,
                        &mut vm_melee,
                        &mut vm_unit,
                        &mut vm_wpos,
                        &mut vm_int_vals,
                        &mut vm_movement,
                        &mut vm_spell,
                        &mut vm_behavior,
                        &mut vm_threat_list,
                    ),
                    (
                        Guid::new(*creature_guid, creature.internal_values.clone()),
                        Powers::new(
                            creature.internal_values.clone(),
                            0, // TODO: creature base health
                            0, // TODO: creature base mana
                        ),
                        Melee::new(
                            creature.internal_values.clone(),
                            creature_base_damage,
                            creature_base_damage * (1. + creature_template.base_damage_variance),
                            true,
                            [
                                creature_template.melee_base_attack_time,
                                creature_template.melee_base_attack_time * 0.75 as u32,
                                creature_template.ranged_base_attack_time,
                            ],
                        ),
                        Unit::new(
                            creature.internal_values.clone(),
                            self.world_context.data_store.clone(),
                        ),
                        *wpos,
                        WrappedInternalValues(creature.internal_values.clone()),
                        Movement::new(creature.default_movement_kind),
                        SpellCast::new(),
                        // FIXME: Will need different behavior depending on some flags
                        Behavior::new_wild_monster(creature_faction_template),
                        ThreatList::new(),
                    ),
                );

                if creature.npc_flags.contains(NpcFlags::QuestGiver) {
                    let quest_relations = self
                        .world_context
                        .data_store
                        .get_quest_relations_for_creature(creature.entry);
                    entities.add_component(
                        entity_id,
                        &mut vm_quest_actor,
                        QuestActor::new(quest_relations),
                    );
                }

                entities.add_component(entity_id, &mut vm_creature, creature);

                self.entity_manager.insert(*creature_guid, entity_id);

                vm_creature
                    .get(entity_id)
                    .unwrap()
                    .internal_values
                    .write()
                    .reset_dirty();

                entity_id
            },
        );

        self.spatial_grid
            .insert(wpos.as_position(), creature_entity_id);

        // TODO: Don't attempt this during startup, it's pointless
        for session in self.spatial_grid.sessions_nearby_position(
            &wpos.as_position(),
            self.visibility_distance,
            true,
            None,
        ) {
            // Broadcast the new creature to nearby players
            let smsg_create_object: SmsgCreateObject;
            {
                let new_creature_entity_id = self.lookup_entity_ecs(creature_guid).unwrap();
                let world_guard = self.world.lock();
                let (v_movement, v_creature) = world_guard
                    .borrow::<(View<Movement>, View<Creature>)>()
                    .unwrap();

                let movement = v_movement
                    .get(new_creature_entity_id)
                    .ok()
                    .map(|m| m.build_update(self.world_context.clone(), &wpos.as_position()));

                smsg_create_object = v_creature
                    .get(new_creature_entity_id)
                    .unwrap()
                    .build_create_object(movement);
            }

            session.create_entity(creature_guid, smsg_create_object);
        }
    }

    pub fn add_game_object(
        &self,
        game_object_guid: &ObjectGuid,
        game_object: GameObject,
        wpos: &WorldPosition,
    ) {
        let entity_id = self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_game_object: ViewMut<GameObject>| {
                let entity_id = entities.add_entity(
                    &mut vm_guid,
                    Guid::new(*game_object_guid, game_object.internal_values.clone()),
                );
                entities.add_component(entity_id, &mut vm_game_object, game_object);

                vm_game_object
                    .get(entity_id)
                    .unwrap()
                    .internal_values
                    .write()
                    .reset_dirty();
                entity_id
            },
        );

        self.spatial_grid.insert(wpos.as_position(), entity_id);

        self.entity_manager.insert(*game_object_guid, entity_id);

        // TODO: Notify nearby players if map.has_players
    }

    pub fn visibility_distance(&self) -> f32 {
        self.visibility_distance
    }

    pub fn broadcast_movement(
        &self,
        origin_guid: &ObjectGuid,
        opcode: Opcode,
        movement_info: &MovementInfo,
    ) {
        if let Some(origin_entity_id) = self.lookup_entity_ecs(origin_guid) {
            for session in self.spatial_grid.sessions_nearby_entity(
                &origin_entity_id,
                self.visibility_distance,
                true,
                false,
            ) {
                session
                    .send_movement(opcode, origin_guid, movement_info)
                    .unwrap();
            }
        }
    }

    pub fn broadcast_packet<
        const OPCODE: u16,
        Payload: protocol::server::ServerMessagePayload<OPCODE>,
    >(
        &self,
        origin_guid: &ObjectGuid,
        packet: &ServerMessage<OPCODE, Payload>,
        range: Option<f32>,
        include_self: bool,
    ) {
        self.packet_broadcaster
            .broadcast_packet(origin_guid, packet, range, include_self);
    }

    pub fn get_session(&self, player_guid: &ObjectGuid) -> Option<Arc<WorldSession>> {
        self.session_holder.get_session(player_guid)
    }
}

#[derive(Unique)]
pub struct HasPlayers(pub bool);

impl std::ops::Deref for HasPlayers {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HasPlayers {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Unique)]
pub struct VisibilityDistance(pub f32);

impl std::ops::Deref for VisibilityDistance {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for VisibilityDistance {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
