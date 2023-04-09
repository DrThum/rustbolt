use std::{
    collections::HashMap,
    sync::{atomic::AtomicU32, Arc},
};

use futures::future::{BoxFuture, FutureExt};
use log::{error, trace};
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
};
use wow_srp::tbc_header::HeaderCrypto;

use crate::{
    entities::player::Player,
    protocol::{
        opcodes::Opcode,
        server::{ServerMessage, ServerMessagePayload},
    },
    world_context::WorldContext,
    world_socket::WorldSocket,
    WorldSocketError,
};

pub type PacketHandler = Box<
    dyn Send + Sync + Fn(Arc<WorldSession>, Arc<WorldContext>, Vec<u8>) -> BoxFuture<'static, ()>,
>;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(|session, ctx, data| $handler(session, ctx, data).boxed()) as PacketHandler,
        )
    };
}

pub struct OpcodeHandler {
    handlers: HashMap<u32, PacketHandler>,
}

impl OpcodeHandler {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::from([
                define_handler!(Opcode::MsgNullAction, WorldSession::unhandled),
                define_handler!(
                    Opcode::CmsgCharCreate,
                    WorldSession::handle_cmsg_char_create
                ),
                define_handler!(Opcode::CmsgCharEnum, WorldSession::handle_cmsg_char_enum),
                define_handler!(
                    Opcode::CmsgCharDelete,
                    WorldSession::handle_cmsg_char_delete
                ),
                define_handler!(
                    Opcode::CmsgPlayerLogin,
                    WorldSession::handle_cmsg_player_login
                ),
                define_handler!(Opcode::CmsgPing, WorldSession::handle_cmsg_ping),
                define_handler!(
                    Opcode::CmsgRealmSplit,
                    WorldSession::handle_cmsg_realm_split
                ),
                define_handler!(
                    Opcode::CmsgLogoutRequest,
                    WorldSession::handle_cmsg_logout_request
                ),
                define_handler!(
                    Opcode::CmsgItemQuerySingle,
                    WorldSession::handle_cmsg_item_query_single
                ),
                define_handler!(Opcode::CmsgNameQuery, WorldSession::handle_cmsg_name_query),
                define_handler!(Opcode::CmsgQueryTime, WorldSession::handle_cmsg_query_time),
                define_handler!(
                    Opcode::CmsgUpdateAccountData,
                    WorldSession::handle_cmsg_update_account_data
                ),
            ]),
        }
    }

    pub fn get_handler(&self, opcode: u32) -> &PacketHandler {
        self.handlers
            .get(&opcode)
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
                self.handlers.get(&(Opcode::MsgNullAction as u32)).unwrap()
            })
    }
}

pub struct WorldSession {
    socket: WorldSocket,
    pub account_id: u32,
    pub player: Arc<RwLock<Player>>,
    client_latency: AtomicU32,
}

impl WorldSession {
    pub fn new(socket: TcpStream, encryption: HeaderCrypto, account_id: u32) -> WorldSession {
        let (read_half, write_half) = tokio::io::split(socket);

        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let encryption = Arc::new(Mutex::new(encryption));

        let socket = WorldSocket {
            write_half,
            read_half,
            encryption,
            account_id,
        };

        WorldSession {
            socket,
            account_id,
            player: Arc::new(RwLock::new(Player::new())),
            client_latency: AtomicU32::new(0),
        }
    }

    pub async fn shutdown(&self) {
        self.socket.shutdown().await;
    }

    pub fn client_latency(&self) -> u32 {
        self.client_latency
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn update_client_latency(&self, latency: u32) {
        self.client_latency
            .store(latency, std::sync::atomic::Ordering::Relaxed);
    }

    pub async fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> {
        let mut socket = self.socket.write_half.lock().await;
        let mut encryption = self.socket.encryption.lock().await;

        trace!("Sending {:?} ({:#X})", Opcode::n(OPCODE).unwrap(), OPCODE);
        packet.send(&mut socket, &mut encryption).await
    }

    pub async fn process_incoming_packet(
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> Result<(), WorldSocketError> {
        let client_message = session.socket.read_packet().await?;
        let handler = world_context
            .opcode_handler
            .get_handler(client_message.header.opcode);

        handler(
            session.clone(),
            world_context.clone(),
            client_message.payload,
        )
        .await;

        Ok(())
    }
}
