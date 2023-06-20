use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

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
    pub recipient: Option<NullString>,
    #[br(if(chat_type == ChatMessageType::Channel))]
    pub channel: Option<NullString>,
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
