use std::{collections::HashMap, fmt::Display, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use log::{info, warn};
use parking_lot::RwLock;
use shared::models::terrain_info::{TerrainBlock, MAP_WIDTH_IN_BLOCKS};
use tokio::runtime::Runtime;

use crate::{
    config::WorldConfig,
    entities::{
        creature::Creature,
        object_guid::ObjectGuid,
        player::Player,
        position::{Position, WorldPosition},
        update::{CreateData, WorldEntity},
    },
    protocol::{self, opcodes::Opcode, packets::MovementInfo, server::ServerMessage},
    repositories::creature::CreatureRepository,
    session::world_session::WorldSession,
    DataStore,
};

use super::{map::Map, world_context::WorldContext};

pub enum MapManagerError {
    UnknownMapId,
    CommonMapAlreadyInstanced,
}

#[derive(Eq, Hash, PartialEq)]
pub struct TerrainBlockCoords {
    pub row: usize,
    pub col: usize,
}

pub struct MapManager {
    pub runtime: Runtime,
    config: Arc<WorldConfig>,
    maps: RwLock<HashMap<MapKey, Arc<Map>>>,
    data_store: Arc<DataStore>,
    next_instance_id: RelaxedCounter,
    terrains: RwLock<HashMap<u32, Arc<HashMap<TerrainBlockCoords, TerrainBlock>>>>,
}

impl MapManager {
    pub fn new(data_store: Arc<DataStore>, config: Arc<WorldConfig>) -> Self {
        let world_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all() // TODO: Allow to conf the # of worker threads
            .build()
            .unwrap();

        Self {
            runtime: world_runtime,
            config,
            maps: RwLock::new(HashMap::new()),
            data_store,
            next_instance_id: RelaxedCounter::new(1),
            terrains: RwLock::new(HashMap::new()),
        }
    }

    // Instantiate all common (= continents) maps on startup
    pub fn instantiate_continents(
        &self,
        data_store: Arc<DataStore>,
        world_context: Arc<WorldContext>,
        config: Arc<WorldConfig>,
        conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    ) {
        info!("Instantiating continents...");
        for map in data_store
            .get_all_map_records()
            .filter(|m| !m.is_instanceable())
        {
            let mut map_terrains: HashMap<TerrainBlockCoords, TerrainBlock> = HashMap::new();

            if config.world.dev.load_terrain {
                // Load terrain for this map
                for row in 0..MAP_WIDTH_IN_BLOCKS {
                    for col in 0..MAP_WIDTH_IN_BLOCKS {
                        let maybe_terrain = TerrainBlock::load_from_disk(
                            &config.common.data.directory,
                            &map.internal_name,
                            row,
                            col,
                        );

                        if let Some(terrain_block) = maybe_terrain {
                            let key = TerrainBlockCoords { row, col };
                            map_terrains.insert(key, terrain_block);
                        }
                    }
                }
            } else {
                info!("Terrain loading disabled in configuration");
            }

            let map_terrains = Arc::new(map_terrains);
            self.terrains.write().insert(map.id, map_terrains.clone());

            let spawns = CreatureRepository::load_creature_spawns(conn, map.id);

            let key = MapKey::for_continent(map.id);
            self.runtime.block_on(async {
                self.maps.write().insert(
                    key,
                    Map::new(
                        key,
                        world_context.clone(),
                        map_terrains,
                        spawns,
                        data_store.clone(),
                    ),
                );
            });
        }
    }

    pub fn instantiate_map(
        &self,
        map_id: u32,
        world_context: Arc<WorldContext>,
        conn: &r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>,
    ) -> Result<Arc<Map>, MapManagerError> {
        match self.data_store.get_map_record(map_id) {
            None => Err(MapManagerError::UnknownMapId),
            Some(map_record) => {
                if !map_record.is_instanceable() {
                    return Err(MapManagerError::CommonMapAlreadyInstanced);
                } else {
                    // Load terrain for this map, or get it from the cache
                    let mut terrain_guard = self.terrains.write();
                    let map_terrain: &mut Arc<HashMap<TerrainBlockCoords, TerrainBlock>> =
                        terrain_guard.entry(map_id).or_insert_with(|| {
                            let mut map_terrains: HashMap<TerrainBlockCoords, TerrainBlock> =
                                HashMap::new();

                            for row in 0..MAP_WIDTH_IN_BLOCKS {
                                for col in 0..MAP_WIDTH_IN_BLOCKS {
                                    let maybe_terrain = TerrainBlock::load_from_disk(
                                        &self.config.common.data.directory,
                                        &map_record.internal_name,
                                        row,
                                        col,
                                    );

                                    if let Some(terrain_block) = maybe_terrain {
                                        let key = TerrainBlockCoords { row, col };
                                        map_terrains.insert(key, terrain_block);
                                    }
                                }
                            }

                            Arc::new(map_terrains)
                        });

                    let mut map_guard = self.maps.write();
                    let instance_id: u32 = self.next_instance_id.inc().try_into().unwrap();

                    let spawns = CreatureRepository::load_creature_spawns(conn, map_id);

                    let map_key = MapKey::for_instance(map_id, instance_id);
                    let map = Map::new(
                        map_key,
                        world_context.clone(),
                        map_terrain.clone(),
                        spawns,
                        self.data_store.clone(),
                    );
                    map_guard.insert(map_key, map.clone());

                    Ok(map)
                }
            }
        }
    }

    pub fn get_map(&self, map_key: MapKey) -> Option<Arc<Map>> {
        let guard = self.maps.read();
        guard.get(&map_key).cloned()
    }

    pub fn add_session_to_map(
        &self,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        player: Arc<RwLock<Player>>,
    ) {
        let from_map: Option<MapKey>;
        let player_position: WorldPosition;
        let player_guid: ObjectGuid;
        {
            let player_guard = player.read();
            from_map = player_guard.current_map();
            player_position = player_guard.position().clone();
            player_guid = player_guard.guid().clone();
        }

        if let Some(from_map_key) = from_map {
            let origin_map = self.maps.read().get(&from_map_key).cloned();
            if let Some(origin_map) = origin_map {
                origin_map.remove_player(&player_guid);
            }
        }

        // TODO: handle instance id here
        let destination = MapKey::for_continent(player_position.map);
        let destination_map = self.maps.read().get(&destination).cloned();
        if let Some(destination_map) = destination_map {
            destination_map.add_player(session.clone(), world_context.clone(), player.clone());
            session.set_map(destination);
        } else {
            warn!("map {} not found as destination in MapManager", destination);
        }
    }

    pub fn remove_player_from_map(&self, player_guid: &ObjectGuid, from_map: Option<MapKey>) {
        if let Some(from_map_key) = from_map {
            let origin_map = self.maps.read().get(&from_map_key).cloned();
            if let Some(origin_map) = origin_map {
                origin_map.remove_player(player_guid);
            }
        }
    }

    pub fn add_creature_to_map(
        &self,
        map_key: MapKey,
        world_context: Arc<WorldContext>,
        creature: Arc<RwLock<Creature>>,
    ) {
        let guard = self.maps.read();
        if let Some(map) = guard.get(&map_key) {
            map.add_creature(Some(world_context.clone()), creature.clone());
        }
    }

    pub fn lookup_entity(
        &self,
        guid: &ObjectGuid,
        map_key: Option<MapKey>,
    ) -> Option<Arc<RwLock<dyn WorldEntity + Sync + Send>>> {
        if let Some(map_key) = map_key {
            let maps = self.maps.read();
            let map = maps.get(&map_key);
            if let Some(map) = map {
                map.lookup_entity(guid)
            } else {
                None
            }
        } else if guid.is_player() {
            let map_guard = self.maps.read();
            for (_, map) in &*map_guard {
                if let Some(entity) = map.lookup_entity(guid) {
                    return Some(entity);
                }
            }

            None
        } else {
            warn!("lookup on multiple maps is only allowed for players");
            None
        }
    }

    pub fn broadcast_movement(
        &self,
        mover: Arc<RwLock<Player>>,
        opcode: Opcode,
        movement_info: &MovementInfo,
    ) {
        let player_guid: ObjectGuid;
        let current_map_key: Option<MapKey>;

        {
            let player_guard = mover.read();
            player_guid = player_guard.guid().clone();
            current_map_key = player_guard.current_map();
        }

        if let Some(current_map_key) = current_map_key {
            let map: Option<Arc<Map>> = self.maps.read().get(&current_map_key).cloned();
            if let Some(map) = map {
                for session in
                    map.sessions_nearby_entity(&player_guid, map.visibility_distance(), true, false)
                {
                    session
                        .send_movement(opcode, &player_guid, movement_info)
                        .unwrap();
                }
            }
        }
    }

    pub fn update_player_position(
        &self,
        world_context: Arc<WorldContext>,
        session: Arc<WorldSession>,
        position: &Position,
    ) {
        if let Some(current_map_key) = session.get_current_map() {
            let map: Option<Arc<Map>>;
            {
                map = self.maps.read().get(&current_map_key).cloned();
            }
            if let Some(map) = map {
                let update_data: Vec<CreateData>;
                let player_guid: ObjectGuid;
                {
                    let player_guard = session.player.read();
                    update_data = player_guard.get_create_data(0, world_context.clone());
                    player_guid = player_guard.guid().clone();
                }

                map.update_player_position(
                    &player_guid,
                    session.clone(),
                    position,
                    update_data,
                    world_context.clone(),
                );
            }
        }
    }

    pub fn broadcast_packet<
        const OPCODE: u16,
        Payload: protocol::server::ServerMessagePayload<OPCODE>,
    >(
        &self,
        origin_guid: &ObjectGuid,
        map_key: Option<MapKey>,
        packet: &ServerMessage<OPCODE, Payload>,
        range: Option<f32>,
        include_self: bool,
    ) {
        if let Some(current_map_key) = map_key {
            let map: Option<Arc<Map>>;
            {
                map = self.maps.read().get(&current_map_key).cloned();
            }
            if let Some(map) = map {
                for session in map.sessions_nearby_entity(
                    origin_guid,
                    range.unwrap_or(map.visibility_distance()),
                    true,
                    include_self,
                ) {
                    session.send(packet).unwrap();
                }
            }
        }
    }

    pub fn nearby_sessions(
        &self,
        map: Option<MapKey>,
        player_guid: &ObjectGuid,
        include_self: bool,
    ) -> Vec<Arc<WorldSession>> {
        let mut result = Vec::new();

        if let Some(current_map_key) = map {
            let maps_guard = self.maps.read();
            if let Some(map) = maps_guard.get(&current_map_key) {
                let sessions = &mut map.sessions_nearby_entity(
                    player_guid,
                    map.visibility_distance(),
                    true,
                    include_self,
                );
                result.append(sessions);
            }
        }

        result
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct MapKey {
    map_id: u32,
    instance_id: Option<u32>,
}

impl MapKey {
    pub fn for_continent(map_id: u32) -> Self {
        Self {
            map_id,
            instance_id: None,
        }
    }

    pub fn for_instance(map_id: u32, instance_id: u32) -> Self {
        Self {
            map_id,
            instance_id: Some(instance_id),
        }
    }

    pub fn new(map_id: u32, instance_id: Option<u32>) -> Self {
        match instance_id {
            Some(instance_id) => Self::for_instance(map_id, instance_id),
            None => Self::for_continent(map_id),
        }
    }
}

impl Display for MapKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{}",
            self.map_id,
            if let Some(instance_id) = self.instance_id {
                format!(" ({})", instance_id)
            } else {
                "".to_string()
            }
        )
    }
}
