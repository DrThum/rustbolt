use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::world_session::WorldSession;

pub struct SessionHolder {
    sessions: HashMap<u32, Arc<RwLock<WorldSession>>>,
}

impl SessionHolder {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn insert_session(
        &mut self,
        session: WorldSession,
    ) -> Option<Arc<RwLock<WorldSession>>> {
        let account_id = session.account_id;
        let session = Arc::new(RwLock::new(session));
        self.sessions.insert(account_id, session)
    }

    pub async fn get_session_for_account(
        &self,
        account_id: u32,
    ) -> Option<&Arc<RwLock<WorldSession>>> {
        self.sessions.get(&account_id)
    }
}
