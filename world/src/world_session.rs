use std::sync::Arc;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{net::TcpStream, sync::Mutex};
use wow_srp::tbc_header::HeaderCrypto;

pub struct WorldSession {
    pub socket: Arc<Mutex<TcpStream>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_char: Arc<Pool<SqliteConnectionManager>>,
}
