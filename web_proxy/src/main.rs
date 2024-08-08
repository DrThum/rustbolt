use actix_web::{
    get, http::header::ContentType, put, web, App, HttpResponse, HttpServer, Responder,
};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;
use serde::{Deserialize, Deserializer, Serialize};
use shared::{
    models::loot::{LootTable, UpdateLootTable},
    repositories::loot::LootRepository,
};
use web_proxy::{
    wowhead::{self, models::WowheadEntityType},
    WorldDb, WowheadCacheDb,
};

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

enum CreatureTemplateColumnIndex {
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
struct Point {
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
struct Bounds {
    pub map_id: u32,
    pub south_west: Point,
    pub north_east: Point,
}

#[get("/spawns")]
async fn get_spawns(
    db_pool: web::Data<WorldDb>,
    bounds: web::Query<Bounds>,
) -> actix_web::Result<impl Responder> {
    let spawns: Vec<CreatureSpawnDbRecord> = web::block(move || {
        let bounds = bounds.0;

        // Obtaining a connection from the pool is also a potentially blocking operation.
        // So, it should be called within the `web::block` closure, as well.
        let conn = db_pool.0.get().expect("couldn't get db connection from pool");

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
    })
    .await?;

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&spawns).unwrap()))
}

#[get("/spawn/{entry}")]
async fn get_template(
    db_pool: web::Data<WorldDb>,
    path: web::Path<u32>,
) -> actix_web::Result<impl Responder> {
    let template: Option<CreatureTemplate> = web::block(move || {
        let conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");

        let mut stmt = conn
            .prepare_cached(
                "SELECT entry, name, loot_table_id FROM creature_templates WHERE entry = :entry",
            )
            .unwrap();

        let mut result = stmt
            .query_map(named_params! { ":entry": path.into_inner() }, |row| {
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
                LootRepository::fetch_loot_table_by_id(&conn, loot_table_id).unwrap()
            });

            template.loot_table = loot_table;
            Some(template)
        } else {
            None
        }
    })
    .await?;

    if let Some(template) = template {
        Ok(HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(serde_json::to_string(&template).unwrap()))
    } else {
        Ok(HttpResponse::NotFound().into())
    }
}

#[put("/spawn/{entry}/lootTable")]
async fn update_loot_table(
    db_pool: web::Data<WorldDb>,
    path: web::Path<u32>,
    loot_table: web::Json<UpdateLootTable>,
) -> actix_web::Result<impl Responder> {
    let template_id = path.into_inner();
    let updated_loot_table: LootTable = web::block(move || {
        let mut conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");
        let loot_table_id = loot_table.0.id;

        LootRepository::update_loot_table(&mut conn, template_id, loot_table.0);
        LootRepository::fetch_loot_table_by_id(&conn, loot_table_id).unwrap()
    })
    .await?
    .expect("cannot find the loot table that we just updated");

    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&updated_loot_table).unwrap()))
}

#[get("/wowhead/{entity_type}/{id}")]
async fn fetch_loot_table_from_wowhead(
    db_pool: web::Data<WowheadCacheDb>,
    path: web::Path<(String, u32)>,
) -> actix_web::Result<impl Responder> {
    let (entity_type, id) = path.into_inner();

    web::block(move || {
        let conn = db_pool
            .0
            .get()
            .expect("couldn't get db connection from pool");
        let maybe_loot_table =
            wowhead::service::get_loot_table(&conn, WowheadEntityType::Npc /* TODO */, id);
        println!("controller: table is found {}", maybe_loot_table.is_some());
    })
    .await?;

    Ok(HttpResponse::Ok())
}

mod embedded_wowhead_cache {
    use refinery::embed_migrations;
    embed_migrations!("../sql_migrations/wowhead_cache");
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // connect to SQLite DBs
    // FIXME: load data dir from config
    let world_db_pool = r2d2::Pool::new(SqliteConnectionManager::file("data/databases/world.db"))
        .expect("failed to create DB connection pool (world)");
    let cache_db_pool = r2d2::Pool::new(
        SqliteConnectionManager::file("data/databases/wowhead_cache.db").with_init(move |c| {
            embedded_wowhead_cache::migrations::runner().run(c).unwrap();
            Ok(())
        }),
    )
    .expect("failed to create DB connection pool (cache)");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(WorldDb(world_db_pool.clone())))
            .app_data(web::Data::new(WowheadCacheDb(cache_db_pool.clone())))
            .service(get_spawns)
            .service(get_template)
            .service(update_loot_table)
            .service(fetch_loot_table_from_wowhead)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
