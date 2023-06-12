use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use enumflags2::make_bitflags;
use log::{error, warn};
use parking_lot::{Mutex, RwLock};
use shared::models::terrain_info::{TerrainBlock, BLOCK_WIDTH, MAP_WIDTH_IN_BLOCKS};
use shipyard::{
    AllStoragesViewMut, EntitiesViewMut, EntityId, IntoWorkload, UniqueViewMut, ViewMut, World,
};

use crate::{
    ecs::{
        components::{guid::Guid, health::Health, melee::Melee, unit::Unit},
        resources::DeltaTime,
        systems::{melee, updates},
    },
    entities::{
        internal_values::{InternalValues, WrappedInternalValues},
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
        update::{
            CreateData, MovementUpdateData, UpdateBlockBuilder, UpdateFlag, UpdateType, WorldEntity,
        },
        update_fields::UNIT_END,
    },
    protocol::{
        self,
        opcodes::Opcode,
        packets::{MovementInfo, SmsgCreateObject},
        server::ServerMessage,
    },
    repositories::creature::CreatureSpawnDbRecord,
    session::world_session::WorldSession,
    shared::constants::{HighGuidType, ObjectTypeId, PLAYER_DEFAULT_COMBAT_REACH},
    DataStore,
};

use super::{
    map_manager::{MapKey, TerrainBlockCoords, WrappedMapManager},
    quad_tree::QuadTree,
    world_context::WorldContext,
};

pub const DEFAULT_VISIBILITY_DISTANCE: f32 = 90.0;

pub struct Map {
    key: MapKey,
    world: Arc<Mutex<World>>, // Maybe a Mutex is needed
    _world_context: Arc<WorldContext>,
    sessions: RwLock<HashMap<ObjectGuid, Arc<WorldSession>>>,
    entities: RwLock<HashMap<ObjectGuid, Arc<RwLock<dyn WorldEntity + Sync + Send>>>>,
    ecs_entities: RwLock<HashMap<ObjectGuid, EntityId>>,
    terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
    entities_tree: RwLock<QuadTree>,
    visibility_distance: f32,
}

