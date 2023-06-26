use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

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

    pub fn insert_session(&self, session: Arc<WorldSession>) -> Option<Arc<WorldSession>> {
        let account_id = session.account_id;
        self.sessions.write().insert(account_id, session)
    }

    pub fn remove_session(&self, account_id: u32) {
        self.sessions.write().remove(&account_id);
    }

    pub fn get_session_for_account(&self, account_id: u32) -> Option<Arc<WorldSession>> {
        self.sessions.read().get(&account_id).cloned()
    }
}
