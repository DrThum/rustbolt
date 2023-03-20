use std::sync::Arc;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{net::TcpStream, sync::Mutex};

pub struct WorldSession {
    pub socket: Arc<Mutex<TcpStream>>,
    pub db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_char: Arc<Pool<SqliteConnectionManager>>,
}

impl WorldSession {
    pub fn new(
        socket: Arc<Mutex<TcpStream>>,
        db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
        db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    ) -> WorldSession {
        WorldSession {
            socket,
            db_pool_auth,
            db_pool_char,
        }
    }
}
