use std::{collections::HashMap, fmt::Display, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use log::info;
use parking_lot::RwLock;

use crate::{
    config::WorldConfig,
    entities::object_guid::ObjectGuid,
    repositories::{creature::CreatureRepository, game_object::GameObjectRepository},
    DataStore,
};

use super::{map::Map, terrain_manager::TerrainManager, world_context::WorldContext};

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
    terrains: RwLock<HashMap<u32, Arc<TerrainManager>>>,
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
        let load_terrain = config.world.dev.load_terrain;
        let data_dir = &config.common.data.directory;

        for map in data_store
            .get_all_map_records()
            .filter(|m| m.is_continent())
        {
            let map_terrain_manager = Arc::new(if load_terrain {
                // Load terrain for this map
                TerrainManager::load(data_dir, &map.internal_name)
            } else {
                info!("Terrain loading disabled in configuration");
                TerrainManager::empty()
            });

            self.terrains
                .write()
                .insert(map.id, map_terrain_manager.clone());

            let creature_spawns = CreatureRepository::load_creature_spawns(conn, map.id);
            let game_object_spawns = GameObjectRepository::load_game_object_spawns(conn, map.id);

            let key = MapKey::for_continent(map.id);
            let map = Arc::new(Map::new(
                key,
                world_context.clone(),
                map_terrain_manager,
                creature_spawns,
                game_object_spawns,
            ));
            self.maps.write().insert(key, map.clone());
            let config = config.clone();
            std::thread::Builder::new()
                .name(format!("Map {}", map.id()))
                .spawn(move || {
                    map.start(config);
                })
                .unwrap();
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
                    let map_terrain_manager: &mut Arc<TerrainManager> =
                        terrain_guard.entry(map_id).or_insert_with(|| {
                            Arc::new(TerrainManager::load(
                                &self.config.common.data.directory,
                                &map_record.internal_name,
                            ))
                        });

                    let mut map_guard = self.maps.write();
                    let instance_id: u32 = self.next_instance_id.inc().try_into().unwrap();

                    let creature_spawns = CreatureRepository::load_creature_spawns(conn, map_id);
                    let game_object_spawns =
                        GameObjectRepository::load_game_object_spawns(conn, map_id);

                    let map_key = MapKey::for_instance(map_id, instance_id);
                    let map = Arc::new(Map::new(
                        map_key,
                        world_context.clone(),
                        map_terrain_manager.clone(),
                        creature_spawns,
                        game_object_spawns,
                    ));
                    map_guard.insert(map_key, map.clone());
                    map.start(self.config.clone());

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
