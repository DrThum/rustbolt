use crate::protocol::client::ClientMessage;
use crate::protocol::server::ServerMessage;
use std::sync::Arc;

use crate::protocol::packets::{CmsgAuthSession, SmsgAuthChallenge, SmsgAuthResponse};
use binrw::io::Cursor;
use binrw::BinReaderExt;
use hex::FromHex;
use log::{error, trace};
use protocol::client::ClientMessageHeader;
use protocol::handlers;
use rusqlite::Connection;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use world_session::WorldSession;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::tbc_header::{HeaderCrypto, ProofSeed};

mod constants;
mod entities;
mod protocol;
pub mod world_session;

// TypeState pattern (https://yoric.github.io/post/rust-typestate/)
struct SocketOpened;
struct ServerSentAuthChallenge {
    seed: ProofSeed,
}
struct ServerSentAuthResponse {
    encryption: Arc<Mutex<HeaderCrypto>>,
}

struct WorldSocketState<S> {
    session: Arc<WorldSession>,
    _state: S,
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

impl<S> WorldSocketState<S> {
    async fn read_socket_plain(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket_guard = self.session.socket.lock().await;
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
}

impl WorldSocketState<SocketOpened> {
    async fn send_challenge(
        self,
    ) -> Result<WorldSocketState<ServerSentAuthChallenge>, WorldSocketError> {
        let seed = ProofSeed::new();
        let packet = ServerMessage::new(SmsgAuthChallenge {
            server_seed: seed.seed(),
        });
        let mut socket = self.session.socket.lock().await;
        packet.send_unencrypted(&mut socket).await?;
        drop(socket);
        trace!("Sent SMSG_AUTH_CHALLENGE");

        Ok(WorldSocketState {
            session: self.session,
            _state: ServerSentAuthChallenge { seed },
        })
    }
}

impl WorldSocketState<ServerSentAuthChallenge> {
    async fn handle_auth_session(
        mut self,
    ) -> Result<WorldSocketState<ServerSentAuthResponse>, WorldSocketError> {
        let auth_session_client_message = self.read_socket_plain().await?;

        let mut reader = Cursor::new(auth_session_client_message.payload);
        let cmsg_auth_session: CmsgAuthSession = reader.read_le()?;
        let username: String = cmsg_auth_session._username.to_string();
        let username: NormalizedString = NormalizedString::new(username).unwrap();

        let mut conn = self.session.db_pool_auth.get()?;
        let session_key = fetch_session_key(&mut conn, username.to_string()).unwrap();
        let session_key: [u8; 40] = <Vec<u8>>::from_hex(session_key)
            .unwrap()
            .try_into()
            .unwrap();

        let encryption = self
            ._state
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
            result: 0x0C, // AUTH_OK
            _billing_time: 0,
            _billing_flags: 0,
            _billing_rested: 0,
        });

        packet.send(&self.session.socket, &encryption).await?;
        trace!("Sent SMSG_AUTH_RESPONSE");

        Ok(WorldSocketState {
            session: self.session,
            _state: ServerSentAuthResponse { encryption },
        })
    }
}

impl WorldSocketState<ServerSentAuthResponse> {
    async fn read_socket(&mut self) -> Result<ClientMessage, WorldSocketError> {
        let mut buf = [0_u8; 6];
        let mut socket = self.session.socket.lock().await;
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
                let mut encryption = self._state.encryption.lock().await;
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

        handler(
            client_message.payload,
            Arc::clone(&self._state.encryption),
            Arc::clone(&self.session),
        )
        .await;

        Ok(())
    }
}

pub async fn process(session: Arc<WorldSession>) -> Result<(), WorldSocketError> {
    let mut state = WorldSocketState {
        session,
        _state: SocketOpened,
    }
    .send_challenge()
    .await?
    .handle_auth_session()
    .await?;

    loop {
        state.handle_packet().await?;
    }
}

fn fetch_session_key(conn: &mut Connection, username: String) -> Option<String> {
    let mut stmt = conn
        .prepare("SELECT session_key FROM accounts WHERE UPPER(username) = :username")
        .unwrap();
    let mut rows = stmt.query(&[(":username", &username)]).unwrap();

    if let Some(row) = rows.next().unwrap() {
        Some(row.get("session_key").unwrap())
    } else {
        None
    }
}
