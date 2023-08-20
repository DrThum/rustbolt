use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::{FromSql, FromSqlError};

use crate::{
    datastore::data_types::{PlayerCreateActionButton, PlayerCreatePosition, PlayerCreateSpell},
    shared::constants::{CharacterClass, CharacterRace},
};

pub struct PlayerStaticDataRepository;

impl PlayerStaticDataRepository {
    pub fn load_positions(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<PlayerCreatePosition> {
        let mut stmt = conn
            .prepare("SELECT race, class, map, zone, x, y, z, o FROM player_create_positions")
            .unwrap();

        let count = 52; // This shouldn't change in TBC
        let bar = ProgressBar::new(count);

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(PlayerCreatePosition {
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    map: row.get("map").unwrap(),
                    zone: row.get("zone").unwrap(),
                    x: row.get("x").unwrap(),
                    y: row.get("y").unwrap(),
                    z: row.get("z").unwrap(),
                    o: row.get("o").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_spells(conn: &PooledConnection<SqliteConnectionManager>) -> Vec<PlayerCreateSpell> {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM player_create_spells")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare("SELECT race, class, spell_id FROM player_create_spells")
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(PlayerCreateSpell {
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    spell_id: row.get("spell_id").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_action_buttons(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<PlayerCreateActionButton> {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM player_create_action_buttons")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare("SELECT race, class, position, action_type, action_value FROM player_create_action_buttons").unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(PlayerCreateActionButton {
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    position: row.get("position").unwrap(),
                    action_type: row.get("action_type").unwrap(),
                    action_value: row.get("action_value").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_base_health_mana_per_level(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<PlayerBaseHealthManaPerLevelDbRecord> {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM player_base_health_mana_per_level")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare("SELECT class, level, base_health, base_mana FROM player_base_health_mana_per_level").unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(PlayerBaseHealthManaPerLevelDbRecord {
                    class: row.get("class").unwrap(),
                    level: row.get("level").unwrap(),
                    base_health: row.get("base_health").unwrap(),
                    base_mana: row.get("base_mana").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }

    pub fn load_base_attributes_per_level(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<PlayerBaseAttributesPerLevelDbRecord> {
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM player_base_attributes_per_level")
            .unwrap();
        let mut count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare("SELECT race, class, level, strength, agility, stamina, intellect, spirit FROM player_base_attributes_per_level").unwrap();

        let result = stmt
            .query_map([], |row| {
                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(PlayerBaseAttributesPerLevelDbRecord {
                    race: row.get("race").unwrap(),
                    class: row.get("class").unwrap(),
                    level: row.get("level").unwrap(),
                    strength: row.get("strength").unwrap(),
                    agility: row.get("agility").unwrap(),
                    stamina: row.get("stamina").unwrap(),
                    intellect: row.get("intellect").unwrap(),
                    spirit: row.get("spirit").unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}

pub struct PlayerBaseHealthManaPerLevelDbRecord {
    pub class: CharacterClass,
    pub level: u32,
    pub base_health: u32,
    pub base_mana: u32,
}

impl PlayerBaseHealthManaPerLevelDbRecord {
    pub fn key(&self) -> u32 {
        ((self.class as u32) << 8) | self.level
    }
}

impl FromSql for CharacterClass {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        CharacterClass::n(value).map_or(
            Err(FromSqlError::Other("invalid character class".into())),
            Ok,
        )
    }
}

pub struct PlayerBaseAttributesPerLevelDbRecord {
    pub race: CharacterRace,
    pub class: CharacterClass,
    pub level: u32,
    pub strength: u32,
    pub agility: u32,
    pub stamina: u32,
    pub intellect: u32,
    pub spirit: u32,
}

impl PlayerBaseAttributesPerLevelDbRecord {
    pub fn key(&self) -> u32 {
        ((self.race as u32) << 16) | ((self.class as u32) << 8) | self.level
    }
}

impl FromSql for CharacterRace {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        CharacterRace::n(value).map_or(
            Err(FromSqlError::Other("invalid character race".into())),
            Ok,
        )
    }
}
