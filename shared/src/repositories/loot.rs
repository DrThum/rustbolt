use crate::models::loot::LootGroup;
use crate::models::loot::LootTable;
use crate::models::loot::UpdateLootItem;
use crate::models::loot::UpdateLootTable;
use indicatif::ProgressBar;
use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rand::distributions::WeightedIndex;
use rusqlite::named_params;
use rusqlite::Transaction;

use crate::{models::loot::LootItem, utils::value_range::ValueRange};

use super::error::RResult;

pub struct LootRepository;

impl LootRepository {
    pub fn load_loot_tables(
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> RResult<Vec<LootTable>> {
        let mut stmt = conn.prepare_cached("SELECT COUNT(id) FROM loot_tables")?;
        let mut group_count = stmt.query_map([], |row| row.get::<usize, u64>(0)).unwrap();

        let count = group_count.next().unwrap().unwrap_or(0);
        let bar = ProgressBar::new(count);

        let mut stmt = conn.prepare_cached("SELECT id FROM loot_tables").unwrap();

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
        let mut stmt = conn.prepare_cached("SELECT id FROM loot_tables WHERE id = :id")?;
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
        let mut stmt_groups = conn.prepare_cached(
            "
                        SELECT id, chance, num_rolls_min, num_rolls_max, condition_id
                        FROM loot_table_groups
                        JOIN loot_groups ON loot_table_groups.loot_group_id = loot_groups.id
                        WHERE loot_table_id = :loot_table_id",
        )?;

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
                        id: row_group.get(Id as usize)?,
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

    pub fn replace_loot_table(
        conn: &mut PooledConnection<SqliteConnectionManager>,
        template_id: u32,
        loot_table: UpdateLootTable,
    ) {
        let transaction = conn.transaction().unwrap();

        {
            let mut stmt = transaction
                .prepare_cached(
                    "INSERT OR IGNORE INTO loot_tables(id, description)
                VALUES (:loot_table_id, NULL)",
                )
                .unwrap();

            stmt.execute(named_params! { ":loot_table_id": loot_table.id })
                .unwrap();

            let mut stmt = transaction
                .prepare_cached(
                    "UPDATE creature_templates
                SET loot_table_id = :loot_table_id
                WHERE entry = :template_id",
                )
                .unwrap();
            stmt.execute(
                named_params! { ":loot_table_id": loot_table.id, ":template_id": template_id },
            )
            .unwrap();

            // Unlink groups from the loot table (they are relinked later)
            let mut stmt = transaction
                .prepare_cached(
                    "DELETE FROM loot_table_groups WHERE loot_table_id = :loot_table_id",
                )
                .unwrap();

            stmt.execute(named_params! { ":loot_table_id": loot_table.id })
                .unwrap();

            for group in loot_table.groups.iter() {
                // If the group already exists, update it and replace its items
                let group_id = if let Some(group_id) = group.id {
                    let mut stmt = transaction.prepare_cached(
                        "UPDATE loot_groups
                        SET chance = :chance, num_rolls_min = :num_rolls_min, num_rolls_max = :num_rolls_max
                        WHERE id = :group_id"
                    ).unwrap();

                    stmt.execute(named_params! {
                        ":chance": group.chance,
                        ":num_rolls_min": group.num_rolls.min(),
                        ":num_rolls_max": group.num_rolls.max(),
                        ":group_id": group_id,
                    })
                    .unwrap();

                    let mut stmt = transaction
                        .prepare_cached("DELETE FROM loot_items WHERE group_id = :group_id")
                        .unwrap();

                    stmt.execute(named_params! { ":group_id": group_id})
                        .unwrap();

                    Self::add_items_to_group(&transaction, group_id as i64, &group.items);

                    group_id as i64
                } else {
                    // Otherwise, create the group and link it to the loot table
                    let mut stmt = transaction.prepare_cached(
                        "INSERT INTO loot_groups(id, chance, num_rolls_min, num_rolls_max, condition_id)
                        VALUES(NULL, :chance, :num_rolls_min, :num_rolls_max, NULL)"
                    ).unwrap();

                    stmt.execute(named_params! {
                        ":chance": group.chance,
                        ":num_rolls_min": group.num_rolls.min(),
                        ":num_rolls_max": group.num_rolls.max(),
                    })
                    .unwrap();

                    let group_id = transaction.last_insert_rowid();

                    Self::add_items_to_group(&transaction, group_id, &group.items);

                    group_id
                };

                let mut stmt = transaction
                    .prepare_cached(
                        "INSERT INTO loot_table_groups(loot_table_id, loot_group_id, description)
                    VALUES(:loot_table_id, :group_id, NULL)",
                    )
                    .unwrap();

                stmt.execute(named_params! {
                    ":loot_table_id": loot_table.id,
                    ":group_id": group_id,
                })
                .unwrap();
            }

            // Clear orphaned loot groups
            let mut stmt = transaction.prepare_cached(
                "DELETE FROM loot_groups WHERE id NOT IN (SELECT loot_group_id FROM loot_table_groups)"
            ).unwrap();

            stmt.execute([]).unwrap();
        }

        transaction.commit().unwrap();
    }

    fn add_items_to_group(conn: &Transaction, group_id: i64, items: &[UpdateLootItem]) {
        for item in items.iter() {
            let mut stmt = conn.prepare_cached(
                "INSERT INTO loot_items(group_id, item_id, chance, count_min, count_max, condition_id)
                VALUES(:group_id, :item_id, :chance, :count_min, :count_max, NULL)"
            ).unwrap();

            stmt.execute(named_params! {
                ":group_id": group_id,
                ":item_id": item.item_id,
                ":chance": item.chance,
                ":count_min": item.count.min(),
                ":count_max": item.count.max(),
            })
            .unwrap();
        }
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
