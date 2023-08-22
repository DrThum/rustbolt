use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use log::{error, warn};
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
    datastore::WrappedDataStore,
    ecs::{
        components::{
            behavior::Behavior,
            guid::Guid,
            health::Health,
            melee::Melee,
            movement::{Movement, MovementKind},
            quest_actor::QuestActor,
            spell_cast::SpellCast,
            threat_list::ThreatList,
            unit::Unit,
        },
        resources::DeltaTime,
        systems::{behavior, combat, melee, movement, spell, updates},
    },
    entities::{
        creature::Creature,
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
    repositories::{character::CharacterRecord, creature::CreatureSpawnDbRecord},
    session::world_session::WorldSession,
    shared::constants::{HighGuidType, NpcFlags},
    DataStore,
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
        spawns: Vec<CreatureSpawnDbRecord>,
        data_store: Arc<DataStore>,
    ) -> Arc<Map> {
        let world = World::new();
        world.add_unique(DeltaTime::default());
        world.add_unique(WrappedDataStore(world_context.data_store.clone()));
        world.add_unique(WrappedSpellEffectHandler(
            world_context.spell_effect_handler.clone(),
        ));
        world.add_unique(WrappedWorldContext(world_context.clone()));

        let workload = || {
            (
                movement::update_movement,
                behavior::tick,
                combat::update_combat_state,
                combat::select_target,
                melee::attempt_melee_attack, // TODO: player only, move creature to behavior tree
                spell::update_spell,
                updates::send_entity_update,
            )
                .into_workload()
        };
        world.add_workload(workload);

        let world = Arc::new(Mutex::new(world));

        let map = Map {
            key,
            world: world.clone(),
            world_context,
            sessions: RwLock::new(HashMap::new()),
            ecs_entities: RwLock::new(HashMap::new()),
            terrain,
            entities_tree: RwLock::new(QuadTree::new(
                super::quad_tree::QUADTREE_DEFAULT_NODE_CAPACITY,
            )),
            visibility_distance: DEFAULT_VISIBILITY_DISTANCE,
        };

        for spawn in spawns {
            let guid = ObjectGuid::with_entry(HighGuidType::Unit, spawn.entry, spawn.guid);

            let position = WorldPosition {
                map_key: key,
                zone: 1, // FIXME: calculate from position and terrain
                x: spawn.position_x,
                y: spawn.position_y,
                z: spawn.position_z,
                o: spawn.orientation,
            };

            let creature = Creature::from_spawn(&spawn, data_store.clone())
                .expect("unable to build InternalValues for creature from DB spawn");

            map.add_creature(&guid, creature, &position);
        }

        let map = Arc::new(map);

        map.world.lock().add_unique(WrappedMap(map.clone()));

        thread::spawn(move || {
            let mut time = Instant::now();

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
                // TODO: 50 in config
                thread::sleep(Duration::from_millis(50).saturating_sub(tick_duration));
            }
        });

        map
    }

    pub fn world(&self) -> MutexGuard<World> {
        self.world.lock()
    }

    pub fn lookup_entity_ecs(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.ecs_entities.read().get(guid).copied()
    }

    pub fn add_player_on_login(&self, session: Arc<WorldSession>, char_data: &CharacterRecord) {
        let player_guid = ObjectGuid::from_raw(char_data.guid).unwrap();

        {
            let mut tree = self.entities_tree.write();
            tree.insert(char_data.position.to_position(), player_guid);
        }

        {
            let mut guard = self.sessions.write();
            if let Some(previous_session) = guard.insert(player_guid.clone(), session.clone()) {
                warn!(
                    "session from account {} was already on map {}",
                    previous_session.account_id, self.key
                );
            }
        }

        self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_health: ViewMut<Health>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>,
             mut vm_player: ViewMut<Player>,
             mut vm_movement: ViewMut<Movement>,
             mut vm_spell: ViewMut<SpellCast>| {
                let player = Player::load_from_db(
                    session.account_id,
                    char_data.guid,
                    self.world_context.clone(),
                );

                session.send_initial_spells(&player);
                session.send_initial_action_buttons(&player);
                session.send_initial_reputations(&player);

                let base_health_mana_record = self
                    .world_context
                    .data_store
                    .get_player_base_health_mana(char_data.class, char_data.level as u32)
                    .expect("unable to retrieve base health/mana for this class/level combination");

                let entity_id = entities.add_entity(
                    (
                        &mut vm_guid,
                        &mut vm_health,
                        &mut vm_melee,
                        &mut vm_unit,
                        &mut vm_wpos,
                        &mut vm_int_vals,
                        &mut vm_player,
                        &mut vm_movement,
                        &mut vm_spell,
                    ),
                    (
                        Guid::new(player_guid.clone(), player.internal_values.clone()),
                        Health::new(
                            char_data.current_health,
                            base_health_mana_record.base_health, // FIXME: calculate max from base + modifiers
                            player.internal_values.clone(),
                        ),
                        Melee::new(
                            player.internal_values.clone(),
                            5, // FIXME: damage should be dynamic
                            false,
                        ),
                        Unit::new(
                            player.internal_values.clone(),
                            self.world_context.data_store.clone(),
                        ),
                        WorldPosition {
                            map_key: self.key,
                            zone: 0, /* TODO */
                            x: char_data.position.x,
                            y: char_data.position.y,
                            z: char_data.position.z,
                            o: char_data.position.o,
                        },
                        WrappedInternalValues(player.internal_values.clone()),
                        player,
                        Movement::new(MovementKind::PlayerControlled),
                        SpellCast::new(),
                    ),
                );

                self.ecs_entities
                    .write()
                    .insert(player_guid.clone(), entity_id);

                session.set_player_entity_id(entity_id);

                let movement = vm_movement.get(entity_id).ok().map(|m| {
                    m.build_update(
                        self.world_context.clone(),
                        &char_data.position.to_position(),
                    )
                });

                let player = vm_player.get(entity_id).unwrap();
                player.internal_values.write().reset_dirty();
                let smsg_create_object = player.build_create_object(movement, true);

                session.create_entity(&player_guid, smsg_create_object);
            },
        );

        {
            let guids_around: Vec<ObjectGuid> = self.entities_tree.read().search_around_position(
                &char_data.position.to_position(),
                self.visibility_distance(),
                true,
                Some(&player_guid),
            );
            for other_guid in guids_around {
                if let Some(other_entity_id) = self.lookup_entity_ecs(&other_guid) {
                    // Broadcast the new player to nearby players
                    let other_session = self.sessions.read().get(&other_guid).cloned();
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
                                    &char_data.position.to_position(),
                                )
                            });

                            smsg_create_object = v_player
                                .get(new_player_entity_id)
                                .unwrap()
                                .build_create_object(movement, false);
                        }

                        other_session.create_entity(&player_guid, smsg_create_object);
                    }

                    // Send nearby entities to the new player
                    let mut smsg_create_object: Option<SmsgCreateObject> = None;
                    {
                        let world_guard = self.world.lock();

                        let (v_movement, v_player, v_creature, v_wpos) = world_guard
                            .borrow::<(
                                View<Movement>,
                                View<Player>,
                                View<Creature>,
                                View<WorldPosition>,
                            )>()
                            .unwrap();

                        let movement = v_movement.get(other_entity_id).ok().map(|m| {
                            m.build_update(
                                self.world_context.clone(),
                                &v_wpos[other_entity_id].to_position(),
                            )
                        });

                        if let Some(player) = v_player.get(other_entity_id).ok() {
                            smsg_create_object = Some(player.build_create_object(movement, false));
                        } else if let Some(creature) = v_creature.get(other_entity_id).ok() {
                            smsg_create_object = Some(creature.build_create_object(movement));
                        }
                    }

                    if let Some(smsg) = smsg_create_object {
                        session.create_entity(&other_guid, smsg);
                    } else {
                        warn!("add_player_on_login: unable to generate a SmsgCreateObject for guid {:?}", other_guid);
                    }
                } else {
                    error!("found an entity in quadtree but not in MapManager");
                }
            }
        }
    }

    pub fn remove_player(&self, player_guid: &ObjectGuid) {
        self.world
            .lock()
            .run(|mut all_storages: AllStoragesViewMut| {
                if let Some(entity_id) = self.ecs_entities.write().remove(player_guid) {
                    all_storages.delete_entity(entity_id);
                } else {
                    error!(
                        "attempt to remove player {} who is not on map",
                        player_guid.counter()
                    );
                }
            });

        {
            let other_sessions =
                self.sessions_nearby_entity(player_guid, self.visibility_distance(), false, false);
            for other_session in other_sessions {
                other_session.destroy_entity(player_guid);
            }

            let mut tree = self.entities_tree.write();
            tree.delete(player_guid);
        }

        {
            let mut guard = self.sessions.write();
            if let None = guard.remove(player_guid) {
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
        self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_health: ViewMut<Health>,
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
                        &mut vm_health,
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
                        Guid::new(creature_guid.clone(), creature.internal_values.clone()),
                        Health::new(80, 80, creature.internal_values.clone()),
                        Melee::new(
                            creature.internal_values.clone(),
                            5, // FIXME: damage should be dynamic
                            true,
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
                        Behavior::new_wild_monster(),
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

                self.ecs_entities
                    .write()
                    .insert(creature_guid.clone(), entity_id);

                vm_creature
                    .get(entity_id)
                    .unwrap()
                    .internal_values
                    .write()
                    .reset_dirty();
            },
        );

        {
            let mut tree = self.entities_tree.write();
            tree.insert(wpos.to_position(), *creature_guid);
        }

        // TODO: Don't attempt this during startup, it's pointless
        for session in self.sessions_nearby_position(
            &wpos.to_position(),
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
                    .map(|m| m.build_update(self.world_context.clone(), &wpos.to_position()));

                smsg_create_object = v_creature
                    .get(new_creature_entity_id)
                    .unwrap()
                    .build_create_object(movement);
            }

            session.create_entity(creature_guid, smsg_create_object);
        }
    }

    pub fn update_entity_position(
        &self,
        entity_guid: &ObjectGuid,
        origin_session: Option<Arc<WorldSession>>, // Must be defined if entity is a player
        new_position: &Position,
        v_movement: &View<Movement>,
        v_player: &View<Player>,
        v_creature: &View<Creature>,
        vm_wpos: &mut ViewMut<WorldPosition>,
    ) {
        let previous_position: Option<Position>;
        {
            let mut tree = self.entities_tree.write();
            previous_position = tree.update(new_position, entity_guid);
        }

        if let Some(previous_position) = previous_position {
            if previous_position.is_same_spot(new_position) {
                return;
            }

            let mut moving_entity_smsg_create_object: Option<SmsgCreateObject> = None;
            if let Some(entity_id) = self.lookup_entity_ecs(entity_guid) {
                vm_wpos[entity_id].update_local(new_position);

                let movement = v_movement
                    .get(entity_id)
                    .ok()
                    .map(|m| m.build_update(self.world_context.clone(), new_position));

                if let Some(player) = v_player.get(entity_id).ok() {
                    moving_entity_smsg_create_object =
                        Some(player.build_create_object(movement, false));
                } else if let Some(creature) = v_creature.get(entity_id).ok() {
                    moving_entity_smsg_create_object = Some(creature.build_create_object(movement));
                }
            }

            let visibility_distance = self.visibility_distance();
            let in_range_before = self.entities_tree.read().search_around_position(
                &previous_position,
                visibility_distance,
                true,
                Some(entity_guid),
            );
            let in_range_before: HashSet<ObjectGuid> = in_range_before.iter().cloned().collect();
            let in_range_now = self.entities_tree.read().search_around_position(
                new_position,
                visibility_distance,
                true,
                Some(entity_guid),
            );
            let in_range_now: HashSet<ObjectGuid> = in_range_now.iter().cloned().collect();

            let appeared_for = &in_range_now - &in_range_before;
            let disappeared_for = &in_range_before - &in_range_now;

            for other_guid in appeared_for {
                if let Some(other_entity_id) = self.lookup_entity_ecs(&other_guid) {
                    let other_session = self.sessions.read().get(&other_guid).cloned();
                    if let Some(other_session) = other_session {
                        // Make the moving player appear for the other player
                        other_session.create_entity(
                            entity_guid,
                            moving_entity_smsg_create_object.as_ref().unwrap().clone(),
                        );
                    }

                    // Make the entity (player or otherwise) appear for the moving player
                    let mut smsg_create_object: Option<SmsgCreateObject> = None;

                    {
                        let movement = v_movement.get(other_entity_id).ok().map(|m| {
                            m.build_update(
                                self.world_context.clone(),
                                &vm_wpos[other_entity_id].to_position(),
                            )
                        });

                        if let Some(player) = v_player.get(other_entity_id).ok() {
                            smsg_create_object = Some(player.build_create_object(movement, false));
                        } else if let Some(creature) = v_creature.get(other_entity_id).ok() {
                            smsg_create_object = Some(creature.build_create_object(movement));
                        }
                    }

                    if let Some(smsg) = smsg_create_object {
                        origin_session
                            .as_ref()
                            .map(|os| os.create_entity(&other_guid, smsg));
                    } else {
                        warn!("add_player_on_login: unable to generate a SmsgCreateObject for guid {:?}", other_guid);
                    }
                }
            }

            for other_guid in disappeared_for {
                let other_session = self.sessions.read().get(&other_guid).cloned();
                if let Some(other_session) = other_session {
                    // Destroy the moving player for the other player
                    other_session.destroy_entity(entity_guid);
                }

                // Destroy the other entity for the moving player
                origin_session
                    .as_ref()
                    .map(|os| os.destroy_entity(&other_guid));
            }
        } else {
            error!("updating position for player not on map");
        }
    }

    pub fn sessions_nearby_entity(
        &self,
        source_guid: &ObjectGuid,
        range: f32,
        search_in_3d: bool,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let guids = self.entities_tree.read().search_around_entity(
            source_guid,
            range,
            search_in_3d,
            if include_self {
                None
            } else {
                Some(source_guid)
            },
        );

        self.sessions
            .read()
            .iter()
            .filter_map(|(&guid, session)| {
                if guids.contains(&guid) {
                    Some(session.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn sessions_nearby_position(
        &self,
        position: &Position,
        range: f32,
        search_in_3d: bool,
        exclude_guid: Option<&ObjectGuid>,
    ) -> Vec<Arc<WorldSession>> {
        let guids = self.entities_tree.read().search_around_position(
            position,
            range,
            search_in_3d,
            exclude_guid,
        );

        self.sessions
            .read()
            .iter()
            .filter_map(|(&guid, session)| {
                if guids.contains(&guid) {
                    Some(session.clone())
                } else {
                    None
                }
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
                .and_then(|mesh| mesh.cast_ray(&Isometry::identity(), &ray, std::f32::MAX, false));

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

    pub fn visibility_distance(&self) -> f32 {
        self.visibility_distance
    }

    pub fn broadcast_movement(
        &self,
        origin_guid: &ObjectGuid,
        opcode: Opcode,
        movement_info: &MovementInfo,
    ) {
        for session in
            self.sessions_nearby_entity(origin_guid, self.visibility_distance(), true, false)
        {
            session
                .send_movement(opcode, origin_guid, movement_info)
                .unwrap();
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
        for session in self.sessions_nearby_entity(
            origin_guid,
            range.unwrap_or(self.visibility_distance()),
            true,
            include_self,
        ) {
            session.send(packet).unwrap();
        }
    }

    pub fn get_session(&self, player_guid: &ObjectGuid) -> Option<Arc<WorldSession>> {
        self.sessions.read().get(player_guid).cloned()
    }

    pub fn get_random_point_around(&self, origin: &Vector3, radius: f32) -> Vector3 {
        if radius <= 0. {
            return origin.clone();
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

        let mut point = origin.clone();
        point.x = x;
        point.y = y;
        point.z = z;

        point
    }
}

#[derive(Unique)]
pub struct WrappedMap(pub Arc<Map>);
