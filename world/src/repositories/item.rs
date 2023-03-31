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

    pub fn load_player_inventory(
        conn: &PooledConnection<SqliteConnectionManager>,
        player_guid: u32,
    ) -> Vec<ItemDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT items.guid, items.entry, character_inventory.character_guid, character_inventory.slot FROM items JOIN character_inventory ON character_inventory.item_guid = items.guid WHERE character_inventory.character_guid = :player_guid").unwrap();

        let result = stmt
            .query_map(named_params! { ":player_guid": player_guid }, |row| {
                let guid: u32 = row.get(0).unwrap();
                let item_entry: u32 = row.get(1).unwrap();
                let owner_guid: u64 = row.get(2).unwrap();
                let slot: u32 = row.get(3).unwrap();

                Ok(ItemDbRecord {
                    guid,
                    entry: item_entry,
                    owner_guid: Some(owner_guid),
                    slot
                })
            })
            .unwrap();

        result.filter(|res| res.is_ok()).map(|res| res.unwrap()).into_iter().collect()
    }
}

pub struct ItemDbRecord {
    pub guid: u32,
    pub entry: u32,
    pub owner_guid: Option<u64>,
    pub slot: u32,
}
