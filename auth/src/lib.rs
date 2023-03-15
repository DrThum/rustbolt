use std::sync::Arc;

use crate::packets::{
    CmdAuthLogonChallengeClient, CmdAuthLogonChallengeServer, CmdAuthLogonProofClient,
    CmdAuthLogonProofServer, CmdRealmListClient, CmdRealmListServer,
};
pub use crate::packets::{Realm, RealmType};
use binrw::io::Cursor;
use binrw::{BinReaderExt, BinWriterExt};

use hex::ToHex;
use log::{error, trace};
use rusqlite::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wow_srp::server::SrpProof;

mod packets;

// TypeState pattern (https://yoric.github.io/post/rust-typestate/)
struct SocketOpened;
struct ServerSentLogonChallenge {
    proof: SrpProof,
}
struct ClientAuthenticated;

pub struct AuthState<S> {
    socket: TcpStream,
    _state: S,
}

// Error types to be moved to another file
#[derive(Debug)]
pub enum AuthError {
    ClientDisconnected,
    SocketError(std::io::Error),
    BinRwError(binrw::Error),
}

impl From<std::io::Error> for AuthError {
    fn from(value: std::io::Error) -> Self {
        Self::SocketError(value)
    }
}

impl From<binrw::Error> for AuthError {
    fn from(value: binrw::Error) -> Self {
        Self::BinRwError(value)
    }
}

impl<S> AuthState<S> {
    async fn read_socket(&mut self) -> Result<[u8; 1024], AuthError> {
        let mut buf = [0_u8; 1024];
        match self.socket.read(&mut buf).await {
            Ok(0) => {
                trace!("Client disconnected");
                return Err(AuthError::ClientDisconnected);
            }
            Ok(_) => Ok(buf),
            Err(e) => {
                error!("Socket error, closing");
                return Err(AuthError::SocketError(e));
            }
        }
    }
}

impl AuthState<SocketOpened> {
    async fn handle_challenge(mut self) -> Result<AuthState<ServerSentLogonChallenge>, AuthError> {
        let buf = self.read_socket().await?;

        let mut reader = Cursor::new(buf);
        let cmd_auth_logon_challenge_client: CmdAuthLogonChallengeClient = reader.read_le()?;
        trace!("Received {:?}", cmd_auth_logon_challenge_client);

        let (cmd_auth_logon_challenge_server, proof) =
            CmdAuthLogonChallengeServer::new(&cmd_auth_logon_challenge_client.account_name);

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&cmd_auth_logon_challenge_server)?;
        self.socket.write(writer.get_ref()).await?;
        trace!("Sent auth logon challenge (server)");

        let new_state = AuthState {
            socket: self.socket,
            _state: ServerSentLogonChallenge { proof },
        };
        Ok(new_state)
    }
}

impl AuthState<ServerSentLogonChallenge> {
    async fn handle_proof(
        mut self,
        conn: &mut Connection,
    ) -> Result<AuthState<ClientAuthenticated>, AuthError> {
        let buf = self.read_socket().await?;

        let mut reader = Cursor::new(buf);
        let cmd_auth_logon_proof_client: CmdAuthLogonProofClient = reader.read_le()?;
        trace!("Received {:?}", cmd_auth_logon_proof_client);

        let (cmd_auth_logon_proof_server, server_proof) =
            CmdAuthLogonProofServer::new(cmd_auth_logon_proof_client, self._state.proof);

        // Save the session key to the database
        save_session_key(conn, server_proof.session_key().encode_hex::<String>()).unwrap();

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&cmd_auth_logon_proof_server)?;
        self.socket.write(writer.get_ref()).await?;
        trace!("Sent auth logon proof (server)");

        let new_state = AuthState {
            socket: self.socket,
            _state: ClientAuthenticated,
        };
        Ok(new_state)
    }
}

impl AuthState<ClientAuthenticated> {
    async fn handle_realm_list(&mut self, realms: &Arc<Vec<Realm>>) -> Result<(), AuthError> {
        let buf = self.read_socket().await?;

        let mut reader = Cursor::new(buf);
        let cmd_realm_list_client: CmdRealmListClient = reader.read_le()?;
        trace!("Received {:?}", cmd_realm_list_client);

        let cmd_realm_list_server = CmdRealmListServer {
            _opcode: packets::Opcode::CmdRealmList,
            _size: 8 + realms.iter().fold(0, |acc, r| acc + r.size()),
            _padding: 0,
            _num_realms: realms.len().try_into().unwrap(),
            _realms: &*realms,
            _padding_footer: 0,
        };

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&cmd_realm_list_server)?;
        self.socket.write(writer.get_ref()).await?;
        trace!("Sent realm list (server)");

        Ok(())
    }
}

// TODO: Improve with https://docs.rs/wow_srp/latest/wow_srp/server/index.html
pub async fn process(socket: TcpStream, realms: Arc<Vec<Realm>>) -> Result<(), AuthError> {
    // TODO: Don't open one connection per socket
    let mut conn = Connection::open("./data/databases/auth.db").unwrap();

    let mut authenticated_state = AuthState {
        socket,
        _state: SocketOpened,
    }
    .handle_challenge()
    .await?
    .handle_proof(&mut conn)
    .await?;

    loop {
        authenticated_state.handle_realm_list(&realms).await?;
    }
}

fn save_session_key(conn: &mut Connection, session_key: String) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare_cached("UPDATE accounts SET session_key = ? WHERE username = 'a'")?;
    stmt.execute([session_key])?;

    Ok(())
}
