use std::{
    cell::RefCell,
    collections::VecDeque,
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use parking_lot::{ReentrantMutex, ReentrantMutexGuard, RwLock};
use shipyard::{
    AllStoragesViewMut, EntitiesViewMut, EntityId, Get, IntoWorkload, Unique, UniqueViewMut, View,
    ViewMut, World,
};

use crate::{
    config::WorldConfig,
    ecs::{
        components::{
            applied_auras::AppliedAuras,
            behavior::Behavior,
            cooldowns::Cooldowns,
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
        systems::{
            aura, behavior, combat, cooldown, inventory, melee, movement, packets::process_packets,
            powers, spell, unwind, updates,
        },
    },
    entities::{
        attributes::Attributes,
        creature::Creature,
        game_object::GameObject,
        internal_values::{InternalValues, WrappedInternalValues},
        object_guid::ObjectGuid,
        player::Player,
        position::WorldPosition,
        update_fields::{PLAYER_END, UNIT_END},
    },
    protocol::{
        self,
        client::ClientMessage,
        opcodes::Opcode,
        packets::{MovementInfo, SmsgCreateObject},
        server::ServerMessage,
    },
    repositories::{
        character::CharacterRecord, creature::CreatureSpawnDbRecord,
        game_object::GameObjectSpawnDbRecord,
    },
    session::{
        session_holder::WrappedSessionHolder,
        world_session::{WorldSession, WorldSessionState},
    },
    shared::constants::{
        HighGuidType, NpcFlags, WeaponAttackType, CREATURE_BUFF_LIMIT, PLAYER_CONTROLLED_BUFF_LIMIT,
    },
    SessionHolder,
};

use super::{
    aura_effect_handler::WrappedAuraEffectHandler,
    entity_manager::{EntityManager, WrappedEntityManager},
    map_manager::MapKey,
    packet_broadcaster::{PacketBroadcaster, WrappedPacketBroadcaster},
    packet_queue::{PacketQueue, WrappedPacketQueue},
    spatial_grid::{SpatialGrid, WrappedSpatialGrid},
    spell_effect_handler::WrappedSpellEffectHandler,
    terrain_manager::{TerrainManager, WrappedTerrainManager},
    world_context::{WorldContext, WrappedWorldContext},
};

pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;

pub struct Map {
    key: MapKey,
    world: ReentrantMutex<RefCell<World>>,
    world_context: Arc<WorldContext>,
    session_holder: Arc<SessionHolder<ObjectGuid>>,
    entity_manager: Arc<EntityManager>,
    terrain_manager: Arc<TerrainManager>,
    spatial_grid: Arc<SpatialGrid>,
    packet_broadcaster: Arc<PacketBroadcaster>,
    packet_queue: Arc<PacketQueue>,
    visibility_distance: f32,
    running: Arc<AtomicBool>,
}

impl Map {
    pub fn new(
        key: MapKey,
        world_context: Arc<WorldContext>,
        terrain_manager: Arc<TerrainManager>,
        creature_spawns: Vec<CreatureSpawnDbRecord>,
        game_object_spawns: Vec<GameObjectSpawnDbRecord>,
    ) -> Map {
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
        let packet_queue = Arc::new(PacketQueue::new());

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
        world.add_unique(WrappedAuraEffectHandler(
            world_context.aura_effect_handler.clone(),
        ));
        world.add_unique(WrappedWorldContext(world_context.clone()));
        world.add_unique(WrappedSpatialGrid(spatial_grid.clone()));
        world.add_unique(WrappedEntityManager(entity_manager.clone()));
        world.add_unique(VisibilityDistance(visibility_distance));
        world.add_unique(WrappedPacketBroadcaster(packet_broadcaster.clone()));
        world.add_unique(WrappedTerrainManager(terrain_manager.clone()));
        world.add_unique(WrappedSessionHolder(session_holder.clone()));
        world.add_unique(map_record);
        world.add_unique(HasPlayers(false));
        world.add_unique(WrappedPacketQueue(packet_queue.clone()));

        let world = ReentrantMutex::new(RefCell::new(world));

        let map = Map {
            key,
            world,
            world_context: world_context.clone(),
            session_holder: session_holder.clone(),
            entity_manager: entity_manager.clone(),
            terrain_manager,
            spatial_grid: spatial_grid.clone(),
            packet_broadcaster,
            packet_queue,
            visibility_distance,
            running: Arc::new(AtomicBool::new(false)),
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

            map.add_creature(&guid, &spawn, &position, world_context.clone());
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

        map
    }

    pub fn start(&self, config: Arc<WorldConfig>) {
        self.running.store(true, Ordering::SeqCst);
        let target_tick_time = config.world.game.target_tick_time_ms;

        let map_update_workload = || {
            (
                unwind::unwind_creatures,
                updates::update_player_surroundings,
                updates::update_attributes,
                movement::update_movement,
                combat::update_combat_state,
                behavior::tick,
                aura::update_auras,
                powers::regenerate_powers,
                combat::select_target,
                melee::attempt_melee_attack,
                spell::update_spell,
                updates::send_entity_update,
                inventory::send_inventory_update,
                cooldown::send_cooldowns,
            )
                .into_workload()
        };
        self.world().add_workload(map_update_workload);

        let mut time = Instant::now();
        let mut update_times: VecDeque<u128> = VecDeque::with_capacity(200);
        let mut last_update_time_print = Instant::now();

        while self.running.load(Ordering::SeqCst) {
            let tick_start_time = Instant::now();
            let elapsed_since_last_tick = tick_start_time.duration_since(time);
            time = tick_start_time;

            {
                let world_guard = self.world();
                world_guard.run(
                    |mut dt: UniqueViewMut<DeltaTime>, mut hp: UniqueViewMut<HasPlayers>| {
                        // Update the delta time
                        *dt = DeltaTime(elapsed_since_last_tick);
                        // Update whether the map has players
                        *hp = HasPlayers(!world_guard.borrow::<View<Player>>().unwrap().is_empty());
                    },
                );

                world_guard.run(process_packets);
                world_guard.run_workload(map_update_workload).unwrap();
            }

            let tick_duration = Instant::now().duration_since(tick_start_time);
            if update_times.len() == 200 {
                update_times.pop_front();
            }

            update_times.push_back(tick_duration.as_millis());

            if tick_start_time.duration_since(last_update_time_print) > Duration::from_secs(10) {
                let mean_tick_time = update_times.iter().sum::<u128>() / update_times.len() as u128;
                info!("Mean tick time on map {}: {mean_tick_time}", self.key);
                last_update_time_print = tick_start_time;
            }

            thread::sleep(Duration::from_millis(target_tick_time).saturating_sub(tick_duration));
        }
    }

    pub fn world(&self) -> WorldRef {
        let guard = self.world.lock();

        WorldRef {
            guard,
            _world: PhantomData,
        }
    }

    pub fn id(&self) -> u32 {
        self.key.map_id
    }

    pub fn lookup_entity_ecs(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.entity_manager.lookup(guid)
    }

    pub fn add_player_on_login(
        self: Arc<Map>,
        session: Arc<WorldSession>,
        char_data: &CharacterRecord,
    ) {
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

        self.world().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_powers: ViewMut<Powers>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>,
             mut vm_player: ViewMut<Player>,
             mut vm_movement: ViewMut<Movement>,
             (
                mut vm_spell,
                mut vm_nearby_players,
                mut vm_cooldowns,
                mut vm_app_auras,
                mut vm_attributes,
            ): (
                ViewMut<SpellCast>,
                ViewMut<NearbyPlayers>,
                ViewMut<Cooldowns>,
                ViewMut<AppliedAuras>,
                ViewMut<Attributes>,
            )| {
                let internal_values =
                    Arc::new(RwLock::new(InternalValues::new(PLAYER_END as usize)));
                let mut attributes = Attributes::new(internal_values.clone());

                let player = Player::load_from_db(
                    session.account_id,
                    char_data.guid,
                    self.world_context.clone(),
                    session.clone(),
                    &mut attributes,
                    internal_values.clone(),
                );
                let spell_cooldowns = Player::load_spell_cooldowns_from_db(
                    char_data.guid,
                    self.world_context.clone(),
                );

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
                        &mut vm_movement,
                        (
                            &mut vm_spell,
                            &mut vm_nearby_players,
                            &mut vm_cooldowns,
                            &mut vm_app_auras,
                            &mut vm_attributes,
                        ),
                        &mut vm_player,
                    ),
                    (
                        Guid::new(player_guid, player.internal_values.clone()),
                        Powers::new(player.internal_values.clone()),
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
                        Movement::new(MovementKind::PlayerControlled),
                        (
                            SpellCast::new(),
                            NearbyPlayers::new(), // Player is always nearby a player
                            Cooldowns::new(spell_cooldowns),
                            AppliedAuras::new(
                                PLAYER_CONTROLLED_BUFF_LIMIT,
                                player.internal_values.clone(),
                            ),
                            attributes,
                        ),
                        player,
                    ),
                );
                session.set_player_entity_id(entity_id);
            },
        );

        self.add_player(session.clone());

        session.set_state(WorldSessionState::InWorld);
    }

    pub fn transfer_player_from_other_map(self: Arc<Map>, session: Arc<WorldSession>) {
        session.clone().set_state(WorldSessionState::InMapTransfer);

        let Some(origin_map) = session.current_map() else {
            error!("transfer_player_from_other_map: player has no current map");
            return;
        };
        let Some(player_entity_id) = session.player_entity_id() else {
            error!("transfer_player_from_other_map: player has no entity id");
            return;
        };
        let Some(player_guid) = session.player_guid() else {
            error!("transfer_player_from_other_map: player has no guid");
            return;
        };

        origin_map
            .world()
            .move_entity(&mut self.world(), player_entity_id);

        // Removal from previous map
        {
            let other_sessions = origin_map.spatial_grid.sessions_nearby_entity(
                &player_entity_id,
                origin_map.visibility_distance,
                false,
                false,
            );
            for other_session in other_sessions {
                other_session.destroy_entity(&player_guid);
            }

            origin_map.spatial_grid.delete(&player_entity_id);
            origin_map.entity_manager.remove(&player_guid);
            origin_map.session_holder.remove_session(&player_guid);
        }

        // Addition to new map (TODO: deduplicate from add_player_on_login)
        {
            session.set_map(self.clone());

            if let Some(previous_session) = self
                .session_holder
                .insert_session(player_guid, session.clone())
            {
                warn!(
                    "session from account {} was already on map {}",
                    previous_session.account_id, self.key
                );
            }

            self.add_player(session.clone());
        }

        session.clone().set_state(WorldSessionState::InWorld);
    }

    fn add_player(self: &Arc<Map>, session: Arc<WorldSession>) {
        let Some(player_entity_id) = session.player_entity_id() else {
            error!("Map::add_player: session has no player EntityId");
            return;
        };
        let Some(player_guid) = session.player_guid() else {
            error!("transfer_player_from_other_map: player has no guid");
            return;
        };

        let player_position = self
            .world()
            .run(|v_wpos: View<WorldPosition>| v_wpos[player_entity_id].clone())
            .as_position();

        self.world().run(
            |v_player: View<Player>, v_movement: View<Movement>, v_cooldowns: View<Cooldowns>| {
                let player = v_player.get(player_entity_id).unwrap();
                let spell_cooldowns = v_cooldowns.get(player_entity_id).unwrap();

                session.send_initial_spells(player, spell_cooldowns, self.world_context.clone());
                session.send_initial_action_buttons(player);
                session.send_initial_reputations(player);

                self.entity_manager.insert(player_guid, player_entity_id);

                let movement = v_movement
                    .get(player_entity_id)
                    .ok()
                    .map(|m| m.build_update(self.world_context.clone(), &player_position));

                player.internal_values.write().reset_dirty();
                let smsg_create_object = player.build_create_object(movement, true);

                session.create_entity(&player_guid, smsg_create_object);
            },
        );

        self.spatial_grid.insert(player_position, player_entity_id);

        let entities_around: Vec<EntityId> = self.spatial_grid.search_ids_around_position(
            &player_position,
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
                    let world_guard = self.world();
                    let (v_movement, v_player) = world_guard
                        .borrow::<(View<Movement>, View<Player>)>()
                        .unwrap();

                    let movement = v_movement
                        .get(new_player_entity_id)
                        .ok()
                        .map(|m| m.build_update(self.world_context.clone(), &player_position));

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
                let world_guard = self.world();

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

    pub fn remove_player_on_logout(&self, player_guid: &ObjectGuid) {
        let maybe_player_entity_id = self.world().run(|mut all_storages: AllStoragesViewMut| {
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
        spawn: &CreatureSpawnDbRecord,
        wpos: &WorldPosition,
        world_context: Arc<WorldContext>,
    ) {
        let internal_values = Arc::new(RwLock::new(InternalValues::new(UNIT_END as usize)));
        let mut attributes = Attributes::new(internal_values.clone());
        let creature = Creature::from_spawn(
            internal_values.clone(),
            &spawn,
            world_context.clone(),
            &mut attributes,
        )
        .expect("unable to build InternalValues for creature from DB spawn");

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

        let creature_entity_id = self.world().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_powers: ViewMut<Powers>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>,
             mut vm_creature: ViewMut<Creature>,
             mut vm_movement: ViewMut<Movement>,
             (
                mut vm_spell,
                mut vm_quest_actor,
                mut vm_behavior,
                mut vm_threat_list,
                mut vm_app_auras,
                mut vm_attributes,
            ): (
                ViewMut<SpellCast>,
                ViewMut<QuestActor>,
                ViewMut<Behavior>,
                ViewMut<ThreatList>,
                ViewMut<AppliedAuras>,
                ViewMut<Attributes>,
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
                        (&mut vm_threat_list, &mut vm_app_auras, &mut vm_attributes),
                    ),
                    (
                        Guid::new(*creature_guid, creature.internal_values.clone()),
                        Powers::new(creature.internal_values.clone()),
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
                        (
                            ThreatList::new(),
                            AppliedAuras::new(
                                CREATURE_BUFF_LIMIT,
                                creature.internal_values.clone(),
                            ),
                            attributes,
                        ),
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
                let world_guard = self.world();
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
        let entity_id = self.world().run(
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

    pub fn queue_packet(&self, world_session: Arc<WorldSession>, packet: ClientMessage) {
        self.packet_queue.queue_packet(world_session, packet);
    }

    pub fn get_area_id(&self, position_x: f32, position_y: f32) -> Option<u32> {
        self.terrain_manager.get_area_id(position_x, position_y)
    }
}

impl Drop for Map {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
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

pub struct WorldRef<'a> {
    guard: ReentrantMutexGuard<'a, RefCell<World>>,
    _world: PhantomData<&'a World>,
}

impl<'a> std::ops::Deref for WorldRef<'a> {
    type Target = World;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.guard.as_ptr() }
    }
}

impl<'a> std::ops::DerefMut for WorldRef<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.guard.as_ptr() }
    }
}
