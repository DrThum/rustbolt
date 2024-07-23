use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use log::{error, info, warn};
use parking_lot::{Mutex, MutexGuard, RwLock};
use parry3d::{
    math::{Isometry, Point},
    query::{Ray, RayCast},
};
use rand::Rng;
use shared::models::terrain_info::{Terrain, Vector3, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};
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
            unwind::Unwind,
        },
        resources::DeltaTime,
        systems::{behavior, combat, inventory, melee, movement, powers, spell, unwind, updates},
    },
    entities::{
        creature::Creature,
        game_object::GameObject,
        internal_values::WrappedInternalValues,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
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
    session::world_session::WorldSession,
    shared::constants::{HighGuidType, NpcFlags, WeaponAttackType},
};

use super::{
    map_manager::{MapKey, TerrainBlockCoords},
    quad_tree::QuadTree,
    spell_effect_handler::WrappedSpellEffectHandler,
    world_context::{WorldContext, WrappedWorldContext},
};

pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;
// Add a safety margin when calculating ground height to account for rounding when generating
// terrain data and ensure that our ray (when we are within a WMO) starts from high enough to
// actually hit the floor.
pub const HEIGHT_MEASUREMENT_TOLERANCE: f32 = 1.;

pub struct Map {
    key: MapKey,
    world: Arc<Mutex<World>>,
    world_context: Arc<WorldContext>,
    sessions: RwLock<HashMap<ObjectGuid, Arc<WorldSession>>>,
    ecs_entities: RwLock<HashMap<ObjectGuid, EntityId>>,
    terrain: Arc<HashMap<TerrainBlockCoords, Terrain>>,
    entities_tree: RwLock<QuadTree>,
    visibility_distance: f32,
}

