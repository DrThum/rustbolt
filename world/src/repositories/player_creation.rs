use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::datastore::data_types::PlayerCreatePosition;

pub struct PlayerCreationRepository;

impl PlayerCreationRepository {
    pub fn load_positions(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<PlayerCreatePosition> {
        let mut stmt = conn
            .prepare_cached(
                "SELECT race, class, map, zone, x, y, z, o FROM player_create_positions",
            )
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

        result
            .filter(|res| res.is_ok())
            .map(|res| res.unwrap())
            .into_iter()
            .collect()
    }
}
