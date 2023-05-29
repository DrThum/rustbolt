use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use super::world_session::WorldSession;

pub struct SessionHolder {
    sessions: RwLock<HashMap<u32, Arc<WorldSession>>>,
}

impl SessionHolder {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn insert_session(&self, session: Arc<WorldSession>) -> Option<Arc<WorldSession>> {
        let account_id = session.account_id;
        let mut guard = self.sessions.write().await;
        guard.insert(account_id, session)
    }

    pub async fn get_session_for_account(&self, account_id: u32) -> Option<Arc<WorldSession>> {
        let guard = self.sessions.read().await;
        guard.get(&account_id).cloned()
    }
}
