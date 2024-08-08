use r2d2_sqlite::SqliteConnectionManager;

pub mod repositories {
    pub mod wowhead_cache;
}

pub mod wowhead {
    pub mod models;
    pub mod service;
}

type DbPool = r2d2::Pool<SqliteConnectionManager>;
pub struct WorldDb(pub DbPool);
pub struct WowheadCacheDb(pub DbPool);