impl Map {
    pub fn new(
        key: MapKey,
        world_context: Arc<WorldContext>,
        terrain: Arc<HashMap<TerrainBlockCoords, Terrain>>,
        creature_spawns: Vec<CreatureSpawnDbRecord>,
        game_object_spawns: Vec<GameObjectSpawnDbRecord>,
        config: Arc<WorldConfig>,
    ) -> Arc<Map> {
        let world = World::new();
        world.add_unique(DeltaTime::default());
        world.add_unique(WrappedSpellEffectHandler(
            world_context.spell_effect_handler.clone(),
        ));
        world.add_unique(WrappedWorldContext(world_context.clone()));

        let workload = || {
            (
                unwind::unwind_creatures,
                updates::update_attributes_from_modifiers, // Must be before regenerate_powers
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

        let world = Arc::new(Mutex::new(world));

        let map = Map {
            key,
            world: world.clone(),
            world_context: world_context.clone(),
            sessions: RwLock::new(HashMap::new()),
            ecs_entities: RwLock::new(HashMap::new()),
            terrain,
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            visibility_distance: DEFAULT_VISIBILITY_DISTANCE,
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

        map.world.lock().add_unique(WrappedMap(map.clone()));

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
                        world_guard.run(|mut dt: UniqueViewMut<DeltaTime>| {
                            *dt = DeltaTime(elapsed_since_last_tick);
                        });
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

    pub fn world(&self) -> MutexGuard<World> {
        self.world.lock()
    }

    pub fn id(&self) -> u32 {
        self.key.map_id
    }

    pub fn lookup_entity_ecs(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.ecs_entities.read().get(guid).copied()
    }

    pub fn has_players(&self) -> bool {
        !self.sessions.read().is_empty()
    }

    pub fn add_player_on_login(&self, session: Arc<WorldSession>, char_data: &CharacterRecord) {
        let player_guid = ObjectGuid::from_raw(char_data.guid).unwrap();

        {
            let mut guard = self.sessions.write();
            if let Some(previous_session) = guard.insert(player_guid, session.clone()) {
                warn!(
                    "session from account {} was already on map {}",
                    previous_session.account_id, self.key
                );
            }
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

                self.ecs_entities.write().insert(player_guid, entity_id);

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

        {
            let mut tree = self.entities_tree.write();
            tree.insert(char_data.position.as_position(), player_entity_id);
        }

        {
            let entities_around = self.entities_tree.read().search_around_position(
                &char_data.position.as_position(),
                self.visibility_distance(),
                true,
                Some(&player_entity_id),
            );
            for (other_entity_id, _) in entities_around {
                let other_entity_guid = self
                    .world()
                    .run(|v_guid: View<Guid>| v_guid[other_entity_id].0);
                // Broadcast the new player to nearby players
                let other_session = self.sessions.read().get(&other_entity_guid).cloned();
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
                        smsg_create_object = Some(game_object.build_create_object());
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
                    if let Some(entity_id) = self.ecs_entities.write().remove(player_guid) {
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
            let other_sessions = self.sessions_nearby_entity(
                &player_entity_id,
                self.visibility_distance(),
                false,
                false,
            );
            for other_session in other_sessions {
                other_session.destroy_entity(player_guid);
            }

            let mut tree = self.entities_tree.write();
            tree.delete(&player_entity_id);
        }

        {
            let mut guard = self.sessions.write();
            if guard.remove(player_guid).is_none() {
                warn!("player guid {:?} was not on map {}", player_guid, self.key);
            }
        }
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

                self.ecs_entities.write().insert(*creature_guid, entity_id);

                vm_creature
                    .get(entity_id)
                    .unwrap()
                    .internal_values
                    .write()
                    .reset_dirty();

                entity_id
            },
        );

        {
            let mut tree = self.entities_tree.write();
            tree.insert(wpos.as_position(), creature_entity_id);
        }

        // TODO: Don't attempt this during startup, it's pointless
        for session in self.sessions_nearby_position(
            &wpos.as_position(),
            self.visibility_distance(),
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

        {
            let mut tree = self.entities_tree.write();
            tree.insert(wpos.as_position(), entity_id);
        }

        self.ecs_entities
            .write()
            .insert(*game_object_guid, entity_id);

        // TODO: Notify nearby players if map.has_players
    }

    pub fn update_entity_position(
        &self,
        entity_guid: &ObjectGuid,
        mover_entity_id: EntityId,
        origin_session: Option<Arc<WorldSession>>, // Must be defined if entity is a player
        new_position: &Position,                   // FIXME: remove the &
        v_movement: &View<Movement>,
        v_player: &View<Player>,
        v_creature: &View<Creature>,
        v_game_object: &View<GameObject>,
        v_guid: &View<Guid>,
        vm_wpos: &mut ViewMut<WorldPosition>,
        vm_behavior: &mut ViewMut<Behavior>,
        vm_nearby_players: &mut ViewMut<NearbyPlayers>,
        vm_unwind: &mut ViewMut<Unwind>,
    ) {
        let is_moving_entity_a_player = origin_session.is_some();
        let previous_position: Option<Position>;
        {
            let mut tree = self.entities_tree.write();
            previous_position = tree.update(new_position, &mover_entity_id);
        }

        if let Some(previous_position) = previous_position {
            if previous_position.is_same_spot(new_position) {
                return;
            }

            vm_wpos[mover_entity_id].update_local(new_position);

            let VisibilityChangesAfterMovement {
                moving_entity_appeared_for: appeared_for,
                moving_entity_disappeared_for: disappeared_for,
                entities_in_range_now: in_range_now,
                ..
            } = self.get_visibility_changes_for_entities(
                previous_position,
                *new_position,
                mover_entity_id,
            );

            let mut moving_entity_smsg_create_object: Option<SmsgCreateObject> = None;

            for other_entity_id in appeared_for {
                let other_guid = v_guid[other_entity_id].0;
                let other_session = self.sessions.read().get(&other_guid).cloned();
                // Make the moving entity appear for the other player
                if let Some(other_session) = other_session {
                    // Construct the SMSG the first time that it's needed
                    if moving_entity_smsg_create_object.is_none() {
                        let movement = v_movement
                            .get(mover_entity_id)
                            .ok()
                            .map(|m| m.build_update(self.world_context.clone(), new_position));

                        if let Ok(player) = v_player.get(mover_entity_id) {
                            moving_entity_smsg_create_object =
                                Some(player.build_create_object(movement, false));
                        } else if let Ok(creature) = v_creature.get(mover_entity_id) {
                            moving_entity_smsg_create_object =
                                Some(creature.build_create_object(movement));
                        }
                    }

                    other_session.create_entity(
                        entity_guid,
                        moving_entity_smsg_create_object.as_ref().unwrap().clone(),
                    );
                }

                // Make the entity (player or otherwise) appear for the moving player
                if let Some(origin_session) = origin_session.as_ref() {
                    let smsg_create_object: SmsgCreateObject = {
                        let movement = v_movement.get(other_entity_id).ok().map(|m| {
                            m.build_update(
                                self.world_context.clone(),
                                &vm_wpos[other_entity_id].as_position(),
                            )
                        });

                        if let Ok(player) = v_player.get(other_entity_id) {
                            player.build_create_object(movement, false)
                        } else if let Ok(creature) = v_creature.get(other_entity_id) {
                            creature.build_create_object(movement)
                        } else if let Ok(game_object) = v_game_object.get(other_entity_id) {
                            game_object.build_create_object()
                        } else {
                            unreachable!("cannot generate SMSG_CREATE_OBJECT for this entity type");
                        }
                    };

                    origin_session.create_entity(&other_guid, smsg_create_object);
                }

                // If a player appeared, increment the NearbyPlayers counter for the creature
                if is_moving_entity_a_player {
                    NearbyPlayers::increment(other_entity_id, vm_nearby_players);
                }
                // If a creature moved within visibility distance of a player, increment the
                // NearbyPlayers counter for the creature
                else if v_player.get(other_entity_id).is_ok() {
                    NearbyPlayers::increment(mover_entity_id, vm_nearby_players);
                }
            }

            for other_entity_id in disappeared_for {
                let other_guid = v_guid[other_entity_id].0;
                let other_session = self.sessions.read().get(&other_guid).cloned();
                if let Some(other_session) = other_session {
                    // Destroy the moving player for the other player
                    other_session.destroy_entity(entity_guid);
                }

                // Destroy the other entity for the moving player
                if let Some(os) = origin_session.as_ref() {
                    os.destroy_entity(&other_guid)
                }

                // If a player moved away, decrement the NearbyPlayers counter for the creature
                if is_moving_entity_a_player {
                    NearbyPlayers::decrement(other_entity_id, vm_nearby_players, vm_unwind);
                }
                // If a creature moved away from a player, decrement the NearbyPlayers counter for
                // the creature
                else if v_player.get(other_entity_id).is_ok() {
                    NearbyPlayers::decrement(mover_entity_id, vm_nearby_players, vm_unwind);
                }
            }

            // If a creature is involved (whether it moved or it witnessed another entity moving),
            // inform its behavior tree that a neighbor has moved (for aggro, script, etc)
            for &neighbor in in_range_now.iter() {
                if let Ok(mut source_behavior) = vm_behavior.get(mover_entity_id) {
                    source_behavior.neighbor_moved(neighbor);
                }

                if let Ok(mut neighbor_behavior) = vm_behavior.get(neighbor) {
                    neighbor_behavior.neighbor_moved(mover_entity_id);
                }
            }
        } else {
            error!("updating position for entity not on map");
        }
    }

    fn get_visibility_changes_for_entities(
        &self,
        previous_position: Position,
        new_position: Position,
        mover_entity_id: EntityId,
    ) -> VisibilityChangesAfterMovement {
        let visibility_distance = self.visibility_distance;
        let in_range_before: Vec<EntityId>;
        let in_range_now: Vec<EntityId>;

        let traveled_distance = previous_position.distance_to(new_position, true);
        if traveled_distance <= visibility_distance {
            // Most of the moves are going to be small (entity just walking around), so in these
            // cases we can perform a single search in the quadtree, in a circle that will include
            // both the old and new positions circles (which are going to mostly overlap)
            let center = previous_position.center_between(new_position);
            let search_radius = visibility_distance + traveled_distance / 2.;
            let in_range_all = self.entities_tree.read().search_around_position(
                &center,
                search_radius,
                true,
                Some(&mover_entity_id),
            );

            let previous_vec3 = previous_position.vec3();
            let new_vec3 = new_position.vec3();
            in_range_before = in_range_all
                .iter()
                .filter_map(|(entity_id, vec3)| {
                    if vec3.square_distance_3d(&previous_vec3)
                        <= visibility_distance * visibility_distance
                    {
                        Some(*entity_id)
                    } else {
                        None
                    }
                })
                .collect();

            in_range_now = in_range_all
                .iter()
                .filter_map(|(entity_id, vec3)| {
                    if vec3.square_distance_3d(&new_vec3)
                        <= visibility_distance * visibility_distance
                    {
                        Some(*entity_id)
                    } else {
                        None
                    }
                })
                .collect();
        } else {
            in_range_before = self
                .entities_tree
                .read()
                .search_around_position(
                    &previous_position,
                    visibility_distance,
                    true,
                    Some(&mover_entity_id),
                )
                .into_iter()
                .map(|(entity_id, _)| entity_id)
                .collect();
            in_range_now = self
                .entities_tree
                .read()
                .search_around_position(
                    &new_position,
                    visibility_distance,
                    true,
                    Some(&mover_entity_id),
                )
                .into_iter()
                .map(|(entity_id, _)| entity_id)
                .collect();
        }

        let in_range_now: HashSet<EntityId> = in_range_now.into_iter().collect();
        let in_range_before: HashSet<EntityId> = in_range_before.into_iter().collect();

        let appeared_for = &in_range_now - &in_range_before;
        let disappeared_for = &in_range_before - &in_range_now;

        VisibilityChangesAfterMovement {
            moving_entity_appeared_for: appeared_for,
            moving_entity_disappeared_for: disappeared_for,
            entities_in_range_before: in_range_before,
            entities_in_range_now: in_range_now,
        }
    }

    pub fn sessions_nearby_entity(
        &self,
        source_entity_id: &EntityId,
        range: f32,
        search_in_3d: bool,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let entity_ids: Vec<EntityId> = self
            .entities_tree
            .read()
            .search_around_entity(
                source_entity_id,
                range,
                search_in_3d,
                if include_self {
                    None
                } else {
                    Some(source_entity_id)
                },
            )
            .into_iter()
            .map(|(entity_id, _)| entity_id)
            .collect();

        self.sessions
            .read()
            .iter()
            .filter_map(|(guid, session)| {
                if let Some(entity_id) = self.lookup_entity_ecs(guid) {
                    if entity_ids.contains(&entity_id) {
                        return Some(session.clone());
                    }
                }

                None
            })
            .collect()
    }

    pub fn sessions_nearby_position(
        &self,
        position: &Position,
        range: f32,
        search_in_3d: bool,
        exclude_id: Option<&EntityId>,
    ) -> Vec<Arc<WorldSession>> {
        let entity_ids: Vec<EntityId> = self
            .entities_tree
            .read()
            .search_around_position(position, range, search_in_3d, exclude_id)
            .into_iter()
            .map(|(entity_id, _)| entity_id)
            .collect();

        self.sessions
            .read()
            .iter()
            .filter_map(|(guid, session)| {
                if let Some(entity_id) = self.lookup_entity_ecs(guid) {
                    if entity_ids.contains(&entity_id) {
                        return Some(session.clone());
                    }
                }

                None
            })
            .collect()
    }

    pub fn get_ground_or_floor_height(
        &self,
        position_x: f32,
        position_y: f32,
        position_z: f32,
    ) -> Option<f32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };

        self.terrain.get(&terrain_block_coords).and_then(|terrain| {
            // Check if we are within a WMO first
            let ray = Ray::new(
                Point::new(
                    position_x,
                    position_y,
                    position_z + HEIGHT_MEASUREMENT_TOLERANCE,
                ),
                -parry3d::na::Vector3::z(),
            );
            let time_of_impact = terrain
                .collision_mesh
                .as_ref()
                .and_then(|mesh| mesh.cast_ray(&Isometry::identity(), &ray, f32::MAX, false));

            let intersection_point = time_of_impact.map(|toi| ray.origin + ray.dir * toi);
            let wmo_height = intersection_point
                .map(|ip| ip[2])
                .filter(|&h| h <= (position_z + HEIGHT_MEASUREMENT_TOLERANCE));

            // Fallback on the ground
            let ground_height = terrain
                .ground
                .get_height(position_x, position_y)
                .filter(|&h| h <= (position_z + HEIGHT_MEASUREMENT_TOLERANCE));

            match (wmo_height, ground_height) {
                (None, None) => None,
                (None, Some(_)) => ground_height,
                (Some(_), None) => wmo_height,
                (Some(wmo), Some(ground)) => Some(wmo.max(ground)),
            }
        })
    }

    pub fn get_area_id(&self, position_x: f32, position_y: f32) -> Option<u32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };

        self.terrain.get(&terrain_block_coords).map(|terrain| {
            // TODO: Area ID might come from the WMO
            terrain.ground.get_area_id(position_x, position_y)
        })
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
            for session in self.sessions_nearby_entity(
                &origin_entity_id,
                self.visibility_distance(),
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
        if let Some(origin_entity_id) = self.lookup_entity_ecs(origin_guid) {
            for session in self.sessions_nearby_entity(
                &origin_entity_id,
                range.unwrap_or(self.visibility_distance()),
                true,
                include_self,
            ) {
                session.send(packet).unwrap();
            }
        }
    }

    pub fn get_session(&self, player_guid: &ObjectGuid) -> Option<Arc<WorldSession>> {
        self.sessions.read().get(player_guid).cloned()
    }

    pub fn get_random_point_around(&self, origin: &Vector3, radius: f32) -> Vector3 {
        if radius <= 0. {
            return *origin;
        }

        let mut rng = rand::thread_rng();
        let angle: f32 = rng.gen_range(0.0..2. * std::f32::consts::PI);
        let distance = rng.gen_range(0.0..=radius);

        let random_x = origin.x + distance * angle.cos();
        let random_y = origin.y + distance * angle.sin();
        let z = self
            .get_ground_or_floor_height(random_x, random_y, origin.z)
            .unwrap_or(origin.z);

        Vector3::new(random_x, random_y, z)
    }

    pub fn get_point_around_at_angle(
        &self,
        origin: &WorldPosition,
        distance: f32,
        angle: f32,
    ) -> WorldPosition {
        let x = origin.x + distance * angle.cos();
        let y = origin.y + distance * angle.sin();

        let z = self
            .get_ground_or_floor_height(x, y, origin.z)
            .unwrap_or(origin.z);

        let mut point = *origin;
        point.x = x;
        point.y = y;
        point.z = z;

        point
    }
}

#[derive(Unique)]
pub struct WrappedMap(pub Arc<Map>);

#[allow(dead_code)]
struct VisibilityChangesAfterMovement {
    pub moving_entity_appeared_for: HashSet<EntityId>,
    pub moving_entity_disappeared_for: HashSet<EntityId>,
    pub entities_in_range_before: HashSet<EntityId>,
    pub entities_in_range_now: HashSet<EntityId>,
}
