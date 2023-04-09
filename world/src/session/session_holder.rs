use std::{collections::HashMap, sync::Arc};

use super::world_session::WorldSession;

pub struct SessionHolder {
    sessions: HashMap<u32, Arc<WorldSession>>,
}

impl SessionHolder {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
        }
    }

    pub async fn insert_session(
        &mut self,
        session: Arc<WorldSession>,
    ) -> Option<Arc<WorldSession>> {
        let account_id = session.account_id;
        self.sessions.insert(account_id, session)
    }

    pub fn get_session_for_account(&self, account_id: u32) -> Option<&Arc<WorldSession>> {
        self.sessions.get(&account_id)
    }
}
