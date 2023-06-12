use crate::protocol::client::ClientMessage;
use crate::protocol::server::ServerMessage;
use crate::repositories::account::AccountRepository;
use crate::shared::constants::{ADDON_PUBLIC_KEY, STANDARD_ADDON_CRC};
use crate::shared::response_codes::ResponseCodes;
use std::sync::Arc;

pub use crate::datastore::DataStore;
use crate::protocol::packets::{
    ClientAddonInfo, CmsgAuthSession, ServerAddonInfo, SmsgAddonInfo, SmsgAuthChallenge,
    SmsgAuthResponse,
};
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
use tokio::sync::mpsc::error::SendError;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::tbc_header::ProofSeed;

pub mod config;
pub mod database_context;
mod datastore;
mod ecs {
    pub mod components;
    pub mod resources;
    pub mod systems;
}
mod entities {
    pub mod object_guid;
    pub mod position;
    pub mod update;
    pub mod update_fields;

    pub mod creature;
    pub mod item;
    pub mod player;

    pub mod internal_values;
}
pub mod game {
    pub mod map;
    pub mod map_manager;
    pub mod quad_tree;
    pub mod world_context;
}
mod protocol {
    pub mod client;
    pub mod handlers;
    pub mod opcodes;
    pub mod packets;
    pub mod server;
}
mod repositories {
    pub mod account;
    pub mod character;
    pub mod creature;
    pub mod item;
    pub mod player_creation;
}
pub mod session {
    pub mod opcode_handler;
    pub mod session_holder;
    pub mod world_session;
    pub mod world_socket;
}
mod shared {
    pub mod constants;
    pub mod response_codes;
}

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

impl<T> From<SendError<T>> for WorldSocketError {
    fn from(_value: SendError<T>) -> Self {
        Self::SocketError(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "mpsc channel",
        ))
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
        session_holder: Arc<SessionHolder>,
    ) -> Result<WorldSocketState<ServerSentAuthResponse>, WorldSocketError> {
        let auth_session_client_message = self.read_socket_plain().await?;
        let payload_clone = auth_session_client_message.payload.clone();

        let mut reader = Cursor::new(auth_session_client_message.payload);
        let cmsg_auth_session: CmsgAuthSession = reader.read_le()?;

        let addon_infos_offset_in_cmsg = cmsg_auth_session.len();
        let addon_infos_raw = &payload_clone[addon_infos_offset_in_cmsg..];
        let addon_infos = Self::extract_addon_infos(addon_infos_raw);

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
                cmsg_auth_session.client_proof,
                cmsg_auth_session.client_seed,
            )
            .unwrap();

        let session = WorldSession::new(
            self.state.socket,
            encryption,
            account_id,
            self.state.world_context.clone(),
        );

        let packet = ServerMessage::new(SmsgAuthResponse {
            result: ResponseCodes::AuthOk as u8,
            billing_time: 0,
            billing_flags: 0,
            billing_rested: 0,
            expansion: 1,
            position_in_queue: 0,
        });

        session.send(&packet).unwrap();

        let packet = ServerMessage::new(Self::build_addon_infos(addon_infos));

        session.send(&packet).unwrap();

        if let Some(previous_session) = session_holder.insert_session(session) {
            previous_session
                .shutdown(&mut self.state.world_context.database.characters.get().unwrap());
        }

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

    fn extract_addon_infos(raw_data: &[u8]) -> Vec<ClientAddonInfo> {
        let mut reader = Cursor::new(&raw_data[..4]);
        let uncompressed_size: u32 = reader.read_le().unwrap();

        let compressed_data = &raw_data[4..].to_vec();
        let data = miniz_oxide::inflate::decompress_to_vec_zlib(&compressed_data).unwrap();

        let mut offset: usize = 0;
        let mut infos: Vec<ClientAddonInfo> = Vec::new();
        while offset < uncompressed_size as usize {
            let slice: &[u8] = &data[offset..];
            let mut reader = Cursor::new(slice);
            let info: ClientAddonInfo = reader.read_le().unwrap();
            offset += info.len();
            infos.push(info);
        }

        infos
    }

    fn build_addon_infos(infos: Vec<ClientAddonInfo>) -> SmsgAddonInfo {
        let addon_infos: Vec<ServerAddonInfo> = infos
            .into_iter()
            .map(|client_addon_info| {
                let use_public_key: bool = client_addon_info.crc != STANDARD_ADDON_CRC;
                let public_key = if use_public_key {
                    Some(ADDON_PUBLIC_KEY)
                } else {
                    None
                };

                ServerAddonInfo {
                    state: 2,
                    use_crc_or_public_key: true,
                    use_public_key,
                    public_key,
                    unk: Some(0),
                    use_url: false,
                }
            })
            .collect();

        SmsgAddonInfo { addon_infos }
    }
}

pub async fn process(
    socket: TcpStream,
    world_context: Arc<WorldContext>,
    session_holder: Arc<SessionHolder>,
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
        WorldSession::process_incoming_packet(session).await?;
    }
}
