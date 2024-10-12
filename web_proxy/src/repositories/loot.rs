use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, OptionalExtension};

pub fn get_next_loot_table_id(conn: &PooledConnection<SqliteConnectionManager>) -> u32 {
    let mut stmt = conn
        .prepare_cached(
            "SELECT (ID+1) FROM loot_tables AS t1
             LEFT JOIN loot_tables as t2
             ON t1.ID+1 = t2.ID
             WHERE t2.ID IS NULL
             LIMIT 1",
        )
        .unwrap();

    stmt
        .query_row([], |row| Ok(row.get::<usize, u32>(0).unwrap()))
        .unwrap()
}

pub fn is_loot_id_available(
    conn: &PooledConnection<SqliteConnectionManager>,
    desired_id: u32,
) -> bool {
    let mut stmt = conn
        .prepare_cached("SELECT 1 FROM loot_tables WHERE id = :desired_id")
        .unwrap();

    let result = stmt
        .query_row(named_params! { ":desired_id": desired_id }, |_| Ok(()))
        .optional();

    // If we found no result, then the id is available
    result.ok().flatten().is_none()
}
