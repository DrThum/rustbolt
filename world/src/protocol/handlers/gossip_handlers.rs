use std::sync::Arc;

use crate::game::world_context::WorldContext;
use crate::protocol::client::ClientMessage;
use crate::protocol::packets::*;
use crate::session::opcode_handler::OpcodeHandler;
use crate::session::world_session::WorldSession;

impl OpcodeHandler {
    pub(crate) fn handle_cmsg_gossip_hello(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
        _vm_all_storages: Option<AllStoragesViewMut>,
    ) {
        let cmsg: CmsgGossipHello = ClientMessage::read_as(data).unwrap();

        OpcodeHandler::send_initial_gossip_menu(cmsg.guid, session.clone(), world_context.clone());
    }
}
