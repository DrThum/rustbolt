use actix_web::{web, App, HttpServer};
use r2d2_sqlite::SqliteConnectionManager;
use web_proxy::{
    controllers::{
        loot_tables::{create_loot_table, fetch_loot_table_from_wowhead, update_loot_table},
        spawns::{get_spawns, get_template},
    },
    WorldDb, WowheadCacheDb,
};

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
            .service(create_loot_table)
            .service(update_loot_table)
            .service(fetch_loot_table_from_wowhead)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
