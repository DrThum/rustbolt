use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::datastore::data_types::{PlayerCreatePosition, PlayerCreateSpell};

pub struct PlayerCreationRepository;

impl PlayerCreationRepository {
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
            .prepare_cached("SELECT COUNT(*) FROM player_create_spells")
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
}
