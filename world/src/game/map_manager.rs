use std::{collections::HashMap, fmt::Display, sync::Arc};

use atomic_counter::{AtomicCounter, RelaxedCounter};
use log::warn;
use tokio::sync::RwLock;

use crate::{session::world_session::WorldSession, DataStore};

use super::map::Map;

pub enum MapManagerError {
    UnknownMapId,
    CommonMapAlreadyInstanced,
}

pub struct MapManager {
    maps: RwLock<HashMap<MapKey, Arc<RwLock<Map>>>>,
    data_store: Arc<DataStore>,
    next_instance_id: RelaxedCounter,
}

impl MapManager {
    pub async fn create_with_continents(data_store: Arc<DataStore>) -> Self {
        let mut maps: HashMap<MapKey, Arc<RwLock<Map>>> = HashMap::new();

        // Instantiate all common (= continents) maps on startup
        for map in data_store
            .get_all_map_records()
            .filter(|m| !m.is_instanceable())
        {
            let key = MapKey::for_continent(map.id);
            maps.insert(key, Arc::new(RwLock::new(Map::new(key))));
        }

        Self {
            maps: RwLock::new(maps),
            data_store,
            next_instance_id: RelaxedCounter::new(1),
        }
    }

    pub async fn instantiate_map(&self, map_id: u32) -> Result<Arc<RwLock<Map>>, MapManagerError> {
        match self.data_store.get_map_record(map_id) {
            None => Err(MapManagerError::UnknownMapId),
            Some(map_record) => {
                if !map_record.is_instanceable() {
                    return Err(MapManagerError::CommonMapAlreadyInstanced);
                } else {
                    let mut guard = self.maps.write().await;
                    let instance_id: u32 = self.next_instance_id.inc().try_into().unwrap();

                    let map_key = MapKey::for_instance(map_id, instance_id);
                    let map = Arc::new(RwLock::new(Map::new(map_key)));
                    guard.insert(map_key, map.clone());

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
            destination_map.write().await.add_player(session.clone()).await;
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
