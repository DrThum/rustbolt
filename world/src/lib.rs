use crate::protocol::client::ClientMessage;
use crate::protocol::server::ServerMessage;
use crate::repositories::account::AccountRepository;
use crate::shared::response_codes::ResponseCodes;
use std::sync::Arc;

pub use crate::datastore::DataStore;
use crate::protocol::packets::{CmsgAuthSession, SmsgAuthChallenge, SmsgAuthResponse};
use binrw::io::Cursor;
use binrw::BinReaderExt;
use game::world_context::WorldContext;
use hex::FromHex;
use log::{error, trace};
use protocol::client::ClientMessageHeader;
pub use session::session_holder::SessionHolder;
use session::world_session::WorldSession;
use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::tbc_header::ProofSeed;

pub mod config;
pub mod database_context;
mod datastore;
mod entities;
pub mod game;
mod protocol;
mod repositories;
pub mod session;
mod shared;

// TypeState pattern (https://yoric.github.io/post/rust-typestate/)
struct SocketOpened {
    socket: TcpStream,
    world_context: Arc<WorldContext>,
}
struct ServerSentAuthChallenge {
    seed: ProofSeed,
    socket: TcpStream,
    world_context: Arc<WorldContext>,
}
struct ServerSentAuthResponse {
    pub session: Arc<WorldSession>,
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
    SessionNotFound,
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
        mut self,
    ) -> Result<WorldSocketState<ServerSentAuthChallenge>, WorldSocketError> {
        let seed = ProofSeed::new();
        let packet = ServerMessage::new(SmsgAuthChallenge {
            server_seed: seed.seed(),
        });
        packet.send_unencrypted(&mut self.state.socket).await?;
        trace!("Sent SMSG_AUTH_CHALLENGE");

        Ok(WorldSocketState {
            state: ServerSentAuthChallenge {
                seed,
                socket: self.state.socket,
                world_context: self.state.world_context,
            },
        })
    }
}

impl WorldSocketState<ServerSentAuthChallenge> {
    async fn read_socket_plain(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        // let mut socket_guard = self.state.socket.lock().await;

        match self.state.socket.read_exact(&mut buf[..6]).await {
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
                self.state
                    .socket
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
        session_holder: Arc<RwLock<SessionHolder>>,
    ) -> Result<WorldSocketState<ServerSentAuthResponse>, WorldSocketError> {
        let auth_session_client_message = self.read_socket_plain().await?;

        let mut reader = Cursor::new(auth_session_client_message.payload);
        let cmsg_auth_session: CmsgAuthSession = reader.read_le()?;
        let username: String = cmsg_auth_session.username.to_string();
        let username: NormalizedString = NormalizedString::new(username).unwrap();

        let mut conn = self.state.world_context.database.auth.get()?;
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

        let session = WorldSession::new(self.state.socket, encryption, account_id);

        let packet = ServerMessage::new(SmsgAuthResponse {
            result: ResponseCodes::AuthOk as u8,
            billing_time: 0,
            billing_flags: 0,
            billing_rested: 0,
            expansion: 1,
            position_in_queue: 0,
        });

        session.send(packet).await.unwrap();

        {
            let mut session_holder = session_holder.write().await;

            if let Some(previous_session) = session_holder.insert_session(session).await {
                previous_session.shutdown().await;
            }
        }

        let session_holder = session_holder.read().await;
        if let Some(session) = session_holder.get_session_for_account(account_id) {
            Ok(WorldSocketState {
                state: ServerSentAuthResponse {
                    session: session.clone(),
                },
            })
        } else {
            Err(WorldSocketError::SessionNotFound)
        }
    }
}

pub async fn process(
    socket: TcpStream,
    world_context: Arc<WorldContext>,
    session_holder: Arc<RwLock<SessionHolder>>,
) -> Result<(), WorldSocketError> {
    let state = WorldSocketState {
        state: SocketOpened {
            socket,
            world_context: world_context.clone(),
        },
    }
    .send_challenge()
    .await?
    .handle_auth_session(session_holder.clone())
    .await?;

    loop {
        let session = state.state.session.clone();
        WorldSession::process_incoming_packet(session, world_context.clone()).await?;
    }
}
