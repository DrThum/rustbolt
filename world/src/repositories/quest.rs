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

        let mut stmt = conn.prepare_cached("SELECT entry, method, zone_or_sort, min_level, level, type, required_classes, required_races, required_skill, required_skill_value, rep_objective_faction, rep_objective_value, required_min_rep_faction, required_min_rep_value, required_max_rep_faction, required_max_rep_value, suggested_players, time_limit, flags, special_flags, character_title, previous_quest_id, next_quest_id, exclusive_group, next_quest_in_chain, source_item_id, source_item_count, source_spell, title, details, objectives, offer_reward_text, request_items_text, end_text, objective_text1, objective_text2, objective_text3, objective_text4, required_item_id1, required_item_id2, required_item_id3, required_item_id4, required_item_count1, required_item_count2, required_item_count3, required_item_count4, required_source_item_id1, required_source_item_id2, required_source_item_id3, required_source_item_id4, required_source_item_count1, required_source_item_count2, required_source_item_count3, required_source_item_count4, required_entity_id1, required_entity_id2, required_entity_id3, required_entity_id4, required_entity_count1, required_entity_count2, required_entity_count3, required_entity_count4, required_spell_cast1, required_spell_cast2, required_spell_cast3, required_spell_cast4, reward_choice_item_id1, reward_choice_item_id2, reward_choice_item_id3, reward_choice_item_id4, reward_choice_item_id5, reward_choice_item_id6, reward_choice_item_count1, reward_choice_item_count2, reward_choice_item_count3, reward_choice_item_count4, reward_choice_item_count5, reward_choice_item_count6, reward_item_id1, reward_item_id2, reward_item_id3, reward_item_id4, reward_item_count1, reward_item_count2, reward_item_count3, reward_item_count4, reward_rep_faction1, reward_rep_faction2, reward_rep_faction3, reward_rep_faction4, reward_rep_faction5, reward_rep_value1, reward_rep_value2, reward_rep_value3, reward_rep_value4, reward_rep_value5, reward_honorable_kills, required_or_reward_money, reward_money_max_level, reward_spell, reward_spell_cast, reward_mail_template_id, reward_mail_delay_seconds, point_map_id, point_x, point_y, point_opt, details_emote1, details_emote2, details_emote3, details_emote4, details_emote_delay1, details_emote_delay2, details_emote_delay3, details_emote_delay4, incomplete_emote, complete_emote, offer_reward_emote1, offer_reward_emote2, offer_reward_emote3, offer_reward_emote4, offer_reward_emote_delay1, offer_reward_emote_delay2, offer_reward_emote_delay3, offer_reward_emote_delay4 FROM quest_templates ORDER BY entry").unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(QuestTemplate {
                    entry: row.get("entry").unwrap(),
                    method: row.get("method").unwrap(),
                    zone_or_sort: row.get("zone_or_sort").unwrap(),
                    min_level: row.get("min_level").unwrap(),
                    level: row.get("level").unwrap(),
                    type_: row.get("type").unwrap(),
                    required_classes: unsafe {
                        row.get::<&str, u32>("required_classes")
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    required_races: unsafe {
                        row.get::<&str, u32>("required_races")
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    required_skill: row.get("required_skill").unwrap(),
                    required_skill_value: row.get("required_skill_value").unwrap(),
                    rep_objective_faction: row.get("rep_objective_faction").unwrap(),
                    rep_objective_value: row.get("rep_objective_value").unwrap(),
                    required_min_rep_faction: row.get("required_min_rep_faction").unwrap(),
                    required_min_rep_value: row.get("required_min_rep_value").unwrap(),
                    required_max_rep_faction: row.get("required_max_rep_faction").unwrap(),
                    required_max_rep_value: row.get("required_max_rep_value").unwrap(),
                    suggested_players: row.get("suggested_players").unwrap(),
                    time_limit: row.get("time_limit").unwrap(),
                    flags: unsafe {
                        row.get::<&str, u32>("flags")
                            .map(|flags| BitFlags::from_bits_unchecked(flags))
                            .unwrap()
                    },
                    special_flags: row.get("special_flags").unwrap(),
                    character_title: row.get("character_title").unwrap(),
                    previous_quest_id: row.get("previous_quest_id").unwrap(),
                    next_quest_id: row.get("next_quest_id").unwrap(),
                    exclusive_group: row.get("exclusive_group").unwrap(),
                    next_quest_in_chain: row.get("next_quest_in_chain").unwrap(),
                    source_item_id: row.get("source_item_id").unwrap(),
                    source_item_count: row.get("source_item_count").unwrap(),
                    source_spell: row.get("source_spell").unwrap(),
                    title: row.get("title").unwrap(),
                    details: row.get("details").unwrap(),
                    objectives: row.get("objectives").unwrap(),
                    offer_reward_text: row.get("offer_reward_text").unwrap(),
                    request_items_text: row.get("request_items_text").unwrap(),
                    end_text: row.get("end_text").unwrap(),
                    objective_text1: row.get("objective_text1").unwrap(),
                    objective_text2: row.get("objective_text2").unwrap(),
                    objective_text3: row.get("objective_text3").unwrap(),
                    objective_text4: row.get("objective_text4").unwrap(),
                    required_item_id1: row.get("required_item_id1").unwrap(),
                    required_item_id2: row.get("required_item_id2").unwrap(),
                    required_item_id3: row.get("required_item_id3").unwrap(),
                    required_item_id4: row.get("required_item_id4").unwrap(),
                    required_item_count1: row.get("required_item_count1").unwrap(),
                    required_item_count2: row.get("required_item_count2").unwrap(),
                    required_item_count3: row.get("required_item_count3").unwrap(),
                    required_item_count4: row.get("required_item_count4").unwrap(),
                    required_source_item_id1: row.get("required_source_item_id1").unwrap(),
                    required_source_item_id2: row.get("required_source_item_id2").unwrap(),
                    required_source_item_id3: row.get("required_source_item_id3").unwrap(),
                    required_source_item_id4: row.get("required_source_item_id4").unwrap(),
                    required_source_item_count1: row.get("required_source_item_count1").unwrap(),
                    required_source_item_count2: row.get("required_source_item_count2").unwrap(),
                    required_source_item_count3: row.get("required_source_item_count3").unwrap(),
                    required_source_item_count4: row.get("required_source_item_count4").unwrap(),
                    required_entity_id1: row.get("required_entity_id1").unwrap(),
                    required_entity_id2: row.get("required_entity_id2").unwrap(),
                    required_entity_id3: row.get("required_entity_id3").unwrap(),
                    required_entity_id4: row.get("required_entity_id4").unwrap(),
                    required_entity_count1: row.get("required_entity_count1").unwrap(),
                    required_entity_count2: row.get("required_entity_count2").unwrap(),
                    required_entity_count3: row.get("required_entity_count3").unwrap(),
                    required_entity_count4: row.get("required_entity_count4").unwrap(),
                    required_spell_cast1: row.get("required_spell_cast1").unwrap(),
                    required_spell_cast2: row.get("required_spell_cast2").unwrap(),
                    required_spell_cast3: row.get("required_spell_cast3").unwrap(),
                    required_spell_cast4: row.get("required_spell_cast4").unwrap(),
                    reward_choice_item_id1: row.get("reward_choice_item_id1").unwrap(),
                    reward_choice_item_id2: row.get("reward_choice_item_id2").unwrap(),
                    reward_choice_item_id3: row.get("reward_choice_item_id3").unwrap(),
                    reward_choice_item_id4: row.get("reward_choice_item_id4").unwrap(),
                    reward_choice_item_id5: row.get("reward_choice_item_id5").unwrap(),
                    reward_choice_item_id6: row.get("reward_choice_item_id6").unwrap(),
                    reward_choice_item_count1: row.get("reward_choice_item_count1").unwrap(),
                    reward_choice_item_count2: row.get("reward_choice_item_count2").unwrap(),
                    reward_choice_item_count3: row.get("reward_choice_item_count3").unwrap(),
                    reward_choice_item_count4: row.get("reward_choice_item_count4").unwrap(),
                    reward_choice_item_count5: row.get("reward_choice_item_count5").unwrap(),
                    reward_choice_item_count6: row.get("reward_choice_item_count6").unwrap(),
                    reward_item_id1: row.get("reward_item_id1").unwrap(),
                    reward_item_id2: row.get("reward_item_id2").unwrap(),
                    reward_item_id3: row.get("reward_item_id3").unwrap(),
                    reward_item_id4: row.get("reward_item_id4").unwrap(),
                    reward_item_count1: row.get("reward_item_count1").unwrap(),
                    reward_item_count2: row.get("reward_item_count2").unwrap(),
                    reward_item_count3: row.get("reward_item_count3").unwrap(),
                    reward_item_count4: row.get("reward_item_count4").unwrap(),
                    reward_rep_faction1: row.get("reward_rep_faction1").unwrap(),
                    reward_rep_faction2: row.get("reward_rep_faction2").unwrap(),
                    reward_rep_faction3: row.get("reward_rep_faction3").unwrap(),
                    reward_rep_faction4: row.get("reward_rep_faction4").unwrap(),
                    reward_rep_faction5: row.get("reward_rep_faction5").unwrap(),
                    reward_rep_value1: row.get("reward_rep_value1").unwrap(),
                    reward_rep_value2: row.get("reward_rep_value2").unwrap(),
                    reward_rep_value3: row.get("reward_rep_value3").unwrap(),
                    reward_rep_value4: row.get("reward_rep_value4").unwrap(),
                    reward_rep_value5: row.get("reward_rep_value5").unwrap(),
                    reward_honorable_kills: row.get("reward_honorable_kills").unwrap(),
                    required_or_reward_money: row.get("required_or_reward_money").unwrap(),
                    reward_money_max_level: row.get("reward_money_max_level").unwrap(),
                    reward_spell: row.get("reward_spell").unwrap(),
                    reward_spell_cast: row.get("reward_spell_cast").unwrap(),
                    reward_mail_template_id: row.get("reward_mail_template_id").unwrap(),
                    reward_mail_delay_seconds: row.get("reward_mail_delay_seconds").unwrap(),
                    point_map_id: row.get("point_map_id").unwrap(),
                    point_x: row.get("point_x").unwrap(),
                    point_y: row.get("point_y").unwrap(),
                    point_opt: row.get("point_opt").unwrap(),
                    details_emote1: row.get("details_emote1").unwrap(),
                    details_emote2: row.get("details_emote2").unwrap(),
                    details_emote3: row.get("details_emote3").unwrap(),
                    details_emote4: row.get("details_emote4").unwrap(),
                    details_emote_delay1: row.get("details_emote_delay1").unwrap(),
                    details_emote_delay2: row.get("details_emote_delay2").unwrap(),
                    details_emote_delay3: row.get("details_emote_delay3").unwrap(),
                    details_emote_delay4: row.get("details_emote_delay4").unwrap(),
                    incomplete_emote: row.get("incomplete_emote").unwrap(),
                    complete_emote: row.get("complete_emote").unwrap(),
                    offer_reward_emote1: row.get("offer_reward_emote1").unwrap(),
                    offer_reward_emote2: row.get("offer_reward_emote2").unwrap(),
                    offer_reward_emote3: row.get("offer_reward_emote3").unwrap(),
                    offer_reward_emote4: row.get("offer_reward_emote4").unwrap(),
                    offer_reward_emote_delay1: row.get("offer_reward_emote_delay1").unwrap(),
                    offer_reward_emote_delay2: row.get("offer_reward_emote_delay2").unwrap(),
                    offer_reward_emote_delay3: row.get("offer_reward_emote_delay3").unwrap(),
                    offer_reward_emote_delay4: row.get("offer_reward_emote_delay4").unwrap(),
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
                named_params! {":actor_type": QuestActorType::Creature as u8 },
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
