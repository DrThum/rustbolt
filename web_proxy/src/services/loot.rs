use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::repositories::{game_object, loot};

pub fn get_next_loot_table_id(
    conn: &PooledConnection<SqliteConnectionManager>,
    entity_type: &String,
    entity_id: u32,
) -> u32 {
    match entity_type.as_str() {
        "creature" => {
            // Try to give this creature its id as loot table id
            let is_id_available_from_go = game_object::is_loot_id_available(conn, entity_id);
            let is_id_available_in_tables = loot::is_loot_id_available(conn, entity_id);
            if is_id_available_from_go && is_id_available_in_tables {
                entity_id
            } else {
                loot::get_next_loot_table_id(conn)
            }
        }
        "gameobject" => {
            let maybe_template_loot_id = game_object::get_loot_table_id(conn, entity_id);
            if let Some(loot_table_id) = maybe_template_loot_id {
                loot_table_id
            } else {
                loot::get_next_loot_table_id(conn)
            }
        }
        _ => panic!("get_next_loot_table_id: unexpected entity_type {entity_type}"),
    }
}
