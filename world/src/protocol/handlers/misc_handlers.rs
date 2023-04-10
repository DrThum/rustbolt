use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_realm_split(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_realm_split: CmsgRealmSplit = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgRealmSplit {
            client_state: cmsg_realm_split.client_state,
            realm_state: 0x00,
            split_date: binrw::NullString::from("01/01/01"),
        });

        session.send(&packet).await.unwrap();
    }
}
