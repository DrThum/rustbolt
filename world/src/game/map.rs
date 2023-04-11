use std::{collections::HashMap, sync::Arc};

use log::warn;
use tokio::sync::RwLock;

use crate::session::world_session::WorldSession;

use super::map_manager::MapKey;

// TODO:
// - Keep an Arc<Map> in WorldSession for fast access
pub struct Map {
    key: MapKey,
    sessions: RwLock<HashMap<u32, Arc<WorldSession>>>,
}

impl Map {
    pub fn new(key: MapKey) -> Self {
        Self {
            key,
            sessions: RwLock::new(HashMap::new()),
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
}
