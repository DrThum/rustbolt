use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::shared::constants::{CharacterClass, MAX_EXPANSION};

pub struct CreatureStaticDataRepository;

impl CreatureStaticDataRepository {
    pub fn load_base_attributes_per_level(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<CreatureBaseAttributesPerLevelDbRecord> {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM creature_base_attributes_per_level")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare("SELECT class, level, health_exp0, health_exp1, mana, damage_exp0, damage_exp1, melee_attack_power, ranged_attack_power, armor FROM creature_base_attributes_per_level").unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(CreatureBaseAttributesPerLevelDbRecord {
                    class: row.get("class").unwrap(),
                    level: row.get("level").unwrap(),
                    health: [
                        row.get("health_exp0").unwrap(),
                        row.get("health_exp1").unwrap(),
                    ],
                    mana: row.get("mana").unwrap(),
                    damage: [
                        row.get("damage_exp0").unwrap(),
                        row.get("damage_exp1").unwrap(),
                    ],
                    melee_attack_power: row.get("melee_attack_power").unwrap(),
                    ranged_attack_power: row.get("ranged_attack_power").unwrap(),
                    armor: row.get("armor").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}

pub struct CreatureBaseAttributesPerLevelDbRecord {
    pub class: CharacterClass,
    pub level: u32,
    pub health: [u32; MAX_EXPANSION],
    pub mana: u32,
    pub damage: [f32; MAX_EXPANSION],
    pub melee_attack_power: f32,
    pub ranged_attack_power: f32,
    pub armor: u32,
}

impl CreatureBaseAttributesPerLevelDbRecord {
    pub fn key(&self) -> u32 {
        ((self.class as u32) << 16) | self.level
    }

    pub fn health(&self, expansion: usize, multiplier: f32) -> u32 {
        ((self.health[expansion] as f32 * multiplier).ceil() as u32).max(1)
    }

    pub fn damage(&self, expansion: usize, multiplier: f32) -> f32 {
        ((self.damage[expansion] as f32 * multiplier).ceil()).max(1.)
    }
}
