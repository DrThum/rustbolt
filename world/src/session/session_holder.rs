use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{
    entities::update::UpdatableEntity,
    game::world_context::WorldContext,
    protocol::{packets::SmsgUpdateObject, server::ServerMessage},
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

    // FIXME: Handle object updates in (future) Map instead of here
    // TODO: trait Ticking { fn tick }
    pub async fn tick(&self, world_context: Arc<WorldContext>) {
        let sessions = &*self.sessions.read().await;

        for (_, session) in sessions {
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

                let map = session.get_current_map().await;

                for other_session in world_context
                    .map_manager
                    .nearby_sessions(map, player.guid(), false)
                    .await
                {
                    // Broadcast the change to nearby players
                    let other_player = other_session.player.read().await;
                    let update_data =
                        player.get_update_data(other_player.guid().raw(), world_context.clone());
                    let smsg_update_object = ServerMessage::new(SmsgUpdateObject {
                        updates_count: update_data.len() as u32,
                        has_transport: false,
                        updates: update_data,
                    });

                    // TODO: implement and use WorldSession::update_entity
                    other_session.send(&smsg_update_object).await.unwrap();
                }

                player.mark_as_up_to_date();
            }
        }
    }
}
