use std::sync::Arc;

use log::{info, warn};

use crate::entities::object_guid::ObjectGuid;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_attack_swing(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg: CmsgAttackSwing = ClientMessage::read_as(data).unwrap();
        if let Some(target_guid) = ObjectGuid::from_raw(cmsg.guid) {
            match world_context
                .map_manager
                .lookup_entity(&target_guid, session.get_current_map().await)
                .await
            {
                Some(entity) => {
                    let entity_guard = entity.read().await;
                    if !entity_guard.guid().is_unit() {
                        warn!("player attempted to attack non-unit entity {target_guid:?}");
                        session.send_attack_stop(Some(target_guid)).await;
                        return;
                    }

                    // Enter combat
                    info!("entering combat");
                }
                None => {
                    warn!("player attempted to attack non-existing entity {target_guid:?}");
                    session.send_attack_stop(Some(target_guid)).await;
                }
            }
        } else {
            session.send_attack_stop(None).await;
        }
    }
}
