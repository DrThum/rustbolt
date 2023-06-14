use std::sync::Arc;

use log::error;
use shipyard::{Get, View};

use crate::{
    entities::{creature::Creature, object_guid::ObjectGuid, player::Player},
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
    pub(crate) fn handle_cmsg_message_chat(
        session: Arc<WorldSession>,
        _world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_message_chat: CmsgMessageChat = ClientMessage::read_as(data).unwrap();

        // TODO: Check that the language exists
        // TODO: Check that the player has the associated skill

        match cmsg_message_chat.chat_type {
            ChatMessageType::Say | ChatMessageType::Yell | ChatMessageType::Emote => {
                let smsg_message_chat = ServerMessage::new(session.build_chat_packet(
                    cmsg_message_chat.chat_type.clone(),
                    cmsg_message_chat.language,
                    None,
                    cmsg_message_chat.msg,
                ));

                let distance = match cmsg_message_chat.chat_type {
                    ChatMessageType::Say | ChatMessageType::Emote => 40.0,
                    ChatMessageType::Yell => 300.0,
                    _ => 0.0,
                };

                // Broadcast to nearby players
                session.current_map().unwrap().broadcast_packet(
                    &session.player_guid().unwrap(),
                    &smsg_message_chat,
                    Some(distance),
                    true,
                );
            }
            t => error!("unsupported message type {:?}", t),
        }
    }

    pub(crate) fn handle_cmsg_text_emote(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
        data: Vec<u8>,
    ) {
        let cmsg_text_emote: CmsgTextEmote = ClientMessage::read_as(data).unwrap();
        if let Some(dbc_record) = world_context
            .data_store
            .get_text_emote_record(cmsg_text_emote.text_emote)
        {
            if let Some(emote) = Emote::n(dbc_record.text_id) {
                match emote {
                    Emote::StateSleep
                    | Emote::StateSit
                    | Emote::StateKneel
                    | Emote::OneshotNone => (),
                    _ => {
                        let player_guid = session.player_guid().unwrap();
                        let packet = ServerMessage::new(SmsgEmote {
                            emote_id: dbc_record.text_id,
                            origin_guid: player_guid.raw(),
                        });

                        session.current_map().unwrap().broadcast_packet(
                            &player_guid,
                            &packet,
                            None,
                            true,
                        );
                    }
                }
            }

            let target_guid =
                ObjectGuid::from_raw(cmsg_text_emote.target_guid).expect("invalid guid received");

            if let Some(map) = session.current_map() {
                if let Some(target_entity_id) = map.lookup_entity_ecs(&target_guid) {
                    let target_name =
                        map.world()
                            .run(|v_player: View<Player>, v_creature: View<Creature>| {
                                if let Some(player) = v_player.get(target_entity_id).ok() {
                                    player.name.clone()
                                } else if let Some(creature) = v_creature.get(target_entity_id).ok()
                                {
                                    creature.name.clone()
                                } else {
                                    "TODO_TARGET_NAME".to_owned()
                                }
                            });

                    let player_guid = session.player_guid().unwrap();
                    let packet = ServerMessage::new(SmsgTextEmote {
                        origin_guid: player_guid.raw(),
                        text_emote: cmsg_text_emote.text_emote,
                        emote_number: cmsg_text_emote.emote_number,
                        target_name_length: target_name.len() as u32,
                        target_name: target_name.into(),
                    });

                    session.current_map().unwrap().broadcast_packet(
                        &player_guid,
                        &packet,
                        Some(40.0),
                        true,
                    );
                }
            }
        }
    }
}
