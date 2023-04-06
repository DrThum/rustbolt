use std::{collections::HashMap, sync::Arc};

use futures::future::{BoxFuture, FutureExt};
use log::{error, trace};
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
    world_context::WorldContext,
    WorldSocketError,
};

pub type PacketHandler = Box<
    dyn Send
        + Sync
        + Fn(Arc<RwLock<WorldSession>>, Arc<WorldContext>, Vec<u8>) -> BoxFuture<'static, ()>,
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
    pub socket: Arc<Mutex<TcpStream>>,
    pub encryption: Arc<Mutex<HeaderCrypto>>,
    pub account_id: u32,
    pub player: Player,
    pub client_latency: u32,
    time_sync_counter: u32,
}

impl WorldSession {
    pub fn new(
        socket: Arc<Mutex<TcpStream>>,
        encryption: Arc<Mutex<HeaderCrypto>>,
        account_id: u32,
    ) -> WorldSession {
        WorldSession {
            socket,
            encryption,
            account_id,
            player: Player::new(),
            client_latency: 0,
            time_sync_counter: 0,
        }
    }

    pub async fn send<const OPCODE: u16, Payload: ServerMessagePayload<OPCODE>>(
        &self,
        packet: ServerMessage<OPCODE, Payload>,
    ) -> Result<(), binrw::Error> {
        trace!("Sent {:?} ({:#X})", Opcode::n(OPCODE).unwrap(), OPCODE);
        packet.send(&self.socket, &self.encryption).await
    }

    pub async fn process_incoming_packet(
        session_lock: Arc<RwLock<WorldSession>>,
        world_context: Arc<WorldContext>,
    ) -> Result<(), WorldSocketError> {
        let mut session = session_lock.write().await;
        let client_message = session.read_socket().await?;
        let handler = world_context
            .opcode_handler
            .get_handler(client_message.header.opcode);

        handler(
            session_lock.clone(),
            world_context.clone(),
            client_message.payload,
        )
        .await;

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
