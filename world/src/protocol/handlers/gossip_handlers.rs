use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::session::opcode_handler::{OpcodeHandler, PacketHandlerArgs};

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_gossip_hello(
        PacketHandlerArgs {
            session,
            world_context,
            data,
            ..
        }: PacketHandlerArgs,
    ) {
        let cmsg: CmsgGossipHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }
}
