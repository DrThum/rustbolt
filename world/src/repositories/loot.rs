use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;

use crate::{
    datastore::data_types::{CreatureLootGroup, CreatureLootItem, CreatureLootTable},
    game::value_range::ValueRange,
    repositories::creature::CreatureLootGroupColumnIndex,
};

use super::creature::CreatureLootItemColumnIndex;

pub struct LootRepository;

impl LootRepository {
    pub fn load_creature_loot_tables(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<CreatureLootTable> {
        let mut stmt = conn
            .prepare_cached("SELECT COUNT(id) FROM creature_loot_tables")
            .unwrap();
        let mut group_count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = group_count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare_cached("SELECT id FROM creature_loot_tables")
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                let loot_table_id: u32 = row.get(0).unwrap();

                let mut stmt_groups = conn
                    .prepare_cached("SELECT group_id, chance, num_rolls_min, num_rolls_max, condition_id FROM creature_loot_groups WHERE loot_table_id = :loot_table_id")
                    .unwrap();

                let result_groups = stmt_groups.query_map(named_params! { ":loot_table_id": loot_table_id }, |row_group| {
                    let group_id: u32 = row.get(CreatureLootGroupColumnIndex::GroupId as usize).unwrap();

                    let mut stmt_items = conn
                        .prepare_cached("SELECT item_id, condition_id FROM creature_loot_items WHERE loot_table_id = :loot_table_id AND group_id = :group_id")
                        .unwrap();

                    let result_items = stmt_items.query_map(named_params! { ":loot_table_id": loot_table_id, ":group_id": group_id }, |row_item| {
                        Ok(CreatureLootItem {
                            item_id: row_item.get(CreatureLootItemColumnIndex::ItemId as usize).unwrap(),
                            chance: row_item.get(CreatureLootItemColumnIndex::Chance as usize).unwrap(),
                            condition_id: row_item.get(CreatureLootItemColumnIndex::ConditionId as usize).unwrap(),
                        })
                    }).unwrap().filter_map(|res| res.ok()).into_iter().collect();

                    bar.inc(1);
                    if bar.position() == count {
                        bar.finish();
                    }

                    Ok(CreatureLootGroup {
                        chance: row_group.get(CreatureLootGroupColumnIndex::Chance as usize).unwrap(),
                        num_rolls: ValueRange::new(
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMin as usize).unwrap(),
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMax as usize).unwrap(),
                        ),
                        items: result_items,
                        condition_id: row_group.get(CreatureLootGroupColumnIndex::ConditionId as usize).unwrap(),
                    })
                }).unwrap().filter_map(|res| res.ok()).into_iter().collect();

                Ok(CreatureLootTable { id: loot_table_id, groups: result_groups })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).into_iter().collect()
    }
}
