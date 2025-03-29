use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Row};

use crate::{
    datastore::data_types::{
        GossipMenuDbRecord, GossipMenuOption, NpcText, NpcTextDbRecord, NpcTextEmote,
    },
    shared::constants::NPC_TEXT_TEXT_COUNT,
};

pub struct GossipRepository;

impl GossipRepository {
    pub fn load_npc_text(conn: &PooledConnection<SqliteConnectionManager>) -> Vec<NpcTextDbRecord> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(id) FROM npc_texts")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT id, text0_male, text0_female, text0_language, text0_probability, text0_emote0_delay, text0_emote0, text0_emote1_delay, text0_emote1, text0_emote2_delay, text0_emote2, text1_male, text1_female, text1_language, text1_probability, text1_emote0_delay, text1_emote0, text1_emote1_delay, text1_emote1, text1_emote2_delay, text1_emote2, text2_male, text2_female, text2_language, text2_probability, text2_emote0_delay, text2_emote0, text2_emote1_delay, text2_emote1, text2_emote2_delay, text2_emote2, text3_male, text3_female, text3_language, text3_probability, text3_emote0_delay, text3_emote0, text3_emote1_delay, text3_emote1, text3_emote2_delay, text3_emote2, text4_male, text4_female, text4_language, text4_probability, text4_emote0_delay, text4_emote0, text4_emote1_delay, text4_emote1, text4_emote2_delay, text4_emote2, text5_male, text5_female, text5_language, text5_probability, text5_emote0_delay, text5_emote0, text5_emote1_delay, text5_emote1, text5_emote2_delay, text5_emote2, text6_male, text6_female, text6_language, text6_probability, text6_emote0_delay, text6_emote0, text6_emote1_delay, text6_emote1, text6_emote2_delay, text6_emote2, text7_male, text7_female, text7_language, text7_probability, text7_emote0_delay, text7_emote0, text7_emote1_delay, text7_emote1, text7_emote2_delay, text7_emote2 FROM npc_texts").unwrap();

        fn transform_text(row: &Row, index: usize) -> NpcText {
            fn transform_emote(row: &Row, text_index: usize, emote_index: usize) -> NpcTextEmote {
                let base_index = NpcTextsColumnIndex::Text0Emote0Delay as usize
                    + (text_index * 10)
                    + (emote_index * 2);
                NpcTextEmote {
                    delay: row.get(base_index).unwrap_or_default(),
                    emote: row.get(base_index + 1).unwrap_or_default(),
                }
            }

            let emotes = [
                transform_emote(row, index, 0),
                transform_emote(row, index, 1),
                transform_emote(row, index, 2),
            ];

            let base_index = NpcTextsColumnIndex::Text0Male as usize + (index * 10);
            NpcText {
                text_male: row
                    .get::<usize, Option<String>>(base_index)
                    .unwrap()
                    .filter(|t| !t.is_empty()),
                text_female: row
                    .get::<usize, Option<String>>(base_index + 1)
                    .unwrap()
                    .filter(|t| !t.is_empty()),
                language: row.get(base_index + 2).unwrap(),
                probability: row.get(base_index + 3).unwrap(),
                emotes,
            }
        }

        let result = stmt
            .query_map([], |row| {
                let texts: [NpcText; NPC_TEXT_TEXT_COUNT] = [
                    transform_text(row, 0),
                    transform_text(row, 1),
                    transform_text(row, 2),
                    transform_text(row, 3),
                    transform_text(row, 4),
                    transform_text(row, 5),
                    transform_text(row, 6),
                    transform_text(row, 7),
                ];

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(NpcTextDbRecord {
                    id: row.get(NpcTextsColumnIndex::Id as usize).unwrap(),
                    texts,
                })
            })
            .unwrap();

        result.flatten().collect()
    }

    pub fn load_gossip_menus(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<GossipMenuDbRecord> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(id) FROM gossip_menus")
            .unwrap();

        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        fn fetch_options(
            conn: &PooledConnection<SqliteConnectionManager>,
            menu_id: u32,
        ) -> Vec<GossipMenuOption> {
            let mut stmt = conn.prepare_cached("SELECT menu_id, id, option_icon, option_text, option_id, npc_option_npcflag, action_menu_id, action_poi_id, box_coded, box_money, box_text FROM gossip_menu_options WHERE menu_id = :menu_id").unwrap();

            let result = stmt
                .query_map(named_params! { ":menu_id": menu_id }, |row| {
                    Ok(GossipMenuOption {
                        id: row.get("id").unwrap(),
                        icon: row.get("option_icon").unwrap(),
                        text: row.get("option_text").unwrap(),
                        option_type: row.get("option_id").unwrap(),
                        npc_option_npcflag: row.get("npc_option_npcflag").unwrap(),
                        action_menu_id: row.get("action_menu_id").unwrap(),
                        action_poi_id: row.get("action_poi_id").unwrap(),
                        box_coded: row.get::<&str, u32>("box_coded").map(|c| c == 1).unwrap(),
                        box_money: row.get("box_money").unwrap(),
                        box_text: row.get("box_text").unwrap(),
                    })
                })
                .unwrap();

            result.flatten().collect()
        }

        let mut stmt = conn
            .prepare_cached("SELECT id, text_id FROM gossip_menus")
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                let id: u32 = row.get("id").unwrap();
                Ok(GossipMenuDbRecord {
                    id,
                    text_id: row.get("text_id").unwrap(),
                    options: fetch_options(conn, id),
                })
            })
            .unwrap();

        result.flatten().collect()
    }
}

#[allow(dead_code)]
enum NpcTextsColumnIndex {
    Id,
    Text0Male,
    Text0Female,
    Text0Language,
    Text0Probability,
    Text0Emote0Delay,
    Text0Emote0,
    Text0Emote1Delay,
    Text0Emote1,
    Text0Emote2Delay,
    Text0Emote2,
    Text1Male,
    Text1Female,
    Text1Language,
    Text1Probability,
    Text1Emote0Delay,
    Text1Emote0,
    Text1Emote1Delay,
    Text1Emote1,
    Text1Emote2Delay,
    Text1Emote2,
    Text2Male,
    Text2Female,
    Text2Language,
    Text2Probability,
    Text2Emote0Delay,
    Text2Emote0,
    Text2Emote1Delay,
    Text2Emote1,
    Text2Emote2Delay,
    Text2Emote2,
    Text3Male,
    Text3Female,
    Text3Language,
    Text3Probability,
    Text3Emote0Delay,
    Text3Emote0,
    Text3Emote1Delay,
    Text3Emote1,
    Text3Emote2Delay,
    Text3Emote2,
    Text4Male,
    Text4Female,
    Text4Language,
    Text4Probability,
    Text4Emote0Delay,
    Text4Emote0,
    Text4Emote1Delay,
    Text4Emote1,
    Text4Emote2Delay,
    Text4Emote2,
    Text5Male,
    Text5Female,
    Text5Language,
    Text5Probability,
    Text5Emote0Delay,
    Text5Emote0,
    Text5Emote1Delay,
    Text5Emote1,
    Text5Emote2Delay,
    Text5Emote2,
    Text6Male,
    Text6Female,
    Text6Language,
    Text6Probability,
    Text6Emote0Delay,
    Text6Emote0,
    Text6Emote1Delay,
    Text6Emote1,
    Text6Emote2Delay,
    Text6Emote2,
    Text7Male,
    Text7Female,
    Text7Language,
    Text7Probability,
    Text7Emote0Delay,
    Text7Emote0,
    Text7Emote1Delay,
    Text7Emote1,
    Text7Emote2Delay,
    Text7Emote2,
}
