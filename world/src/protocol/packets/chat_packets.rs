use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::{ChatMessageType, Language};

#[binread]
pub struct CmsgMessageChat {
    #[br(map = |ct: u32| ChatMessageType::n(ct).expect("non-existing ChatMessageType"))]
    pub chat_type: ChatMessageType,
    #[br(map = |ct: u32| Language::n(ct).expect("non-existing Language"))]
    pub language: Language,
    #[br(if(chat_type == ChatMessageType::Whisper))]
    _recipient: Option<NullString>,
    #[br(if(chat_type == ChatMessageType::Channel))]
    _channel: Option<NullString>,
    pub msg: NullString,
}

#[binwrite]
#[server_opcode]
pub struct SmsgMessageChat {
    // FIXME: Incomplete
    #[bw(map = |&t| t as u8)]
    pub message_type: ChatMessageType,
    #[bw(map = |&l| l as u32)]
    pub language: Language,
    pub sender_guid: u64, // TODO: ObjectGuid?
    pub unk: u32,         // 0,
    pub target_guid: u64,
    pub message_len: u32,
    pub message: NullString,
    pub chat_tag: u8, // 0 for now
}

impl SmsgMessageChat {
    pub fn build(
        message_type: ChatMessageType,
        language: Language,
        sender_guid: Option<&ObjectGuid>,
        target_guid: Option<&ObjectGuid>,
        message: NullString,
    ) -> Self {
        Self {
            message_type,
            language,
            sender_guid: sender_guid.map_or(0, |g| g.raw()),
            unk: 0,
            target_guid: target_guid.map_or(0, |g| g.raw()),
            message_len: message.len() as u32 + 1,
            message,
            chat_tag: 0, // TODO: Implement chat tags (GM, AFK, DND)
        }
    }
}

#[binread]
pub struct CmsgTextEmote {
    pub text_emote: u32,
    pub emote_number: u32,
    pub target_guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgEmote {
    pub emote_id: u32,
    pub origin_guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgTextEmote {
    pub origin_guid: u64,
    pub text_emote: u32,
    pub emote_number: u32,
    pub target_name_length: u32,
    pub target_name: NullString,
}
