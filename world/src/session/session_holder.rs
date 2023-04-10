use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    entities::{object_guid::ObjectGuid, update::UpdatableEntity},
    game::world_context::WorldContext,
    protocol::{
        opcodes::Opcode,
        packets::{MovementInfo, SmsgUpdateObject},
        server::ServerMessage,
    },
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

    // FIXME: Handle object updates in (future) Map instead of here
    // TODO: trait Ticking { fn tick }
    pub async fn tick(&self, world_context: Arc<WorldContext>) {
        let sessions = &*self.sessions.read().await;

        for (account_id, session) in sessions {
            let mut player = session.player.write().await;

            if player.has_changed_since_last_update() {
                // Send the player to themselves
                let guid = player.guid().raw();
                let update_data = player.get_update_data(guid, world_context.clone());
                let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
                    updates_count: update_data.len() as u32,
                    has_transport: false,
                    updates: update_data,
                });

                session.send(&smsg_update_object).await.unwrap();

                // FIXME: this will be handled by the future map system
                for (_, other_session) in sessions.iter().filter(|s| s.0 != account_id) {
                    // Broadcast the change to nearby players
                    let other_player = other_session.player.read().await;
                    let update_data =
                        player.get_update_data(other_player.guid().raw(), world_context.clone());
                    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
                        updates_count: update_data.len() as u32,
                        has_transport: false,
                        updates: update_data,
                    });

                    other_session.send(&smsg_update_object).await.unwrap();
                }

                player.mark_as_up_to_date();
            }
        }
    }
}
