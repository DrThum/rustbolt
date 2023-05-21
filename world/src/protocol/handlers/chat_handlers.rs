use std::sync::Arc;

use log::error;

use crate::{
    game::world_context::WorldContext,
    protocol::{client::ClientMessage, packets::CmsgMessageChat, server::ServerMessage},
    session::{opcode_handler::OpcodeHandler, world_session::WorldSession},
    shared::constants::ChatMessageType,
};

impl OpcodeHandler {
    pub(crate) async fn handle_cmsg_message_chat(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_message_chat: CmsgMessageChat = ClientMessage::read_as(data).unwrap();

        // TODO: Check that the language exists
        // TODO: Check that the player has the associated skill

        match cmsg_message_chat.chat_type {
            ChatMessageType::Say | ChatMessageType::Yell | ChatMessageType::Emote => {
                let smsg_message_chat = ServerMessage::new(
                    session
                        .build_chat_packet(
                            cmsg_message_chat.chat_type.clone(),
                            cmsg_message_chat.language,
                            None,
                            cmsg_message_chat.msg,
                        )
                        .await,
                );

                let distance = match cmsg_message_chat.chat_type {
                    ChatMessageType::Say | ChatMessageType::Emote => 40.0,
                    ChatMessageType::Yell => 300.0,
                    _ => 0.0,
                };

                // Broadcast to nearby players
                world_context
                    .map_manager
                    .broadcast_packet(session.clone(), &smsg_message_chat, distance, true)
                    .await;
            }
            t => error!("unsupported message type {:?}", t),
        }
    }
}
