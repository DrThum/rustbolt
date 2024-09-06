use r2d2_sqlite::SqliteConnectionManager;
use serde::{Deserialize, Deserializer, Serialize};
use shared::models::loot::LootTable;

pub mod controllers {
    pub mod loot_tables;
    pub mod spawns;
}

pub mod repositories {
    pub mod spawns;
    pub mod wowhead_cache;
}

pub mod wowhead {
    pub mod models;
    pub mod service;
}

type DbPool = r2d2::Pool<SqliteConnectionManager>;
pub struct WorldDb(pub DbPool);
pub struct WowheadCacheDb(pub DbPool);

enum CreatureSpawnColumnIndex {
    Guid,
    Entry,
    Map,
    PositionX,
    PositionY,
    PositionZ,
    Orientation,
    Name,
}

#[derive(Serialize)]
pub struct CreatureSpawnDbRecord {
    pub guid: u32,
    pub entry: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub name: String,
}

pub enum CreatureTemplateColumnIndex {
    Entry,
    Name,
    LootTableId,
}

#[derive(Serialize)]
pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub loot_table_id: Option<u32>,
    pub loot_table: Option<LootTable>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl<'de> serde::Deserialize<'de> for Point {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        let parts: Vec<f32> = raw
            .split(',')
            .take(2)
            .map(|p| p.parse::<f32>().expect("coord is not a float"))
            .collect();

        Ok(Point {
            x: parts[0],
            y: parts[1],
        })
    }
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Bounds {
    pub map_id: u32,
    pub south_west: Point,
    pub north_east: Point,
}
