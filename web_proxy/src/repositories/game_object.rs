use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, OptionalExtension};

pub fn get_loot_table_id(
    conn: &PooledConnection<SqliteConnectionManager>,
    game_object_id: u32,
) -> Option<u32> {
    // Type 3 = CHEST, type 25 = FISHINGHOLE
    let mut stmt = conn
        .prepare_cached(
            "SELECT data1 FROM game_object_templates WHERE entry = :entry AND type IN (3, 25)",
        )
        .unwrap();

    let mut result = stmt
        .query_map(named_params! { ":entry": game_object_id }, |row| {
            Ok(row.get::<usize, u32>(0).unwrap())
        })
        .unwrap();

    if let Ok(id) = result.next().unwrap() {
        Some(id)
    } else {
        None
    }
}

pub fn is_loot_id_available(
    conn: &PooledConnection<SqliteConnectionManager>,
    desired_id: u32,
) -> bool {
    // Type 3 = CHEST, type 25 = FISHINGHOLE
    let mut stmt = conn
        .prepare_cached(
            "SELECT 1 FROM game_object_templates WHERE data1 = :desired_id AND type IN (3, 25)",
        )
        .unwrap();

    let result = stmt
        .query_row(named_params! { ":desired_id": desired_id }, |_| Ok(()))
        .optional();

    // If we found no result, then the id is available
    result.ok().flatten().is_none()
}
