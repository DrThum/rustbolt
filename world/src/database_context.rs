use std::sync::Arc;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

pub struct DatabaseContext {
    pub auth: Arc<Pool<SqliteConnectionManager>>,
    pub characters: Arc<Pool<SqliteConnectionManager>>,
    pub world: Arc<Pool<SqliteConnectionManager>>,
}
