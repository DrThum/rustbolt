use std::sync::Arc;

use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::datastore::data_types::QuestTemplate;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::{
    PlayerQuestStatus, QuestGiverStatus, MAX_QUEST_CHOICE_REWARDS_COUNT,
    MAX_QUEST_OBJECTIVES_COUNT, MAX_QUEST_REWARDS_COUNT, QUEST_EMOTE_COUNT,
};
use crate::DataStore;

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

#[binread]
pub struct CmsgQuestLogRemoveQuest {
    pub slot: u8,
}

#[binread]
pub struct CmsgQuestGiverCompleteQuest {
    pub guid: u64,
    pub quest_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverRequestItems {
    pub quest_giver_guid: u64,
    pub quest_id: u32,
    pub title: NullString,
    pub text: NullString,
    pub emote_delay: u32,
    pub emote_id: u32,
    pub auto_finish: u32,
    pub suggested_players: u32,
    pub required_money: u32,
    pub required_items_count: u32,
    pub required_items: Vec<QuestGiverRequiredItem>,
    pub flags1: u32,
    pub flags2: u32,
    pub flags3: u32,
    pub flags4: u32,
}

impl SmsgQuestGiverRequestItems {
    pub fn from_template(
        quest_giver_guid: u64,
        status: &PlayerQuestStatus,
        auto_finish: bool,
        template: &QuestTemplate,
        data_store: Arc<DataStore>,
    ) -> Self {
        let completable = *status == PlayerQuestStatus::ObjectivesCompleted;

        let emote_id = if completable {
            template.complete_emote
        } else {
            template.incomplete_emote
        };

        let mut required_items: Vec<QuestGiverRequiredItem> = Vec::new();
        for index in 0..MAX_QUEST_OBJECTIVES_COUNT {
            let item_id = template.required_item_ids[index];
            if item_id == 0 {
                continue;
            }

            let display_id: u32 = data_store
                .get_item_template(item_id)
                .map(|t| t.display_id)
                .unwrap_or_default();

            required_items.push(QuestGiverRequiredItem {
                id: item_id,
                count: template.required_item_counts[index],
                display_id,
            });
        }

        Self {
            quest_giver_guid,
            quest_id: template.entry,
            title: template
                .title
                .as_ref()
                .unwrap_or(&"".to_owned())
                .clone()
                .into(),
            text: template
                .request_items_text
                .as_ref()
                .unwrap_or(&"".to_owned())
                .clone()
                .into(),
            emote_delay: 0,
            emote_id,
            auto_finish: if auto_finish { 1 } else { 0 },
            suggested_players: template.suggested_players,
            required_money: template.required_or_reward_money.max(0).unsigned_abs(),
            required_items_count: required_items.len() as u32,
            required_items,
            flags1: if completable { 3 } else { 0 },
            flags2: 0x04,
            flags3: 0x08,
            flags4: 0x10,
        }
    }
}

#[binwrite]
pub struct QuestGiverRequiredItem {
    pub id: u32,
    pub count: u32,
    pub display_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverOfferReward {
    pub quest_giver_guid: u64,
    pub quest_id: u32,
    pub title: NullString,
    pub text: NullString,
    pub auto_finish: u32,
    pub suggested_players: u32,
    pub emote_count: u32,
    pub emotes: Vec<QuestGiverOfferRewardEmote>,
    pub choice_items_count: u32,
    pub choice_items: Vec<QuestGiverOfferRewardChoiceItem>,
    pub items_count: u32,
    pub items: Vec<QuestGiverOfferRewardItem>,
    pub required_or_reward_money: i32,
    pub honor_reward: u32, // Multiply by 10
    pub unk: u32,          // 0x08
    pub reward_spell: u32,
    pub reward_spell_cast: u32,
    pub character_title_bit_index: u32,
}

impl SmsgQuestGiverOfferReward {
    pub fn from_template(
        quest_giver_guid: u64,
        auto_finish: bool,
        template: &QuestTemplate,
        data_store: Arc<DataStore>,
    ) -> Self {
        let mut emotes: Vec<QuestGiverOfferRewardEmote> = Vec::new();
        for index in 0..MAX_QUEST_OBJECTIVES_COUNT {
            if template.offer_reward_emotes[index] != 0 {
                emotes.push(QuestGiverOfferRewardEmote {
                    delay: template.offer_reward_emote_delays[index],
                    id: template.offer_reward_emotes[index],
                });
            }
        }

        let mut choice_items: Vec<QuestGiverOfferRewardChoiceItem> = Vec::new();
        for index in 0..MAX_QUEST_CHOICE_REWARDS_COUNT {
            let item_id = template.reward_choice_item_ids[index];
            if item_id != 0 {
                let display_id: u32 = data_store
                    .get_item_template(item_id)
                    .map(|t| t.display_id)
                    .unwrap_or_default();

                choice_items.push(QuestGiverOfferRewardChoiceItem {
                    id: item_id,
                    count: template.reward_choice_item_counts[index],
                    display_id,
                });
            }
        }

        let mut items: Vec<QuestGiverOfferRewardItem> = Vec::new();
        for index in 0..MAX_QUEST_REWARDS_COUNT {
            let item_id = template.reward_item_ids[index];
            if item_id != 0 {
                let display_id: u32 = data_store
                    .get_item_template(item_id)
                    .map(|t| t.display_id)
                    .unwrap_or_default();

                items.push(QuestGiverOfferRewardItem {
                    id: item_id,
                    count: template.reward_item_counts[index],
                    display_id,
                });
            }
        }

        Self {
            quest_giver_guid,
            quest_id: template.entry,
            title: template
                .title
                .as_ref()
                .unwrap_or(&"".to_owned())
                .clone()
                .into(),
            text: template
                .offer_reward_text
                .as_ref()
                .unwrap_or(&"".to_owned())
                .clone()
                .into(),
            auto_finish: if auto_finish { 1 } else { 0 },
            suggested_players: template.suggested_players,
            emote_count: emotes.len() as u32,
            emotes,
            choice_items_count: choice_items.len() as u32,
            choice_items,
            items_count: items.len() as u32,
            items,
            required_or_reward_money: template.required_or_reward_money,
            honor_reward: 10 * template.reward_honorable_kills, // TODO: Depends on level
            unk: 0x08,
            reward_spell: template.reward_spell,
            reward_spell_cast: template.reward_spell_cast,
            character_title_bit_index: 0, // TODO: bit_index from CharTitlesStore.dbc
        }
    }
}

#[binwrite]
pub struct QuestGiverOfferRewardEmote {
    pub delay: u32,
    pub id: u32,
}

#[binwrite]
pub struct QuestGiverOfferRewardChoiceItem {
    pub id: u32,
    pub count: u32,
    pub display_id: u32,
}

#[binwrite]
pub struct QuestGiverOfferRewardItem {
    pub id: u32,
    pub count: u32,
    pub display_id: u32,
}

#[binread]
pub struct CmsgQuestGiverChooseReward {
    pub quest_giver_guid: u64,
    pub quest_id: u32,
    pub chosen_reward_index: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverQuestComplete {
    pub quest_id: u32,
    pub unk: u32, // 0x03
    pub xp: u32,
    pub required_or_reward_money: i32,
    pub honorable_kills: u32, // Multiplied by 10
    pub reward_items_count: u32,
    pub reward_items: Vec<QuestCompleteRewardItem>,
}

#[binwrite]
pub struct QuestCompleteRewardItem {
    pub id: u32,
    pub count: u32,
}
