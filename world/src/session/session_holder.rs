use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    entities::object_guid::ObjectGuid,
    protocol::{opcodes::Opcode, packets::MovementInfo},
};

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

    pub async fn broadcast_movement(
        &self,
        opcode: Opcode,
        movement_info: &MovementInfo,
        origin: &ObjectGuid,
    ) -> Result<(), binrw::Error> {
        for session in self.nearby_sessions(origin).await {
            session.send_movement(opcode, origin, movement_info).await?;
        }

        Ok(())
    }

    // All sessions around me except myself
    // FIXME: Use the future map system to only include nearby players
    pub async fn nearby_sessions(&self, player_guid: &ObjectGuid) -> Vec<Arc<WorldSession>> {
        let mut result = Vec::new();

        let guard = self.sessions.read().await;

        for (_, session) in &*guard {
            if session.is_in_world().await && *session.player.read().await.guid() != *player_guid {
                result.push(session.clone());
            }
        }

        result
    }
}