impl Map {
    pub fn new(
        key: MapKey,
        world_context: Arc<WorldContext>,
        terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
        spawns: Vec<CreatureSpawnDbRecord>,
        data_store: Arc<DataStore>,
    ) -> Arc<Map> {
        let world = World::new();
        world.add_unique(DeltaTime::default());
        world.add_unique(WrappedMapManager(world_context.map_manager.clone()));

        let workload =
            || (melee::attempt_melee_attack, updates::send_entity_update).into_workload();
        world.add_workload(workload);

        let world = Arc::new(Mutex::new(world));

        let map = Map {
            key,
            world: world.clone(),
            _world_context: world_context,
            sessions: RwLock::new(HashMap::new()),
            entities: RwLock::new(HashMap::new()),
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

            map.add_creature(
                None,
                &guid,
                InternalValues::build_for_creature(&spawn, data_store.clone(), &guid)
                    .expect("unable to build InternalValues for creature from DB spawn"),
                &position,
            );
        }

        let map = Arc::new(map);

        let map_clone = map.clone();
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

                map_clone.tick(elapsed_since_last_tick);

                let tick_duration = Instant::now().duration_since(tick_start_time);
                // TODO: 50 in config
                thread::sleep(Duration::from_millis(50).saturating_sub(tick_duration));
            }
        });

        map
    }

    pub fn ecs_world(&self) -> Arc<Mutex<World>> {
        self.world.clone()
    }

    pub fn lookup_entity_ecs(&self, guid: &ObjectGuid) -> Option<EntityId> {
        self.ecs_entities.read().get(guid).copied()
    }

    pub fn add_player(
        &self,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        player: Arc<RwLock<Player>>,
    ) {
        let player_guid: ObjectGuid;
        let player_position: Position;
        {
            let player_guard = player.read();
            player_guid = player_guard.guid().clone();
            player_position = player_guard.position().to_position();
        }

        self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_health: ViewMut<Health>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>| {
                let internal_values = Arc::new(RwLock::new(InternalValues::build_for_player(
                    &player_guid,
                    world_context.clone(),
                )));
                let entity_id = entities.add_entity(
                    (
                        &mut vm_guid,
                        &mut vm_health,
                        &mut vm_melee,
                        &mut vm_unit,
                        &mut vm_wpos,
                        &mut vm_int_vals,
                    ),
                    (
                        Guid::new(player_guid.clone(), internal_values.clone()),
                        Health::new(100, 100, internal_values.clone()),
                        Melee::new(5, PLAYER_DEFAULT_COMBAT_REACH),
                        Unit::new(internal_values.clone()),
                        WorldPosition {
                            map_key: self.key,
                            zone: 0, /* TODO */
                            x: player_position.x,
                            y: player_position.y,
                            z: player_position.z,
                            o: player_position.o,
                        },
                        WrappedInternalValues(internal_values.clone()),
                    ),
                );

                self.ecs_entities
                    .write()
                    .insert(player_guid.clone(), entity_id);
            },
        );

        session.send_initial_spells();
        session.send_initial_action_buttons();
        session.send_initial_reputations();

        {
            let mut guard = self.sessions.write();
            if let Some(previous_session) = guard.insert(player_guid.clone(), session.clone()) {
                warn!(
                    "session from account {} was already on map {}",
                    previous_session.account_id, self.key
                );
            }
        }

        {
            let mut tree = self.entities_tree.write();
            tree.insert(player_position, player_guid);
        }

        self.entities
            .write()
            .insert(player_guid.clone(), player.clone());

        {
            // TODO: Maybe we can group all updates within the same packet?
            let guids_around: Vec<ObjectGuid> = self.entities_tree.read().search_around_position(
                &player_position,
                self.visibility_distance(),
                true,
                None,
            );
            for guid in guids_around {
                if let Some(entity) = self.lookup_entity(&guid) {
                    // Broadcast the new player to nearby players and to itself
                    let other_session = self.sessions.read().get(&guid).cloned();
                    if let Some(other_session) = other_session {
                        let update_data = player
                            .read()
                            .get_create_data(guid.raw(), world_context.clone());
                        let smsg_update_object = SmsgCreateObject {
                            updates_count: update_data.len() as u32,
                            has_transport: false,
                            updates: update_data,
                        };

                        other_session.create_entity(&player_guid, smsg_update_object);
                    }

                    // Send nearby entities to the new player
                    if guid != player_guid {
                        // Don't send the player to itself twice
                        let update_data = entity
                            .read()
                            .get_create_data(player_guid.raw(), world_context.clone());
                        let smsg_update_object = SmsgCreateObject {
                            updates_count: update_data.len() as u32,
                            has_transport: false,
                            updates: update_data,
                        };

                        session.create_entity(&guid, smsg_update_object);
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

        self.entities.write().remove(player_guid);

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
        world_context: Option<Arc<WorldContext>>, // None during startup
        creature_guid: &ObjectGuid,
        values: InternalValues,
        wpos: &WorldPosition,
    ) {
        let internal_values = Arc::new(RwLock::new(values));
        self.world.lock().run(
            |mut entities: EntitiesViewMut,
             mut vm_guid: ViewMut<Guid>,
             mut vm_health: ViewMut<Health>,
             mut vm_melee: ViewMut<Melee>,
             mut vm_unit: ViewMut<Unit>,
             mut vm_wpos: ViewMut<WorldPosition>,
             mut vm_int_vals: ViewMut<WrappedInternalValues>| {
                let entity_id = entities.add_entity(
                    (
                        &mut vm_guid,
                        &mut vm_health,
                        &mut vm_melee,
                        &mut vm_unit,
                        &mut vm_wpos,
                        &mut vm_int_vals,
                    ),
                    (
                        Guid::new(creature_guid.clone(), internal_values.clone()),
                        Health::new(80, 80, internal_values.clone()),
                        Melee::new(5, PLAYER_DEFAULT_COMBAT_REACH), // FIXME: wrong, it's based on
                        // the 3D model for creatures
                        Unit::new(internal_values.clone()),
                        *wpos,
                        WrappedInternalValues(internal_values.clone()),
                    ),
                );

                self.ecs_entities
                    .write()
                    .insert(creature_guid.clone(), entity_id);
            },
        );

        {
            let mut tree = self.entities_tree.write();
            tree.insert(wpos.to_position(), *creature_guid);
        }

        if let Some(world_context) = world_context {
            for session in self.sessions_nearby_position(
                &wpos.to_position(),
                self.visibility_distance(),
                true,
                None,
            ) {
                // Broadcast the new creature to nearby players
                let movement = Some(MovementUpdateData {
                    movement_flags: 0,  // 0x02000000, // TEMP: Flying
                    movement_flags2: 0, // Always 0 in 2.4.3
                    timestamp: world_context.game_time().as_millis() as u32, // Will overflow every 49.7 days
                    position: wpos.to_position(),
                    // pitch: Some(0.0),
                    pitch: None,
                    fall_time: 0,
                    speed_walk: 2.5,
                    speed_run: 7.0,
                    speed_run_backward: 4.5,
                    speed_swim: 4.722222,
                    speed_swim_backward: 2.5,
                    speed_flight: 70.0,
                    speed_flight_backward: 4.5,
                    speed_turn: 3.141594,
                });

                let flags = make_bitflags!(UpdateFlag::{HighGuid | Living | HasPosition});
                let mut update_builder = UpdateBlockBuilder::new();

                for index in 0..UNIT_END {
                    let value = internal_values.read().get_u32(index as usize);
                    if value != 0 {
                        update_builder.add(index as usize, value);
                    }
                }

                let blocks = update_builder.build();

                let update_data = vec![CreateData {
                    update_type: UpdateType::CreateObject2,
                    packed_guid: creature_guid.as_packed(),
                    object_type: ObjectTypeId::Unit,
                    flags,
                    movement,
                    low_guid_part: None,
                    high_guid_part: Some(HighGuidType::Unit as u32),
                    blocks,
                }];

                let smsg_update_object = SmsgCreateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                };

                session.create_entity(creature_guid, smsg_update_object);
            }
        }
    }

    pub fn update_player_position(
        &self,
        player_guid: &ObjectGuid,
        origin_session: Arc<WorldSession>,
        new_position: &Position,
        create_data: Vec<CreateData>,
        world_context: Arc<WorldContext>,
    ) {
        let previous_position: Option<Position>;
        {
            let mut tree = self.entities_tree.write();
            previous_position = tree.update(new_position, player_guid);
        }

        if let Some(previous_position) = previous_position {
            if previous_position.x == new_position.x
                && previous_position.y == new_position.y
                && previous_position.z == new_position.z
            {
                return; // FIXME: this might cause orientation-only changes to not be reflected
            }

            if let Some(player_ecs_entity) = self.lookup_entity_ecs(player_guid) {
                self.world
                    .lock()
                    .run(|mut vm_wpos: ViewMut<WorldPosition>| {
                        vm_wpos[player_ecs_entity].update_local(new_position);
                    });
            }

            let visibility_distance = self.visibility_distance();
            let in_range_before = self.entities_tree.read().search_around_position(
                &previous_position,
                visibility_distance,
                true,
                Some(player_guid),
            );
            let in_range_before: HashSet<ObjectGuid> = in_range_before.iter().cloned().collect();
            let in_range_now = self.entities_tree.read().search_around_position(
                new_position,
                self.visibility_distance(),
                true,
                Some(player_guid),
            );
            let in_range_now: HashSet<ObjectGuid> = in_range_now.iter().cloned().collect();

            let appeared_for = &in_range_now - &in_range_before;
            let disappeared_for = &in_range_before - &in_range_now;

            let smsg_create_object = SmsgCreateObject {
                updates_count: create_data.len() as u32,
                has_transport: false,
                updates: create_data,
            };

            for other_guid in appeared_for {
                if let Some(entity) = self.lookup_entity(&other_guid) {
                    let other_session = self.sessions.read().get(&other_guid).cloned();
                    if let Some(other_session) = other_session {
                        // Make the moving player appear for the other player
                        other_session.create_entity(player_guid, smsg_create_object.clone());
                    }

                    // Make the entity (player or otherwise) appear for the moving player
                    let create_data = entity
                        .read()
                        .get_create_data(player_guid.raw(), world_context.clone());
                    let smsg_create_object = SmsgCreateObject {
                        updates_count: create_data.len() as u32,
                        has_transport: false,
                        updates: create_data,
                    };
                    origin_session.create_entity(&other_guid, smsg_create_object);
                }
            }

            for other_guid in disappeared_for {
                let other_session = self.sessions.read().get(&other_guid).cloned();
                if let Some(other_session) = other_session {
                    // Destroy the moving player for the other player
                    other_session.destroy_entity(player_guid);
                }

                // Destroy the other entity for the moving player
                origin_session.destroy_entity(&other_guid);
            }
        } else {
            error!("updating position for player not on map");
        }
    }

    pub fn lookup_entity(
        &self,
        guid: &ObjectGuid,
    ) -> Option<Arc<RwLock<dyn WorldEntity + Sync + Send>>> {
        self.entities.read().get(guid).cloned()
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

    pub fn get_terrain_height(&self, position_x: f32, position_y: f32) -> Option<f32> {
        let offset: f32 = MAP_WIDTH_IN_BLOCKS as f32 / 2.0;
        let block_row = (offset - (position_x / BLOCK_WIDTH)).floor() as usize;
        let block_col = (offset - (position_y / BLOCK_WIDTH)).floor() as usize;
        let terrain_block_coords = TerrainBlockCoords {
            row: block_row,
            col: block_col,
        };
        if let Some(terrain_block) = self.terrain.get(&terrain_block_coords) {
            return Some(terrain_block.get_height(position_x, position_y));
            // TODO: terrain_block.map(_.get_height) instead
        }

        let position_z = 0.0;
        Some(position_z)
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

    pub fn tick(&self, _diff: Duration) {
        // let entities = self.entities.read();
        // for (_, entity) in &*entities {
        //     let mut entity = entity.write();
        //     entity.tick(diff, self.world_context.clone());
        //
        //     // Broadcast the changes to nearby players
        //     if entity.has_updates() {
        //         for session in self.sessions_nearby_entity(
        //             entity.guid(),
        //             self.visibility_distance(),
        //             true,
        //             false,
        //         ) {
        //             let update_data = entity.get_update_data(
        //                 session.player.read().guid().raw(),
        //                 self.world_context.clone(),
        //             );
        //
        //             let smsg_update_object = SmsgUpdateObject {
        //                 updates_count: update_data.len() as u32,
        //                 has_transport: false,
        //                 updates: update_data,
        //             };
        //
        //             session.update_entity(smsg_update_object);
        //         }
        //
        //         entity.mark_up_to_date();
        //     }
        // }
    }

    pub fn get_session(&self, player_guid: &ObjectGuid) -> Option<Arc<WorldSession>> {
        self.sessions.read().get(player_guid).cloned()
    }
}
