use crate::models::loot::LootGroup;
use crate::models::loot::LootTable;
use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rand::distributions::WeightedIndex;
use rusqlite::named_params;

use crate::{models::loot::LootItem, utils::value_range::ValueRange};

use super::error::RResult;

pub struct LootRepository;

impl LootRepository {
    pub fn load_creature_loot_tables(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> RResult<Vec<LootTable>> {
        let mut stmt = conn.prepare_cached("SELECT COUNT(id) FROM creature_loot_tables")?;
        let mut group_count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = group_count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn
            .prepare_cached("SELECT id FROM creature_loot_tables")
            .unwrap();

        let result = stmt
            .query_map([], |row| {
                let loot_table_id: u32 = row.get(0).unwrap();
                let groups = Self::fetch_loot_groups(conn, loot_table_id).unwrap_or_default();

                bar.inc(1);
                if bar.position() == count {
                    bar.finish();
                }

                Ok(LootTable {
                    id: loot_table_id,
                    groups,
                })
            })
            .unwrap();

        Ok(result.filter_map(Result::ok).collect())
    }

    pub fn fetch_loot_table_by_id(
        conn: &PooledConnection<SqliteConnectionManager>,
        id: u32,
    ) -> RResult<Option<LootTable>> {
        let mut stmt = conn.prepare_cached("SELECT id FROM creature_loot_tables WHERE id = :id")?;
        let loot_table_exists = stmt
            .query_map(named_params! { ":id": id }, |row| row.get::<usize, u64>(0))?
            .filter_map(Result::ok)
            .any(|count| count > 0);

        if !loot_table_exists {
            return Ok(None);
        }

        let groups = Self::fetch_loot_groups(conn, id).unwrap_or_default();

        Ok(Some(LootTable { id, groups }))
    }

    fn fetch_loot_groups(
        conn: &PooledConnection<SqliteConnectionManager>,
        loot_table_id: u32,
    ) -> RResult<Vec<LootGroup>> {
        let mut stmt_groups = conn
                    .prepare_cached("
                        SELECT id, chance, num_rolls_min, num_rolls_max, condition_id
                        FROM creature_loot_table_groups
                        JOIN loot_groups ON creature_loot_table_groups.loot_group_id = loot_groups.id
                        WHERE creature_loot_table_id = :loot_table_id")?;

        let result_groups = stmt_groups
            .query_map(
                named_params! { ":loot_table_id": loot_table_id },
                |row_group| {
                    use CreatureLootGroupColumnIndex::*;

                    let group_id: u32 = row_group.get(Id as usize)?;

                    let mut stmt_items = conn.prepare_cached(
                        "
                    SELECT item_id, chance, count_min, count_max, condition_id
                    FROM loot_items
                    WHERE group_id = :group_id",
                    )?;

                    let result_items: Vec<LootItem> = stmt_items
                        .query_map(named_params! { ":group_id": group_id }, |row_item| {
                            use CreatureLootItemColumnIndex::*;

                            Ok(LootItem {
                                item_id: row_item.get(ItemId as usize)?,
                                chance: row_item.get(Chance as usize)?,
                                count: ValueRange::new(
                                    row_item.get(CountMin as usize)?,
                                    row_item.get(CountMax as usize)?,
                                ),
                                condition_id: row_item.get(ConditionId as usize)?,
                            })
                        })?
                        .filter_map(Result::ok)
                        .collect();

                    assert!(
                        !result_items.is_empty(),
                        "{}",
                        format!("loot group {group_id} has no item")
                    );

                    let distribution =
                        WeightedIndex::new(result_items.iter().map(|item| item.chance)).unwrap();

                    Ok(LootGroup {
                        chance: row_group.get(Chance as usize)?,
                        num_rolls: ValueRange::new(
                            row_group.get(NumRollsMin as usize)?,
                            row_group.get(NumRollsMax as usize)?,
                        ),
                        items: result_items,
                        condition_id: row_group.get(ConditionId as usize)?,
                        distribution,
                    })
                },
            )?
            .filter_map(Result::ok)
            .collect();

        Ok(result_groups)
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
