use std::{collections::HashMap, sync::Arc};

use futures::future::{BoxFuture, FutureExt};
use log::{error, trace};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::{
    io::AsyncReadExt,
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::player::Player,
    game::world::World,
    protocol::{
        client::{ClientMessage, ClientMessageHeader},
        opcodes::Opcode,
        server::{ServerMessage, ServerMessagePayload},
    },
    WorldSocketError,
};

pub type PacketHandler =
    Box<dyn Send + Sync + FnMut(&mut WorldSession, Vec<u8>) -> BoxFuture<'static, ()>>;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(|session, data| $handler(session, data).boxed()) as PacketHandler,
        )
    };
}

pub struct WorldSession {
    pub socket: Arc<Mutex<TcpStream>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    pub db_pool_world: Arc<Pool<SqliteConnectionManager>>,
    pub account_id: u32,
    pub world: Arc<RwLock<&'static mut World>>,
    pub player: Player,
    pub client_latency: u32,
    time_sync_counter: u32,
    handlers: HashMap<u32, PacketHandler>,
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
            handlers: HashMap::from([
                define_handler!(Opcode::MsgNullAction, Self::unhandled),
                define_handler!(Opcode::CmsgCharCreate, Self::handle_cmsg_char_create),
                define_handler!(Opcode::CmsgCharEnum, Self::handle_cmsg_char_enum),
                define_handler!(Opcode::CmsgCharDelete, Self::handle_cmsg_char_delete),
                define_handler!(Opcode::CmsgPlayerLogin, Self::handle_cmsg_player_login),
                define_handler!(Opcode::CmsgPing, Self::handle_cmsg_ping),
                define_handler!(Opcode::CmsgRealmSplit, Self::handle_cmsg_realm_split),
                define_handler!(Opcode::CmsgLogoutRequest, Self::handle_cmsg_logout_request),
                define_handler!(
                    Opcode::CmsgItemQuerySingle,
                    Self::handle_cmsg_item_query_single
                ),
                define_handler!(Opcode::CmsgNameQuery, Self::handle_cmsg_name_query),
                define_handler!(Opcode::CmsgQueryTime, Self::handle_cmsg_query_time),
                define_handler!(
                    Opcode::CmsgUpdateAccountData,
                    Self::handle_cmsg_update_account_data
                ),
            ]),
        }
    }

    pub fn get_handler(&self, opcode: u32) -> &'static PacketHandler {
        self.handlers
            .get_mut(&opcode)
            .map(|h| {
                trace!("Received {:?} ({:#X})", Opcode::n(opcode).unwrap(), opcode);
                h
            })
            .unwrap_or_else(|| {
                error!(
                    "Received unhandled {:?} ({:#X})",
                    Opcode::n(opcode).unwrap(),
                    opcode
                );
                self.handlers
                    .get_mut(&(Opcode::MsgNullAction as u32))
                    .unwrap()
            })
    }

    pub async fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> {
        trace!("Sent {:?} ({:#X})", Opcode::n(OPCODE).unwrap(), OPCODE);
        packet.send(&self.socket, &self.encryption).await
    }

    pub async fn process_incoming_packet(&mut self) -> Result<(), WorldSocketError> {
        let client_message = self.read_socket().await?;
        let handler = &mut self.get_handler(client_message.header.opcode);

        handler(self, client_message.payload).await;

        Ok(())
    }

    async fn read_socket(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket = self.socket.lock().await;

        match socket.read(&mut buf[..6]).await {
            Ok(0) => {
                trace!("Client disconnected");
                return Err(WorldSocketError::ClientDisconnected);
            }
            Ok(n) if n < 6 => {
                error!("Received less than 6 bytes, need to handle partial header");
                return Err(WorldSocketError::SocketError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Received an incomplete client header",
                )));
            }
            Ok(_) => {
                let mut encryption = self.encryption.lock().await;
                let client_header: ClientMessageHeader =
                    encryption.decrypt_client_header(buf).into();

                let bytes_to_read: usize = client_header.size as usize - 4; // Client opcode is u32
                let mut buf_payload = [0_u8; 1024];
                if bytes_to_read > 0 {
                    socket
                        .read(&mut buf_payload[..bytes_to_read])
                        .await
                        .unwrap();

                    Ok(ClientMessage {
                        header: client_header,
                        payload: buf_payload[..bytes_to_read].to_vec(),
                    })
                } else {
                    Ok(ClientMessage {
                        header: client_header,
                        payload: vec![],
                    })
                }
            }
            Err(e) => {
                error!("Socket error, closing");
                return Err(WorldSocketError::SocketError(e));
            }
        }
    }
}
