use std::sync::Arc;

use log::error;

use crate::{
    entities::object_guid::ObjectGuid,
    game::world_context::WorldContext,
    protocol::{
        client::ClientMessage,
        packets::{CmsgMessageChat, CmsgTextEmote, SmsgEmote, SmsgTextEmote},
        server::ServerMessage,
    },
    session::{opcode_handler::OpcodeHandler, world_session::WorldSession},
    shared::constants::{ChatMessageType, Emote},
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

        let player_guid = session.player.read().guid().clone();
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
                    .broadcast_packet(
                        &player_guid,
                        session.get_current_map(),
                        &smsg_message_chat,
                        Some(distance),
                        true,
                    )
                    .await;
            }
            t => error!("unsupported message type {:?}", t),
        }
    }

    pub(crate) async fn handle_cmsg_text_emote(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_text_emote: CmsgTextEmote = ClientMessage::read_as(data).unwrap();
        if let Some(dbc_record) = world_context
            .data_store
            .get_text_emote_record(cmsg_text_emote.text_emote)
        {
            let player_guid = session.player.read().guid().clone();
            if let Some(emote) = Emote::n(dbc_record.text_id) {
                match emote {
                    Emote::StateSleep
                    | Emote::StateSit
                    | Emote::StateKneel
                    | Emote::OneshotNone => (),
                    _ => {
                        let packet = ServerMessage::new(SmsgEmote {
                            emote_id: dbc_record.text_id,
                            origin_guid: session.player.read().guid().raw(),
                        });

                        world_context
                            .map_manager
                            .broadcast_packet(
                                &player_guid,
                                session.get_current_map(),
                                &packet,
                                None,
                                true,
                            )
                            .await;
                    }
                }
            }

            let target_guid =
                ObjectGuid::from_raw(cmsg_text_emote.target_guid).expect("invalid guid received");
            let mut target_name: String = "".to_owned();
            if let Some(entity_ref) = world_context
                .map_manager
                .lookup_entity(&target_guid, session.get_current_map())
                .await
            {
                target_name = entity_ref.read().name();
            }

            let packet = ServerMessage::new(SmsgTextEmote {
                origin_guid: session.player.read().guid().raw(),
                text_emote: cmsg_text_emote.text_emote,
                emote_number: cmsg_text_emote.emote_number,
                target_name_length: target_name.len() as u32,
                target_name: target_name.into(),
            });

            world_context
                .map_manager
                .broadcast_packet(
                    &player_guid,
                    session.get_current_map(),
                    &packet,
                    Some(40.0),
                    true,
                )
                .await;
        }
    }
}
