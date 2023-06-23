use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::{
    protocol::{opcodes::Opcode, server::ServerMessagePayload},
    shared::constants::{
        MAX_QUEST_CHOICE_REWARDS_COUNT, MAX_QUEST_OBJECTIVES_COUNT, MAX_QUEST_REWARDS_COUNT,
        NPC_TEXT_EMOTE_COUNT,
    },
};

#[binread]
pub struct CmsgNameQuery {
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgNameQueryResponse {
    pub guid: u64,
    pub name: NullString,
    pub realm_name: u8, // Use 0, intended for cross-realm battlegrounds
    pub race: u32,
    pub class: u32,
    pub gender: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub is_name_declined: bool, // use false
                                // pub declined_names: [NullString, 5],
}

#[binread]
pub struct CmsgCreatureQuery {
    pub entry: u32,
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgCreatureQueryResponse {
    pub entry: u32,
    pub name: NullString,
    pub name2: u8, // 0
    pub name3: u8, // 0
    pub name4: u8, // 0
    pub sub_name: NullString,
    pub icon_name: NullString,
    pub type_flags: u32,
    pub type_id: u32,
    pub family: u32,
    pub rank: u32,
    pub unk: u32, // 0
    pub pet_spell_data_id: u32,
    pub model_ids: Vec<u32>,
    pub health_multiplier: f32,
    pub power_multiplier: f32,
    pub racial_leader: u8,
}

#[binwrite]
pub struct SmsgCreatureQueryResponseUnknownTemplate {
    pub masked_entry: u32,
}
impl ServerMessagePayload<{ Opcode::SmsgCreatureQueryResponse as u16 }>
    for SmsgCreatureQueryResponseUnknownTemplate
{
}

#[binread]
pub struct CmsgNpcTextQuery {
    pub text_id: u32,
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgNpcTextUpdate {
    pub text_id: u32,
    pub probability: f32,
    pub text0: NullString,
    pub text1: NullString,
    pub language: u32,
    pub emotes: [NpcTextUpdateEmote; NPC_TEXT_EMOTE_COUNT],
}

#[binwrite]
pub struct NpcTextUpdateEmote {
    pub delay: u32,
    pub emote_id: u32,
}

#[binread]
pub struct CmsgQuestQuery {
    pub quest_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestQueryResponse {
    pub quest_id: u32,
    pub method: u32,
    pub quest_level: i32,
    pub zone_or_sort: i32,
    pub quest_type: u32,
    pub suggested_player: u32,
    pub rep_objective_faction: u32,
    pub rep_objective_value: u32,
    pub required_opposite_rep_faction: u32, // Always 0
    pub required_opposite_rep_value: u32,   // Always 0
    pub next_quest_in_chain: u32,
    pub required_or_reward_money: i32,
    pub reward_money_max_level: u32,
    pub reward_spell: u32,
    pub reward_spell_cast: u32,
    pub reward_honorable_kills: u32,
    pub source_item_id: u32,
    pub quest_flags: u32,
    pub character_title_id: u32,
    pub reward_items_id: [u32; MAX_QUEST_REWARDS_COUNT],
    pub reward_items_count: [u32; MAX_QUEST_REWARDS_COUNT],
    pub reward_choice_items_id: [u32; MAX_QUEST_CHOICE_REWARDS_COUNT],
    pub reward_choice_items_count: [u32; MAX_QUEST_CHOICE_REWARDS_COUNT],
    pub point_map_id: u32,
    pub point_x: f32,
    pub point_y: f32,
    pub point_opt: u32,
    pub title: NullString,
    pub objectives: NullString,
    pub details: NullString,
    pub end_text: NullString,
    pub required_entities_and_items:
        [[u32; MAX_QUEST_OBJECTIVES_COUNT]; MAX_QUEST_OBJECTIVES_COUNT],
    pub objective_texts: [NullString; MAX_QUEST_OBJECTIVES_COUNT],
}
