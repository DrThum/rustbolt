use std::sync::Arc;

use crate::{
    game::world_context::WorldContext,
    protocol::{
        client::ClientMessage,
        packets::{CmsgMessageChat, SmsgMessageChat},
        server::ServerMessage,
    },
    session::{opcode_handler::OpcodeHandler, world_session::WorldSession},
};

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_message_chat(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_message_chat: CmsgMessageChat = ClientMessage::read_as(data).unwrap();

        let smsg_message_chat = ServerMessage::new(SmsgMessageChat {
            message_type: 1, // CHAT_MSG_SAY
            language_id: cmsg_message_chat.lang_id,
            sender_guid: session.player.read().await.guid().raw(),
            unk: 0,
            target_guid: 0,
            message_len: cmsg_message_chat.msg.len() as u32 + 1,
            message: cmsg_message_chat.msg,
            chat_tag: 0,
        });

        // Broadcast to nearby players
        world_context
            .map_manager
            .broadcast_packet(session.clone(), &smsg_message_chat, 40.0, true)
            .await;
    }
}
