use crate::entities::player::Player;
use crate::protocol::client::ClientMessage;
use crate::protocol::server::ServerMessage;
use crate::repositories::account::AccountRepository;
use crate::shared::response_codes::ResponseCodes;
use std::sync::Arc;

use crate::protocol::packets::{CmsgAuthSession, SmsgAuthChallenge, SmsgAuthResponse};
use binrw::io::Cursor;
use binrw::BinReaderExt;
use game::world::World;
use hex::FromHex;
use log::{error, trace};
use protocol::client::ClientMessageHeader;
use protocol::handlers;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use world_session::WorldSession;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::tbc_header::ProofSeed;

pub mod config;
mod datastore;
mod entities;
pub mod game;
mod protocol;
mod repositories;
mod shared;
pub mod world_session;

// TypeState pattern (https://yoric.github.io/post/rust-typestate/)
struct SocketOpened {
    socket: Arc<Mutex<TcpStream>>,
    db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    db_pool_world: Arc<Pool<SqliteConnectionManager>>,
    world: Arc<&'static World>,
}
struct ServerSentAuthChallenge {
    seed: ProofSeed,
    socket: Arc<Mutex<TcpStream>>,
    db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    db_pool_world: Arc<Pool<SqliteConnectionManager>>,
    world: Arc<&'static World>,
}
struct ServerSentAuthResponse {
    session: Arc<Mutex<WorldSession>>,
}

struct WorldSocketState<S> {
    state: S,
}

#[derive(Debug)]
pub enum WorldSocketError {
    ClientDisconnected,
    SocketError(std::io::Error),
    BinRwError(binrw::Error),
    DbError(r2d2::Error),
}

impl From<std::io::Error> for WorldSocketError {
    fn from(value: std::io::Error) -> Self {
        Self::SocketError(value)
    }
}

impl From<binrw::Error> for WorldSocketError {
    fn from(value: binrw::Error) -> Self {
        Self::BinRwError(value)
    }
}

impl From<r2d2::Error> for WorldSocketError {
    fn from(value: r2d2::Error) -> Self {
        Self::DbError(value)
    }
}

impl WorldSocketState<SocketOpened> {
    async fn send_challenge(
        self,
    ) -> Result<WorldSocketState<ServerSentAuthChallenge>, WorldSocketError> {
        let seed = ProofSeed::new();
        let packet = ServerMessage::new(SmsgAuthChallenge {
            server_seed: seed.seed(),
        });
        let mut socket_guard = self.state.socket.lock().await;
        packet.send_unencrypted(&mut socket_guard).await?;
        drop(socket_guard);
        trace!("Sent SMSG_AUTH_CHALLENGE");

        Ok(WorldSocketState {
            state: ServerSentAuthChallenge {
                seed,
                socket: self.state.socket,
                db_pool_auth: self.state.db_pool_auth,
                db_pool_char: self.state.db_pool_char,
                db_pool_world: self.state.db_pool_world,
                world: self.state.world,
            },
        })
    }
}

impl WorldSocketState<ServerSentAuthChallenge> {
    async fn read_socket_plain(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket_guard = self.state.socket.lock().await;

        match socket_guard.read_exact(&mut buf[..6]).await {
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
                let mut reader = Cursor::new(buf);
                let client_header: ClientMessageHeader = reader.read_le()?;
                let bytes_to_read: usize = client_header.size as usize - 4; // Client opcode is u32
                let mut buf_payload = [0_u8; 1024];
                socket_guard
                    .read_exact(&mut buf_payload[..bytes_to_read])
                    .await
                    .unwrap();

                Ok(ClientMessage {
                    header: client_header,
                    payload: buf_payload[..bytes_to_read].to_vec(),
                })
            }
            Err(e) => {
                error!("Socket error, closing");
                return Err(WorldSocketError::SocketError(e));
            }
        }
    }

    async fn handle_auth_session(
        mut self,
    ) -> Result<WorldSocketState<ServerSentAuthResponse>, WorldSocketError> {
        let auth_session_client_message = self.read_socket_plain().await?;

        let mut reader = Cursor::new(auth_session_client_message.payload);
        let cmsg_auth_session: CmsgAuthSession = reader.read_le()?;
        let username: String = cmsg_auth_session.username.to_string();
        let username: NormalizedString = NormalizedString::new(username).unwrap();

        let mut conn = self.state.db_pool_auth.get()?;
        let (account_id, session_key) =
            AccountRepository::fetch_id_and_session_key(&mut conn, username.to_string()).unwrap();
        let session_key: [u8; 40] = <Vec<u8>>::from_hex(session_key)
            .unwrap()
            .try_into()
            .unwrap();

        let encryption = self
            .state
            .seed
            .into_header_crypto(
                &username,
                session_key,
                cmsg_auth_session._client_proof,
                cmsg_auth_session._client_seed,
            )
            .unwrap();
        let encryption = Arc::new(Mutex::new(encryption));

        let packet = ServerMessage::new(SmsgAuthResponse {
            result: ResponseCodes::AuthOk as u8,
            billing_time: 0,
            billing_flags: 0,
            billing_rested: 0,
            expansion: 1,
            position_in_queue: 0,
        });

        packet.send(&self.state.socket, &encryption).await?;
        trace!("Sent SMSG_AUTH_RESPONSE");

        Ok(WorldSocketState {
            state: ServerSentAuthResponse {
                session: Arc::new(Mutex::new(WorldSession {
                    socket: self.state.socket,
                    encryption,
                    db_pool_auth: self.state.db_pool_auth,
                    db_pool_char: self.state.db_pool_char,
                    db_pool_world: self.state.db_pool_world,
                    account_id,
                    world: self.state.world,
                    player: Player::new(),
                })),
            },
        })
    }
}

impl WorldSocketState<ServerSentAuthResponse> {
    async fn read_socket(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let session = self.state.session.lock().await;
        let mut socket = session.socket.lock().await;

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
                let mut encryption = session.encryption.lock().await;
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

    async fn handle_packet(&mut self) -> Result<(), WorldSocketError> {
        let client_message = self.read_socket().await?;
        let handler = handlers::get_handler(client_message.header.opcode);

        handler(client_message.payload, Arc::clone(&self.state.session)).await;

        Ok(())
    }
}

pub async fn process(
    socket: Arc<Mutex<TcpStream>>,
    db_pool_auth: Arc<Pool<SqliteConnectionManager>>,
    db_pool_char: Arc<Pool<SqliteConnectionManager>>,
    db_pool_world: Arc<Pool<SqliteConnectionManager>>,
    world: Arc<&'static World>,
) -> Result<(), WorldSocketError> {
    let mut state = WorldSocketState {
        state: SocketOpened {
            socket,
            db_pool_auth,
            db_pool_char,
            db_pool_world,
            world,
        },
    }
    .send_challenge()
    .await?
    .handle_auth_session()
    .await?;

    loop {
        state.handle_packet().await?;
    }
}
