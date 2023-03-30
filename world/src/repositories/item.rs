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
}
