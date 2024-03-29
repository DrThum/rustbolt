use actix_web::{
    get, http::header::ContentType, post, web, App, HttpResponse, HttpServer, Responder,
};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::named_params;
use serde::Serialize;

type DbPool = r2d2::Pool<SqliteConnectionManager>;

enum CreatureSpawnColumnIndex {
    Guid,
    Entry,
    Map,
    PositionX,
    PositionY,
    PositionZ,
    Orientation,
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
}

#[get("/")]
async fn hello(db_pool: web::Data<DbPool>) -> actix_web::Result<impl Responder> {
    let spawns: Vec<CreatureSpawnDbRecord> = web::block(move || {
        // Obtaining a connection from the pool is also a potentially blocking operation.
        // So, it should be called within the `web::block` closure, as well.
        let conn = db_pool.get().expect("couldn't get db connection from pool");

        let mut stmt = conn.prepare_cached("SELECT guid, entry, map, position_x, position_y, position_z, orientation FROM creature_spawns WHERE map = :map_id ORDER BY RANDOM() LIMIT 20").unwrap();

        let result = stmt
            .query_map(named_params! { ":map_id": 0 }, |row| {
                use CreatureSpawnColumnIndex::*;

                Ok(CreatureSpawnDbRecord {
                    guid: row.get(Guid as usize).unwrap(),
                    entry: row.get(Entry as usize).unwrap(),
                    map: row.get(Map as usize).unwrap(),
                    position_x: row.get(PositionX as usize).unwrap(),
                    position_y: row.get(PositionY as usize).unwrap(),
                    position_z: row.get(PositionZ as usize).unwrap(),
                    orientation: row.get(Orientation as usize).unwrap(),
                })
            })
            .unwrap();

        result.filter_map(|res| res.ok()).collect()
    })
    .await?;
    // .map_err(error::ErrorInternalServerError)?;

    // Ok(HttpResponse::Ok().body("Hello world!"))
    Ok(HttpResponse::Ok()
        .content_type(ContentType::json())
        .body(serde_json::to_string(&spawns).unwrap()))
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // connect to SQLite DB
    // FIXME: load data dir from config
    let db_pool = r2d2::Pool::new(SqliteConnectionManager::file("data/databases/world.db"))
        .expect("failed to create DB connection pool");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .service(hello)
            .service(echo)
            .route("/hey", web::get().to(manual_hello))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
