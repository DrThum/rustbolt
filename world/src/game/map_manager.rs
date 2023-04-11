use std::{collections::HashMap, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use tokio::sync::RwLock;

use crate::DataStore;

use super::map::Map;

pub enum MapManagerError {
    UnknownMapId,
    CommonMapAlreadyInstanced,
}

type MapKey = (u32, Option<u32>); // map id, instance id

pub struct MapManager {
    maps: RwLock<HashMap<MapKey, Arc<Map>>>,
    data_store: Arc<DataStore>,
    next_instance_id: RelaxedCounter,
}

impl MapManager {
    pub async fn create_with_continents(data_store: Arc<DataStore>) -> Self {
        let mut maps: HashMap<MapKey, Arc<Map>> = HashMap::new();

        // Instantiate all common (= continents) maps on startup
        for map in data_store.get_all_map_records().filter(|m| !m.is_instanceable()) {
            let key = (map.id, None);
            maps.insert(key, Arc::new(Map::new(key.0, key.1)));
        }

        Self {
            maps: RwLock::new(maps),
            data_store,
            next_instance_id: RelaxedCounter::new(1),
        }
    }

    pub async fn instantiate_map(&self, map_id: u32) -> Result<Arc<Map>, MapManagerError> {
        match self.data_store.get_map_record(map_id) {
            None => Err(MapManagerError::UnknownMapId),
            Some(map_record) => {
                if !map_record.is_instanceable() {
                    return Err(MapManagerError::CommonMapAlreadyInstanced);
                } else {
                    let mut guard = self.maps.write().await;
                    let instance_id: Option<u32> = if map_record.is_instanceable() {
                        Some(self.next_instance_id.inc().try_into().unwrap())
                    } else {
                        None
                    };

                    let map = Arc::new(Map::new(map_id, instance_id));
                    guard.insert((map_id, instance_id), map.clone());

                    Ok(map)
                }
            }
        }
    }

    pub async fn get_map(&self, map_id: u32, instance_id: Option<u32>) -> Option<Arc<Map>> {
        let guard = self.maps.read().await;
        guard.get(&(map_id, instance_id)).cloned()
    }
}
