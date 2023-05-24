use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{
    datastore::data_types::CreatureTemplate, shared::constants::MAX_CREATURE_TEMPLATE_MODELID,
};

pub struct CreatureRepository;

impl CreatureRepository {
    pub fn load_templates(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<CreatureTemplate> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(entry) FROM creature_templates")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT entry, name, sub_name, icon_name, min_level, max_level, model_id1, model_id2, model_id3, model_id4, scale, family, type_id, racial_leader, type_flags, speed_walk, speed_run, rank, health_multiplier, power_multiplier, min_level_health, max_level_health, min_level_mana, max_level_mana, pet_spell_data_id FROM creature_templates ORDER BY entry").unwrap();

        let result = stmt
            .query_map([], |row| {
                let model_ids: Vec<u32> = (1..MAX_CREATURE_TEMPLATE_MODELID)
                    .into_iter()
                    .map(|index| row.get(format!("model_id{}", index).as_str()).unwrap())
                    .collect();

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(CreatureTemplate {
                    entry: row.get("entry").unwrap(),
                    name: row.get("name").unwrap(),
                    sub_name: row.get("sub_name").unwrap(),
                    icon_name: row.get("icon_name").unwrap(),
                    min_level: row.get("min_level").unwrap(),
                    max_level: row.get("max_level").unwrap(),
                    min_level_health: row.get("min_level_health").unwrap(),
                    max_level_health: row.get("max_level_health").unwrap(),
                    min_level_mana: row.get("min_level_mana").unwrap(),
                    max_level_mana: row.get("max_level_mana").unwrap(),
                    model_ids,
                    scale: row.get("scale").unwrap(),
                    speed_walk: row.get("speed_walk").unwrap(),
                    speed_run: row.get("speed_run").unwrap(),
                    family: row.get("family").unwrap(),
                    type_id: row.get("type_id").unwrap(),
                    type_flags: row.get("type_flags").unwrap(),
                    rank: row.get("rank").unwrap(),
                    racial_leader: row.get("racial_leader").unwrap(),
                    health_multiplier: row.get("health_multiplier").unwrap(),
                    power_multiplier: row.get("power_multiplier").unwrap(),
                    pet_spell_data_id: row.get("pet_spell_data_id").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}
