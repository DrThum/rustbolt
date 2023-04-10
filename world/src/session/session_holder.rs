use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    entities::object_guid::ObjectGuid,
    protocol::server::{ServerMessage, ServerMessagePayload},
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

    pub async fn send_around<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: &ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> {
        let guard = self.sessions.read().await;

        // FIXME: Actually send only around once we have a map system
        for (_, session) in &*guard {
            if session.is_in_world().await {
                session.send(packet).await?;
            }
        }
        Ok(())
    }

    // All sessions around me except myself
    // FIXME: Use the future map system to only include nearby players
    pub async fn sessions_around(&self, player_guid: &ObjectGuid) -> Vec<Arc<WorldSession>> {
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
