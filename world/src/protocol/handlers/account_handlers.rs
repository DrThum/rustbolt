use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_update_account_data(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_update_account_data: CmsgUpdateAccountData = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgUpdateAccountData {
            account_data_id: cmsg_update_account_data.account_data_id,
            data: 0,
        });

        session.send(&packet).await.unwrap();
    }
}
