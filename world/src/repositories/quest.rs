use std::time::Duration;

use enumflags2::BitFlags;
use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;

use crate::datastore::data_types::{QuestActorRole, QuestActorType, QuestRelation, QuestTemplate};

pub struct QuestRepository;

impl QuestRepository {
    pub fn load_templates(conn: &PooledConnection<SqliteConnectionManager>) -> Vec<QuestTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM quest_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached(
            "SELECT entry, method, zone_or_sort, min_level, level, type, required_classes, required_races,
            required_skill, required_skill_value, rep_objective_faction, rep_objective_value,
            required_min_rep_faction, required_min_rep_value, required_max_rep_faction, required_max_rep_value,
            suggested_players, time_limit, flags, special_flags, character_title, previous_quest_id, next_quest_id,
            exclusive_group, next_quest_in_chain, source_item_id, source_item_count, source_spell, title, details,
            objectives, offer_reward_text, request_items_text, end_text, objective_text1, objective_text2,
            objective_text3, objective_text4, required_item_id1, required_item_id2, required_item_id3,
            required_item_id4, required_item_count1, required_item_count2, required_item_count3, required_item_count4,
            required_source_item_id1, required_source_item_id2, required_source_item_id3, required_source_item_id4,
            required_source_item_count1, required_source_item_count2, required_source_item_count3, required_source_item_count4,
            required_entity_id1, required_entity_id2, required_entity_id3, required_entity_id4,
            required_entity_count1, required_entity_count2, required_entity_count3, required_entity_count4,
            required_spell_cast1, required_spell_cast2, required_spell_cast3, required_spell_cast4, reward_choice_item_id1,
            reward_choice_item_id2, reward_choice_item_id3, reward_choice_item_id4, reward_choice_item_id5, reward_choice_item_id6,
            reward_choice_item_count1, reward_choice_item_count2, reward_choice_item_count3, reward_choice_item_count4,
            reward_choice_item_count5, reward_choice_item_count6, reward_item_id1, reward_item_id2, reward_item_id3, reward_item_id4,
            reward_item_count1, reward_item_count2, reward_item_count3, reward_item_count4,
            reward_rep_faction1, reward_rep_faction2, reward_rep_faction3, reward_rep_faction4, reward_rep_faction5,
            reward_rep_value1, reward_rep_value2, reward_rep_value3, reward_rep_value4, reward_rep_value5,
            reward_honorable_kills, required_or_reward_money, reward_money_max_level, reward_spell, reward_spell_cast,
            reward_mail_template_id, reward_mail_delay_seconds, point_map_id, point_x, point_y, point_opt, details_emote1,
            details_emote2, details_emote3, details_emote4, details_emote_delay1, details_emote_delay2, details_emote_delay3,
            details_emote_delay4, incomplete_emote, complete_emote, offer_reward_emote1, offer_reward_emote2, offer_reward_emote3,
            offer_reward_emote4, offer_reward_emote_delay1, offer_reward_emote_delay2, offer_reward_emote_delay3, offer_reward_emote_delay4
            FROM quest_templates ORDER BY entry").unwrap();

        use QuestTemplateColumnIndex::*;

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(QuestTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    method: row.get(Method as usize).unwrap(),
                    zone_or_sort: row.get(ZoneOrSort as usize).unwrap(),
                    min_level: row.get(MinLevel as usize).unwrap(),
                    level: row.get(Level as usize).unwrap(),
                    type_: row.get(Type as usize).unwrap(),
                    required_classes: unsafe {
                        row.get::<usize, u32>(RequiredClasses as usize)
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    required_races: unsafe {
                        row.get::<usize, u32>(RequiredRaces as usize)
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    required_skill: row.get(RequiredSkill as usize).unwrap(),
                    required_skill_value: row.get(RequiredSkillValue as usize).unwrap(),
                    rep_objective_faction: row.get(RepObjectiveFaction as usize).unwrap(),
                    rep_objective_value: row.get(RepObjectiveValue as usize).unwrap(),
                    required_min_rep_faction: row.get(RequiredMinRepFaction as usize).unwrap(),
                    required_min_rep_value: row.get(RequiredMinRepValue as usize).unwrap(),
                    required_max_rep_faction: row.get(RequiredMaxRepFaction as usize).unwrap(),
                    required_max_rep_value: row.get(RequiredMaxRepValue as usize).unwrap(),
                    suggested_players: row.get(SuggestedPlayers as usize).unwrap(),
                    time_limit: row
                        .get::<usize, Option<u64>>(TimeLimit as usize)
                        .map(|limit| limit.map(|l| Duration::from_millis(l)))
                        .unwrap(),
                    flags: unsafe {
                        row.get::<usize, u32>(Flags as usize)
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    special_flags: row.get(SpecialFlags as usize).unwrap(),
                    character_title: row.get(CharacterTitle as usize).unwrap(),
                    previous_quest_id: row.get(PreviousQuestId as usize).unwrap(),
                    next_quest_id: row.get(NextQuestId as usize).unwrap(),
                    exclusive_group: row.get(ExclusiveGroup as usize).unwrap(),
                    next_quest_in_chain: row.get(NextQuestInChain as usize).unwrap(),
                    source_item_id: row.get(SourceItemId as usize).unwrap(),
                    source_item_count: row.get(SourceItemCount as usize).unwrap(),
                    source_spell: row.get(SourceSpell as usize).unwrap(),
                    title: row.get(Title as usize).unwrap(),
                    details: row.get(Details as usize).unwrap(),
                    objectives: row.get(Objectives as usize).unwrap(),
                    offer_reward_text: row.get(OfferRewardText as usize).unwrap(),
                    request_items_text: row.get(RequestItemsText as usize).unwrap(),
                    end_text: row.get(EndText as usize).unwrap(),
                    objective_text1: row.get(ObjectiveText1 as usize).unwrap(),
                    objective_text2: row.get(ObjectiveText2 as usize).unwrap(),
                    objective_text3: row.get(ObjectiveText3 as usize).unwrap(),
                    objective_text4: row.get(ObjectiveText4 as usize).unwrap(),
                    required_item_ids: [
                        row.get(RequiredItemId1 as usize).unwrap(),
                        row.get(RequiredItemId2 as usize).unwrap(),
                        row.get(RequiredItemId3 as usize).unwrap(),
                        row.get(RequiredItemId4 as usize).unwrap(),
                    ],
                    required_item_counts: [
                        row.get(RequiredItemCount1 as usize).unwrap(),
                        row.get(RequiredItemCount2 as usize).unwrap(),
                        row.get(RequiredItemCount3 as usize).unwrap(),
                        row.get(RequiredItemCount4 as usize).unwrap(),
                    ],
                    required_source_item_ids: [
                        row.get(RequiredSourceItemId1 as usize).unwrap(),
                        row.get(RequiredSourceItemId2 as usize).unwrap(),
                        row.get(RequiredSourceItemId3 as usize).unwrap(),
                        row.get(RequiredSourceItemId4 as usize).unwrap(),
                    ],
                    required_source_item_counts: [
                        row.get(RequiredSourceItemCount1 as usize).unwrap(),
                        row.get(RequiredSourceItemCount2 as usize).unwrap(),
                        row.get(RequiredSourceItemCount3 as usize).unwrap(),
                        row.get(RequiredSourceItemCount4 as usize).unwrap(),
                    ],
                    required_entity_ids: [
                        row.get(RequiredEntityId1 as usize).unwrap(),
                        row.get(RequiredEntityId2 as usize).unwrap(),
                        row.get(RequiredEntityId3 as usize).unwrap(),
                        row.get(RequiredEntityId4 as usize).unwrap(),
                    ],
                    required_entity_counts: [
                        row.get(RequiredEntityCount1 as usize).unwrap(),
                        row.get(RequiredEntityCount2 as usize).unwrap(),
                        row.get(RequiredEntityCount3 as usize).unwrap(),
                        row.get(RequiredEntityCount4 as usize).unwrap(),
                    ],
                    required_spell_casts: [
                        row.get(RequiredSpellCast1 as usize).unwrap(),
                        row.get(RequiredSpellCast2 as usize).unwrap(),
                        row.get(RequiredSpellCast3 as usize).unwrap(),
                        row.get(RequiredSpellCast4 as usize).unwrap(),
                    ],
                    reward_choice_item_ids: [
                        row.get(RewardChoiceItemId1 as usize).unwrap(),
                        row.get(RewardChoiceItemId2 as usize).unwrap(),
                        row.get(RewardChoiceItemId3 as usize).unwrap(),
                        row.get(RewardChoiceItemId4 as usize).unwrap(),
                        row.get(RewardChoiceItemId5 as usize).unwrap(),
                        row.get(RewardChoiceItemId6 as usize).unwrap(),
                    ],
                    reward_choice_item_counts: [
                        row.get(RewardChoiceItemCount1 as usize).unwrap(),
                        row.get(RewardChoiceItemCount2 as usize).unwrap(),
                        row.get(RewardChoiceItemCount3 as usize).unwrap(),
                        row.get(RewardChoiceItemCount4 as usize).unwrap(),
                        row.get(RewardChoiceItemCount5 as usize).unwrap(),
                        row.get(RewardChoiceItemCount6 as usize).unwrap(),
                    ],
                    reward_item_ids: [
                        row.get(RewardItemId1 as usize).unwrap(),
                        row.get(RewardItemId2 as usize).unwrap(),
                        row.get(RewardItemId3 as usize).unwrap(),
                        row.get(RewardItemId4 as usize).unwrap(),
                    ],
                    reward_item_counts: [
                        row.get(RewardItemCount1 as usize).unwrap(),
                        row.get(RewardItemCount2 as usize).unwrap(),
                        row.get(RewardItemCount3 as usize).unwrap(),
                        row.get(RewardItemCount4 as usize).unwrap(),
                    ],
                    reward_rep_factions: [
                        row.get(RewardRepFaction1 as usize).unwrap(),
                        row.get(RewardRepFaction2 as usize).unwrap(),
                        row.get(RewardRepFaction3 as usize).unwrap(),
                        row.get(RewardRepFaction4 as usize).unwrap(),
                        row.get(RewardRepFaction5 as usize).unwrap(),
                    ],
                    reward_rep_values: [
                        row.get(RewardRepValue1 as usize).unwrap(),
                        row.get(RewardRepValue2 as usize).unwrap(),
                        row.get(RewardRepValue3 as usize).unwrap(),
                        row.get(RewardRepValue4 as usize).unwrap(),
                        row.get(RewardRepValue5 as usize).unwrap(),
                    ],
                    reward_honorable_kills: row.get(RewardHonorableKills as usize).unwrap(),
                    required_or_reward_money: row.get(RequiredOrRewardMoney as usize).unwrap(),
                    reward_money_max_level: row.get(RewardMoneyMaxLevel as usize).unwrap(),
                    reward_spell: row.get(RewardSpell as usize).unwrap(),
                    reward_spell_cast: row.get(RewardSpellCast as usize).unwrap(),
                    reward_mail_template_id: row.get(RewardMailTemplateId as usize).unwrap(),
                    reward_mail_delay_seconds: row.get(RewardMailDelaySeconds as usize).unwrap(),
                    point_map_id: row.get(PointMapId as usize).unwrap(),
                    point_x: row.get(PointX as usize).unwrap(),
                    point_y: row.get(PointY as usize).unwrap(),
                    point_opt: row.get(PointOpt as usize).unwrap(),
                    details_emote1: row.get(DetailsEmote1 as usize).unwrap(),
                    details_emote2: row.get(DetailsEmote2 as usize).unwrap(),
                    details_emote3: row.get(DetailsEmote3 as usize).unwrap(),
                    details_emote4: row.get(DetailsEmote4 as usize).unwrap(),
                    details_emote_delay1: row.get(DetailsEmoteDelay1 as usize).unwrap(),
                    details_emote_delay2: row.get(DetailsEmoteDelay2 as usize).unwrap(),
                    details_emote_delay3: row.get(DetailsEmoteDelay3 as usize).unwrap(),
                    details_emote_delay4: row.get(DetailsEmoteDelay4 as usize).unwrap(),
                    incomplete_emote: row.get(IncompleteEmote as usize).unwrap(),
                    complete_emote: row.get(CompleteEmote as usize).unwrap(),
                    offer_reward_emotes: [
                        row.get(OfferRewardEmote1 as usize).unwrap(),
                        row.get(OfferRewardEmote2 as usize).unwrap(),
                        row.get(OfferRewardEmote3 as usize).unwrap(),
                        row.get(OfferRewardEmote4 as usize).unwrap(),
                    ],
                    offer_reward_emote_delays: [
                        row.get(OfferRewardEmoteDelay1 as usize).unwrap(),
                        row.get(OfferRewardEmoteDelay2 as usize).unwrap(),
                        row.get(OfferRewardEmoteDelay3 as usize).unwrap(),
                        row.get(OfferRewardEmoteDelay4 as usize).unwrap(),
                    ],
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    // Note: only load creature quest relations for now (GameObjects and AreaTriggers not
    // implemented yet)
    pub fn load_relations(conn: &PooledConnection<SqliteConnectionManager>) -> Vec<QuestRelation> {
        let mut stmt = conn
            .prepare_cached(
                "SELECT COUNT(actor_entry) FROM quest_relations WHERE actor_type = :actor_type",
            )
            .unwrap();
        let mut count = stmt
            .query_map(
                named_params! { ":actor_type": QuestActorType::Creature as u8 },
                |row| row.get::<usize, u64>(0),
            )
            .unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare_cached("SELECT actor_type, actor_entry, quest_id, role FROM quest_relations")
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(QuestRelation {
                    actor_type: row
                        .get::<&str, u8>("actor_type")
                        .map(|at| QuestActorType::n(at).unwrap())
                        .unwrap(),
                    actor_entry: row.get("actor_entry").unwrap(),
                    quest_id: row.get("quest_id").unwrap(),
                    role: row
                        .get::<&str, u8>("role")
                        .map(|at| QuestActorRole::n(at).unwrap())
                        .unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}

#[allow(dead_code)]
enum QuestTemplateColumnIndex {
    Entry,
    Method,
    ZoneOrSort,
    MinLevel,
    Level,
    Type,
    RequiredClasses,
    RequiredRaces,
    RequiredSkill,
    RequiredSkillValue,
    RepObjectiveFaction,
    RepObjectiveValue,
    RequiredMinRepFaction,
    RequiredMinRepValue,
    RequiredMaxRepFaction,
    RequiredMaxRepValue,
    SuggestedPlayers,
    TimeLimit,
    Flags,
    SpecialFlags,
    CharacterTitle,
    PreviousQuestId,
    NextQuestId,
    ExclusiveGroup,
    NextQuestInChain,
    SourceItemId,
    SourceItemCount,
    SourceSpell,
    Title,
    Details,
    Objectives,
    OfferRewardText,
    RequestItemsText,
    EndText,
    ObjectiveText1,
    ObjectiveText2,
    ObjectiveText3,
    ObjectiveText4,
    RequiredItemId1,
    RequiredItemId2,
    RequiredItemId3,
    RequiredItemId4,
    RequiredItemCount1,
    RequiredItemCount2,
    RequiredItemCount3,
    RequiredItemCount4,
    RequiredSourceItemId1,
    RequiredSourceItemId2,
    RequiredSourceItemId3,
    RequiredSourceItemId4,
    RequiredSourceItemCount1,
    RequiredSourceItemCount2,
    RequiredSourceItemCount3,
    RequiredSourceItemCount4,
    RequiredEntityId1,
    RequiredEntityId2,
    RequiredEntityId3,
    RequiredEntityId4,
    RequiredEntityCount1,
    RequiredEntityCount2,
    RequiredEntityCount3,
    RequiredEntityCount4,
    RequiredSpellCast1,
    RequiredSpellCast2,
    RequiredSpellCast3,
    RequiredSpellCast4,
    RewardChoiceItemId1,
    RewardChoiceItemId2,
    RewardChoiceItemId3,
    RewardChoiceItemId4,
    RewardChoiceItemId5,
    RewardChoiceItemId6,
    RewardChoiceItemCount1,
    RewardChoiceItemCount2,
    RewardChoiceItemCount3,
    RewardChoiceItemCount4,
    RewardChoiceItemCount5,
    RewardChoiceItemCount6,
    RewardItemId1,
    RewardItemId2,
    RewardItemId3,
    RewardItemId4,
    RewardItemCount1,
    RewardItemCount2,
    RewardItemCount3,
    RewardItemCount4,
    RewardRepFaction1,
    RewardRepFaction2,
    RewardRepFaction3,
    RewardRepFaction4,
    RewardRepFaction5,
    RewardRepValue1,
    RewardRepValue2,
    RewardRepValue3,
    RewardRepValue4,
    RewardRepValue5,
    RewardHonorableKills,
    RequiredOrRewardMoney,
    RewardMoneyMaxLevel,
    RewardSpell,
    RewardSpellCast,
    RewardMailTemplateId,
    RewardMailDelaySeconds,
    PointMapId,
    PointX,
    PointY,
    PointOpt,
    DetailsEmote1,
    DetailsEmote2,
    DetailsEmote3,
    DetailsEmote4,
    DetailsEmoteDelay1,
    DetailsEmoteDelay2,
    DetailsEmoteDelay3,
    DetailsEmoteDelay4,
    IncompleteEmote,
    CompleteEmote,
    OfferRewardEmote1,
    OfferRewardEmote2,
    OfferRewardEmote3,
    OfferRewardEmote4,
    OfferRewardEmoteDelay1,
    OfferRewardEmoteDelay2,
    OfferRewardEmoteDelay3,
    OfferRewardEmoteDelay4,
}
