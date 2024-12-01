use std::{collections::HashMap, fmt::Display, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use log::info;
use parking_lot::RwLock;
use shared::models::terrain_info::{Terrain, TerrainBlock, MAP_WIDTH_IN_BLOCKS};
use shipyard::EntityId;

use crate::{
    config::WorldConfig,
    entities::{object_guid::ObjectGuid, position::WorldPosition},
    protocol::{self, server::ServerMessage},
    repositories::{creature::CreatureRepository, game_object::GameObjectRepository},
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
    config: Arc<WorldConfig>,
    maps: RwLock<HashMap<MapKey, Arc<Map>>>,
    data_store: Arc<DataStore>,
    next_instance_id: RelaxedCounter,
    terrains: RwLock<HashMap<u32, Arc<HashMap<TerrainBlockCoords, Terrain>>>>,
}

impl MapManager {
    pub fn new(data_store: Arc<DataStore>, config: Arc<WorldConfig>) -> Self {
        Self {
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
            .filter(|m| m.is_continent())
        {
            let mut map_terrains: HashMap<TerrainBlockCoords, Terrain> = HashMap::new();

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

            let creature_spawns = CreatureRepository::load_creature_spawns(conn, map.id);
            let game_object_spawns = GameObjectRepository::load_game_object_spawns(conn, map.id);

            let key = MapKey::for_continent(map.id);
            self.maps.write().insert(
                key,
                Map::new(
                    key,
                    world_context.clone(),
                    map_terrains,
                    creature_spawns,
                    game_object_spawns,
                    config.clone(),
                ),
            );
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
                    Err(MapManagerError::CommonMapAlreadyInstanced)
                } else {
                    // Load terrain for this map, or get it from the cache
                    let mut terrain_guard = self.terrains.write();
                    let map_terrain: &mut Arc<HashMap<TerrainBlockCoords, Terrain>> =
                        terrain_guard.entry(map_id).or_insert_with(|| {
                            let mut map_terrains: HashMap<TerrainBlockCoords, Terrain> =
                                HashMap::new();

                            for row in 0..MAP_WIDTH_IN_BLOCKS {
                                for col in 0..MAP_WIDTH_IN_BLOCKS {
                                    let maybe_terrain = TerrainBlock::load_from_disk(
                                        &self.config.common.data.directory,
                                        &map_record.internal_name,
                                        row,
                                        col,
                                    );

                                    if let Some(terrain) = maybe_terrain {
                                        let key = TerrainBlockCoords { row, col };
                                        map_terrains.insert(key, terrain);
                                    }
                                }
                            }

                            Arc::new(map_terrains)
                        });

                    let mut map_guard = self.maps.write();
                    let instance_id: u32 = self.next_instance_id.inc().try_into().unwrap();

                    let creature_spawns = CreatureRepository::load_creature_spawns(conn, map_id);
                    let game_object_spawns =
                        GameObjectRepository::load_game_object_spawns(conn, map_id);

                    let map_key = MapKey::for_instance(map_id, instance_id);
                    let map = Map::new(
                        map_key,
                        world_context.clone(),
                        map_terrain.clone(),
                        creature_spawns,
                        game_object_spawns,
                        self.config.clone(),
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

    pub fn remove_player_from_map(&self, player_guid: &ObjectGuid, from_map: Option<MapKey>) {
        if let Some(from_map_key) = from_map {
            let origin_map = self.maps.read().get(&from_map_key).cloned();
            if let Some(origin_map) = origin_map {
                origin_map.remove_player(player_guid);
            }
        }
    }

    pub fn transfer_player(&self, player_entity_id: EntityId, from_map_key: MapKey, to_map_key: MapKey) {
        // let origin_map = self.maps.read().get_mut(&from_map_key);
        // let destination_map = self.maps.read().get(&to_map_key);
        //
        // match (origin_map, destination_map) {
        //     (Some(origin_map), Some(destination_map)) => {
        //         origin_map.world().move_entity(other, entity)
        //     },
        //     _ => panic!("missing map when transfering (TODO)"),
        // }
        unsafe {
            let mut guard = self.maps.write();
            let origin_map = guard.get_mut(&from_map_key).unwrap() as *mut Arc<Map>;
            let destination_map = guard.get_mut(&to_map_key).unwrap() as *mut Arc<Map>;

            (&mut (*origin_map).world_mut()).lock().move_entity(&mut (*destination_map).world_mut().lock(), player_entity_id);
            
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
        include_self: bool,
    ) {
        if let Some(current_map_key) = map_key {
            let map: Option<Arc<Map>> = self.maps.read().get(&current_map_key).cloned();

            if let Some(map) = map {
                map.broadcast_packet(origin_guid, packet, None, include_self);
            }
        }
    }

    pub fn nearby_sessions(&self, world_position: &WorldPosition) -> Vec<Arc<WorldSession>> {
        let mut result = Vec::new();

        let maps_guard = self.maps.read();
        if let Some(map) = maps_guard.get(&world_position.map_key) {
            let sessions = &mut map.sessions_nearby_position(
                &world_position.as_position(),
                map.visibility_distance(),
                true,
                None,
            );
            result.append(sessions);
        }

        result
    }
}

#[derive(Eq, Hash, PartialEq, Clone, Copy, Debug)]
pub struct MapKey {
    pub map_id: u32,
    pub instance_id: Option<u32>,
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
