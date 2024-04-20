use crate::models::loot::LootGroup;
use crate::models::loot::LootTable;
use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rand::distributions::WeightedIndex;
use rusqlite::named_params;

use crate::{models::loot::LootItem, utils::value_range::ValueRange};

pub struct LootRepository;

impl LootRepository {
    pub fn load_creature_loot_tables(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Vec<LootTable> {
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
                    .prepare_cached("SELECT id, chance, num_rolls_min, num_rolls_max, condition_id FROM creature_loot_table_groups JOIN loot_groups ON creature_loot_table_groups.loot_group_id = loot_groups.id WHERE creature_loot_table_id = :loot_table_id")
                    .unwrap();

                let result_groups = stmt_groups.query_map(named_params! { ":loot_table_id": loot_table_id }, |row_group| {
                    let group_id: u32 = row_group.get(CreatureLootGroupColumnIndex::Id as usize).unwrap();

                    let mut stmt_items = conn
                        .prepare_cached("SELECT item_id, chance, count_min, count_max, condition_id FROM loot_items WHERE group_id = :group_id")
                        .unwrap();

                    let result_items: Vec<LootItem> = stmt_items.query_map(named_params! { ":group_id": group_id }, |row_item| {
                        Ok(LootItem {
                            item_id: row_item.get(CreatureLootItemColumnIndex::ItemId as usize).unwrap(),
                            chance: row_item.get(CreatureLootItemColumnIndex::Chance as usize).unwrap(),
                            count: ValueRange::new(
                                row_item.get(CreatureLootItemColumnIndex::CountMin as usize).unwrap(),
                                row_item.get(CreatureLootItemColumnIndex::CountMax as usize).unwrap(),
                            ),
                            condition_id: row_item.get(CreatureLootItemColumnIndex::ConditionId as usize).unwrap(),
                        })
                    }).unwrap().filter_map(|res| res.ok()).collect();

                    bar.inc(1);
                    if bar.position() == count {
                        bar.finish();
                    }

                    assert!(!result_items.is_empty(), "{}", format!("loot group {group_id} has no item"));

                    let distribution = WeightedIndex::new(result_items.iter().map(|item| item.chance)).unwrap();

                    Ok(LootGroup {
                        chance: row_group.get(CreatureLootGroupColumnIndex::Chance as usize).unwrap(),
                        num_rolls: ValueRange::new(
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMin as usize).unwrap(),
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMax as usize).unwrap(),
                        ),
                        items: result_items,
                        condition_id: row_group.get(CreatureLootGroupColumnIndex::ConditionId as usize).unwrap(),
                        distribution
                    })
                }).unwrap().filter_map(|res| res.ok()).collect();

                Ok(LootTable { id: loot_table_id, groups: result_groups })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }

    pub fn fetch_loot_table_by_id(
        conn: &PooledConnection<SqliteConnectionManager>,
        id: u32,
    ) -> Option<LootTable> {
        let mut stmt = conn
            .prepare_cached("SELECT id FROM creature_loot_tables WHERE id = :id")
            .unwrap();
        let table_count = stmt
            .query_map(named_params! { ":id": id}, |row| row.get::<usize, u64>(0))
            .unwrap()
            .count();

        if table_count == 0 {
            return None;
        }

        let mut stmt_groups = conn
                    .prepare_cached("SELECT id, chance, num_rolls_min, num_rolls_max, condition_id FROM creature_loot_table_groups JOIN loot_groups ON creature_loot_table_groups.loot_group_id = loot_groups.id WHERE creature_loot_table_id = :loot_table_id")
                    .unwrap();

        // FIXME: duplicated code with load_creature_loot_tables
        let result_groups = stmt_groups.query_map(named_params! { ":loot_table_id": id }, |row_group| {
                    let group_id: u32 = row_group.get(CreatureLootGroupColumnIndex::Id as usize).unwrap();

                    let mut stmt_items = conn
                        .prepare_cached("SELECT item_id, chance, count_min, count_max, condition_id FROM loot_items WHERE group_id = :group_id")
                        .unwrap();

                    let result_items: Vec<LootItem> = stmt_items.query_map(named_params! { ":group_id": group_id }, |row_item| {
                        Ok(LootItem {
                            item_id: row_item.get(CreatureLootItemColumnIndex::ItemId as usize).unwrap(),
                            chance: row_item.get(CreatureLootItemColumnIndex::Chance as usize).unwrap(),
                            count: ValueRange::new(
                                row_item.get(CreatureLootItemColumnIndex::CountMin as usize).unwrap(),
                                row_item.get(CreatureLootItemColumnIndex::CountMax as usize).unwrap(),
                            ),
                            condition_id: row_item.get(CreatureLootItemColumnIndex::ConditionId as usize).unwrap(),
                        })
                    }).unwrap().filter_map(|res| res.ok()).collect();

                    let distribution = WeightedIndex::new(result_items.iter().map(|item| item.chance)).unwrap();

                    Ok(LootGroup {
                        chance: row_group.get(CreatureLootGroupColumnIndex::Chance as usize).unwrap(),
                        num_rolls: ValueRange::new(
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMin as usize).unwrap(),
                            row_group.get(CreatureLootGroupColumnIndex::NumRollsMax as usize).unwrap(),
                        ),
                        items: result_items,
                        condition_id: row_group.get(CreatureLootGroupColumnIndex::ConditionId as usize).unwrap(),
                        distribution
                    })
                }).unwrap().filter_map(|res| res.ok()).collect();

        Some(LootTable {
            id,
            groups: result_groups,
        })
    }
}

pub enum CreatureLootGroupColumnIndex {
    Id,
    Chance,
    NumRollsMin,
    NumRollsMax,
    ConditionId,
}

pub enum CreatureLootItemColumnIndex {
    ItemId,
    Chance,
    CountMin,
    CountMax,
    ConditionId,
}
