use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;

use crate::wowhead::models::{WowheadEntityType, WowheadLootItem, WowheadLootTable};

pub struct WowheadCacheRepository {}

impl WowheadCacheRepository {
    pub fn save(conn: &PooledConnection<SqliteConnectionManager>, loot_table: &WowheadLootTable) {
        let mut stmt = conn.prepare_cached("
            INSERT INTO loot_items(entity_type, entity_id, item_id, icon_url, name, loot_percent_chance, min_count, max_count) VALUES
            (:entity_type, :entity_id, :item_id, :icon_url, :name, :loot_percent_chance, :min_count, :max_count)
        ").unwrap();

        for item in loot_table.items.iter() {
            stmt.execute(named_params! {
                ":entity_type": loot_table.entity_type.to_string(),
                ":entity_id": loot_table.id,
                ":item_id": item.id,
                ":icon_url": item.icon_url,
                ":name": item.name,
                ":loot_percent_chance": item.loot_percent_chance,
                ":min_count": item.min_count,
                ":max_count": item.max_count,
            })
            .unwrap();
        }
    }

    pub fn get(
        conn: &PooledConnection<SqliteConnectionManager>,
        entity_type: WowheadEntityType,
        id: u32,
    ) -> Option<WowheadLootTable> {
        let mut stmt = conn
            .prepare_cached(
                "
            SELECT item_id, icon_url, name, ROUND(loot_percent_chance, 2), min_count, max_count
            FROM loot_items
            WHERE entity_type = :entity_type AND entity_id = :entity_id
        ",
            )
            .unwrap();

        let result = stmt
            .query_map(
                named_params! {
                    ":entity_type": entity_type.to_string(),
                    ":entity_id": id,
                },
                |row| {
                    let id: u32 = row.get(0).unwrap();
                    let icon_url: String = row.get(1).unwrap();
                    let name: String = row.get(2).unwrap();
                    let loot_percent_chance: f32 = row.get(3).unwrap();
                    let min_count: Option<u32> = row.get(4).unwrap();
                    let max_count: Option<u32> = row.get(5).unwrap();

                    Ok(WowheadLootItem {
                        id,
                        icon_url,
                        name,
                        loot_percent_chance,
                        min_count,
                        max_count,
                    })
                },
            )
            .unwrap();

        let items: Vec<WowheadLootItem> = result.flatten().collect();

        if items.is_empty() {
            None
        } else {
            Some(WowheadLootTable {
                entity_type,
                id,
                items,
            })
        }
    }

    pub fn ignore_entity(
        conn: &PooledConnection<SqliteConnectionManager>,
        entity_type: WowheadEntityType,
        entity_id: u32,
        reason: String,
    ) {
        let mut stmt = conn
            .prepare_cached(
                "
            INSERT INTO ignored_loot_tables (entity_type, entity_id, ignore_reason)
            VALUES (:entity_type, :entity_id, :reason)",
            )
            .unwrap();

        stmt.execute(named_params! {
            ":entity_type": entity_type.to_string(),
            ":entity_id": entity_id,
            ":reason": reason,
        })
        .unwrap();
    }

    pub fn is_entity_ignored(
        conn: &PooledConnection<SqliteConnectionManager>,
        entity_type: WowheadEntityType,
        entity_id: u32,
    ) -> bool {
        let mut stmt = conn
            .prepare_cached(
                "
            SELECT COUNT(*) FROM ignored_loot_tables
            WHERE entity_type = :entity_type AND entity_id = :entity_id
        ",
            )
            .unwrap();

        let mut result = stmt
            .query_map(
                named_params! {
                    ":entity_type": entity_type.to_string(),
                    ":entity_id": entity_id
                },
                |row| Ok(row.get::<usize, u32>(0)),
            )
            .unwrap();

        result.next().unwrap().unwrap().unwrap() == 1 // lol, FIXME at some point
    }
}
