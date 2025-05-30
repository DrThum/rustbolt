use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::protocol::server::ServerMessage;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_update_account_data(
        PacketHandlerArgs { session, data, .. }: PacketHandlerArgs,
    ) {
        let cmsg_update_account_data: CmsgUpdateAccountData = ClientMessage::read_as(data).unwrap();

        let packet = ServerMessage::new(SmsgUpdateAccountData {
            account_data_id: cmsg_update_account_data.account_data_id,
            data: 0,
        });

        session.send(&packet).unwrap();
    }
}
