use std::{collections::HashMap, sync::Arc};

use log::warn;
use shared::models::terrain_info::TerrainBlock;
use tokio::sync::RwLock;

use crate::session::world_session::WorldSession;

use super::map_manager::{MapKey, TerrainBlockCoords};

pub struct Map {
    key: MapKey,
    sessions: RwLock<HashMap<u32, Arc<WorldSession>>>,
    terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>,
}

impl Map {
    pub fn new(key: MapKey, terrain: Arc<HashMap<TerrainBlockCoords, TerrainBlock>>) -> Self {
        Self {
            key,
            sessions: RwLock::new(HashMap::new()),
            terrain,
        }
    }

    pub async fn add_player(&mut self, session: Arc<WorldSession>) {
        let mut guard = self.sessions.write().await;
        if let Some(previous_session) = guard.insert(session.account_id, session) {
            warn!(
                "session from account {} was already on map {}",
                previous_session.account_id, self.key
            );
        }
    }

    pub async fn remove_player(&mut self, account_id: u32) {
        let mut guard = self.sessions.write().await;
        if let None = guard.remove(&account_id) {
            warn!(
                "session from account {} was not on map {}",
                account_id, self.key
            );
        }
    }

    // All sessions around me except myself
    // FIXME: Use the future k-d tree to only include nearby players
    pub async fn nearby_sessions(&self, account_id: u32) -> Vec<Arc<WorldSession>> {
        let mut result = Vec::new();

        let guard = self.sessions.read().await;

        for (_, session) in &*guard {
            if session.is_in_world().await && session.account_id != account_id {
                result.push(session.clone());
            }
        }

        result
    }
}
