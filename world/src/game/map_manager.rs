use std::{collections::HashMap, fmt::Display, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use log::warn;
use shared::models::terrain_info::{TerrainBlock, MAP_WIDTH_IN_BLOCKS};
use tokio::sync::RwLock;

use crate::{
    protocol::{opcodes::Opcode, packets::MovementInfo},
    session::world_session::WorldSession,
    DataStore,
};

use super::map::Map;

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
    data_dir: String,
    maps: RwLock<HashMap<MapKey, Arc<RwLock<Map>>>>,
    data_store: Arc<DataStore>,
    next_instance_id: RelaxedCounter,
    terrains: RwLock<HashMap<u32, Arc<HashMap<TerrainBlockCoords, TerrainBlock>>>>,
}

impl MapManager {
    pub async fn create_with_continents(data_store: Arc<DataStore>, data_dir: &String) -> Self {
        let mut maps: HashMap<MapKey, Arc<RwLock<Map>>> = HashMap::new();
        let mut terrains: HashMap<u32, Arc<HashMap<TerrainBlockCoords, TerrainBlock>>> =
            HashMap::new();

        // Instantiate all common (= continents) maps on startup
        for map in data_store
            .get_all_map_records()
            .filter(|m| !m.is_instanceable())
        {
            let mut map_terrains: HashMap<TerrainBlockCoords, TerrainBlock> = HashMap::new();

            // Load terrain for this map
            for row in 0..MAP_WIDTH_IN_BLOCKS {
                for col in 0..MAP_WIDTH_IN_BLOCKS {
                    let maybe_terrain =
                        TerrainBlock::load_from_disk(&data_dir, &map.internal_name, row, col);

                    if let Some(terrain_block) = maybe_terrain {
                        let key = TerrainBlockCoords { row, col };
                        map_terrains.insert(key, terrain_block);
                    }
                }
            }

            let map_terrains = Arc::new(map_terrains);
            terrains.insert(map.id, map_terrains.clone());

            let key = MapKey::for_continent(map.id);
            maps.insert(key, Arc::new(RwLock::new(Map::new(key, map_terrains))));
        }

        Self {
            data_dir: data_dir.to_string(),
            maps: RwLock::new(maps),
            data_store,
            next_instance_id: RelaxedCounter::new(1),
            terrains: RwLock::new(terrains),
        }
    }

    pub async fn instantiate_map(&self, map_id: u32) -> Result<Arc<RwLock<Map>>, MapManagerError> {
        match self.data_store.get_map_record(map_id) {
            None => Err(MapManagerError::UnknownMapId),
            Some(map_record) => {
                if !map_record.is_instanceable() {
                    return Err(MapManagerError::CommonMapAlreadyInstanced);
                } else {
                    // Load terrain for this map, or get it from the cache
                    let mut terrain_guard = self.terrains.write().await;
                    let map_terrain: &mut Arc<HashMap<TerrainBlockCoords, TerrainBlock>> =
                        terrain_guard.entry(map_id).or_insert_with(|| {
                            let mut map_terrains: HashMap<TerrainBlockCoords, TerrainBlock> =
                                HashMap::new();

                            for row in 0..MAP_WIDTH_IN_BLOCKS {
                                for col in 0..MAP_WIDTH_IN_BLOCKS {
                                    let maybe_terrain = TerrainBlock::load_from_disk(
                                        &self.data_dir,
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

                    let mut map_guard = self.maps.write().await;
                    let instance_id: u32 = self.next_instance_id.inc().try_into().unwrap();

                    let map_key = MapKey::for_instance(map_id, instance_id);
                    let map = Arc::new(RwLock::new(Map::new(map_key, map_terrain.clone())));
                    map_guard.insert(map_key, map.clone());

                    Ok(map)
                }
            }
        }
    }

    pub async fn get_map(&self, map_key: MapKey) -> Option<Arc<RwLock<Map>>> {
        let guard = self.maps.read().await;
        guard.get(&map_key).cloned()
    }

    pub async fn add_session_to_map(&self, session: Arc<WorldSession>, destination: MapKey) {
        let from_map = session.get_current_map().await;

        let guard = self.maps.write().await;
        if let Some(from_map_key) = from_map {
            if let Some(origin_map) = guard.get(&from_map_key) {
                origin_map
                    .write()
                    .await
                    .remove_player(session.account_id)
                    .await;
            }
        }

        if let Some(destination_map) = guard.get(&destination) {
            destination_map
                .write()
                .await
                .add_player(session.clone())
                .await;
            session.set_map(destination).await;
        } else {
            warn!("map {} not found as destination in MapManager", destination);
        }
    }

    pub async fn remove_session(&self, session: Arc<WorldSession>) {
        let from_map = session.get_current_map().await;

        let guard = self.maps.write().await;
        if let Some(from_map_key) = from_map {
            if let Some(origin_map) = guard.get(&from_map_key) {
                origin_map
                    .write()
                    .await
                    .remove_player(session.account_id)
                    .await;
            }
        }
    }

    pub async fn broadcast_movement(
        &self,
        mover_session: Arc<WorldSession>,
        opcode: Opcode,
        movement_info: &MovementInfo,
    ) {
        if let Some(current_map_key) = mover_session.get_current_map().await {
            let maps_guard = self.maps.read().await;
            if let Some(map) = maps_guard.get(&current_map_key) {
                let map_guard = map.read().await;
                let player_guard = mover_session.player.read().await;

                for session in map_guard.nearby_sessions(mover_session.account_id).await {
                    session
                        .send_movement(opcode, player_guard.guid(), movement_info)
                        .await
                        .unwrap();
                }
            }
        }
    }

    pub async fn nearby_sessions(
        &self,
        map: Option<MapKey>,
        account_id: u32,
    ) -> Vec<Arc<WorldSession>> {
        let mut result = Vec::new();

        if let Some(current_map_key) = map {
            let maps_guard = self.maps.read().await;
            if let Some(map) = maps_guard.get(&current_map_key) {
                let map_guard = map.read().await;

                let sessions = &mut map_guard.nearby_sessions(account_id).await;
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