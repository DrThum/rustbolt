use std::sync::Arc;

use log::warn;

use crate::entities::object_guid::ObjectGuid;
use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
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

                    // If melee
                    let packet = ServerMessage::new(SmsgAttackStart {
                        attacker_guid: session.player.read().await.guid().raw(),
                        target_guid: cmsg.guid,
                    });

                    world_context
                        .map_manager
                        .broadcast_packet(session, &packet, None, true)
                        .await;
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

    pub(crate) async fn handle_cmsg_attack_stop(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        _data: Vec<u8>,
    ) {
        let packet = {
            let player_guard = session.player.read().await;
            ServerMessage::new(SmsgAttackStop {
                player_guid: player_guard.guid().as_packed(),
                enemy_guid: player_guard
                    .selection()
                    .unwrap_or(ObjectGuid::zero())
                    .as_packed(),
                unk: 0,
            })
        };

        world_context
            .map_manager
            .broadcast_packet(session.clone(), &packet, None, true)
            .await;

        let mut player_guard = session.player.write().await;
        player_guard.set_selection(0);
    }
}
