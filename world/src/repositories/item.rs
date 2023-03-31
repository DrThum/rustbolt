use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{named_params, Transaction};

pub struct ItemRepository;

impl ItemRepository {
    pub fn create(transaction: &Transaction, entry: u32) -> u64 {
        let mut stmt = transaction
            .prepare_cached("INSERT INTO items(guid, entry) VALUES (NULL, :entry)")
            .unwrap();
        stmt.execute(named_params! {
            ":entry": entry,
        })
        .unwrap();

        transaction.last_insert_rowid() as u64
    }

    pub fn load(
        conn: &PooledConnection<SqliteConnectionManager>,
        guid: u64,
    ) -> Option<ItemDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT items.entry, character_inventory LEFT JOIN character_inventory ON character_inventory.item_guid = items.guid FROM items WHERE items.guid = :guid").unwrap();

        let mut result = stmt
            .query_map(named_params! { ":guid": guid }, |row| {
                let item_entry: u32 = row.get(0).unwrap();
                let owned_guid: Option<u64> = row.get(1).unwrap();

                Ok((item_entry, owned_guid))
            })
            .unwrap();

        if let Ok(item) = result.next().unwrap() {
            Some(ItemDbRecord {
                entry: item.0,
                owned_guid: item.1,
            })
        } else {
            None
        }
    }
}

pub struct ItemDbRecord {
    pub entry: u32,
    pub owned_guid: Option<u64>,
}
