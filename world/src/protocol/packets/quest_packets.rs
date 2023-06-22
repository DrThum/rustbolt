use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::{QuestGiverStatus, QUEST_EMOTE_COUNT};

#[binread]
pub struct CmsgQuestGiverStatusQuery {
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverStatus {
    pub guid: u64,
    #[bw(map = |status: &QuestGiverStatus| *status as u8)]
    pub status: QuestGiverStatus,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverStatusMultiple {
    pub count: u32,
    pub statuses: Vec<QuestGiverStatusMultipleEntry>,
}

#[binwrite]
pub struct QuestGiverStatusMultipleEntry {
    pub guid: u64,
    #[bw(map = |status: &QuestGiverStatus| *status as u8)]
    pub status: QuestGiverStatus,
}

#[binread]
pub struct CmsgQuestGiverHello {
    pub guid: u64,
}

#[binread]
pub struct CmsgQuestGiverQueryQuest {
    pub guid: u64,
    pub quest_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverQuestDetails {
    pub guid: u64,
    pub quest_id: u32,
    pub title: NullString,
    pub details: NullString,
    pub objectives: NullString,
    #[bw(map = |b: &bool| if *b { 1_u32 } else { 0_u32 })]
    pub auto_accept: bool,
    pub suggested_players: u32,
    pub reward_choice_items_count: u32,
    pub reward_choice_items: Vec<QuestDetailsItemRewards>,
    pub reward_items_count: u32,
    pub reward_items: Vec<QuestDetailsItemRewards>,
    pub required_or_reward_money: i32,
    pub honor_reward: u32, // Need to multiply by 10
    pub reward_spell: u32,
    pub reward_spell_cast: u32,
    pub reward_title_bit_index: u32,
    pub emotes: [QuestDetailsEmote; QUEST_EMOTE_COUNT],
}

#[binwrite]
pub struct QuestDetailsItemRewards {
    pub item_id: u32,
    pub item_count: u32,
    pub item_display_id: u32,
}

#[binwrite]
pub struct QuestDetailsEmote {
    pub emote: u32,
    pub delay: u32,
}

#[binread]
pub struct CmsgQuestGiverAcceptQuest {
    pub guid: u64,
    pub quest_id: u32,
}
