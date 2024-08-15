use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;
use shared::repositories::loot::LootRepository;

use crate::{
    Bounds, CreatureSpawnColumnIndex, CreatureSpawnDbRecord, CreatureTemplate,
    CreatureTemplateColumnIndex,
};

pub struct SpawnsRepository;

impl SpawnsRepository {
    pub fn get_spawns_in_bounds(
        conn: &PooledConnection<SqliteConnectionManager>,
        bounds: &Bounds,
    ) -> Vec<CreatureSpawnDbRecord> {
        let mut stmt = conn.prepare_cached("SELECT guid, creature_spawns.entry, map, position_x, position_y, position_z, orientation, name FROM creature_spawns JOIN creature_templates ON creature_templates.entry = creature_spawns.entry WHERE map = :map_id AND position_x >= :min_x AND position_x <= :max_x AND position_y >= :min_y AND position_y <= :max_y").unwrap();

        let result = stmt
            .query_map(named_params! { ":map_id": bounds.map_id, ":min_x": bounds.south_west.x, ":max_x": bounds.north_east.x, ":min_y": bounds.north_east.y, ":max_y": bounds.south_west.y }, |row| {
                use CreatureSpawnColumnIndex::*;

                Ok(CreatureSpawnDbRecord {
                    guid: row.get(Guid as usize).unwrap(),
                    entry: row.get(Entry as usize).unwrap(),
                    map: row.get(Map as usize).unwrap(),
                    position_x: row.get(PositionX as usize).unwrap(),
                    position_y: row.get(PositionY as usize).unwrap(),
                    position_z: row.get(PositionZ as usize).unwrap(),
                    orientation: row.get(Orientation as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    }

    pub fn get_creature_template(
        conn: &PooledConnection<SqliteConnectionManager>,
        entry: u32,
    ) -> Option<CreatureTemplate> {
        let mut stmt = conn
            .prepare_cached(
                "SELECT entry, name, loot_table_id FROM creature_templates WHERE entry = :entry",
            )
            .unwrap();

        let mut result = stmt
            .query_map(named_params! { ":entry": entry }, |row| {
                use CreatureTemplateColumnIndex::*;

                Ok(CreatureTemplate {
                    entry: row.get(Entry as usize).unwrap(),
                    name: row.get(Name as usize).unwrap(),
                    loot_table_id: row.get(LootTableId as usize).unwrap(),
                    loot_table: None,
                })
            })
            .unwrap();

        if let Ok(mut template) = result.next().unwrap() {
            let loot_table = template.loot_table_id.and_then(|loot_table_id| {
                LootRepository::fetch_loot_table_by_id(conn, loot_table_id).unwrap()
            });

            template.loot_table = loot_table;
            Some(template)
        } else {
            None
        }
    }
}
