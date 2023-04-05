use std::sync::Arc;

use binrw::BinWrite;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::player::Player,
    game::world::World,
    protocol::server::{ServerMessage, ServerMessagePayload},
};

pub struct WorldSession {
    socket: Arc<Mutex<TcpStream>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_world: Arc<Pool<SqliteConnectionManager>>,
    pub account_id: u32,
    pub world: Arc<RwLock<&'static mut World>>,
    pub player: Player,
    pub client_latency: u32,
    time_sync_counter: u32,
}

impl WorldSession {
    pub fn new(
        socket: Arc<Mutex<TcpStream>>,
        encryption: Arc<Mutex<HeaderCrypto>>,
        db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
        db_pool_char: Arc<Pool<SqliteConnectionManager>>,
        db_pool_world: Arc<Pool<SqliteConnectionManager>>,
        account_id: u32,
        world: Arc<RwLock<&'static mut World>>,
    ) -> WorldSession {
        WorldSession {
            socket,
            encryption,
            db_pool_auth,
            db_pool_char,
            db_pool_world,
            account_id,
            world,
            player: Player::new(),
            client_latency: 0,
            time_sync_counter: 0,
        }
    }

    pub async fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE> + BinWrite>(
        &self,
        packet: ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> where
        for<'a> <Payload>::Args<'a>: Default,
    {
        packet.send(&self.socket, &self.encryption).await
    }
}
