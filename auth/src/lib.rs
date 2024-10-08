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
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wow_srp::server::SrpProof;

pub mod config;
mod packets;

// TypeState pattern (https://yoric.github.io/post/rust-typestate/)
struct SocketOpened;
struct ServerSentLogonChallenge {
    proof: SrpProof,
    account_name: String,
}
struct ClientAuthenticated;

struct AuthState<S> {
    socket: TcpStream,
    state: S,
}

// Error types to be moved to another file
#[derive(Debug)]
pub enum AuthError {
    ClientDisconnected,
    SocketError(std::io::Error),
    BinRwError(binrw::Error),
    DbError(r2d2::Error),
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

impl From<r2d2::Error> for AuthError {
    fn from(value: r2d2::Error) -> Self {
        Self::DbError(value)
    }
}

impl<S> AuthState<S> {
    async fn read_socket(&mut self) -> Result<[u8; 1024], AuthError> {
        let mut buf = [0_u8; 1024];
        match self.socket.read(&mut buf).await {
            Ok(0) => {
                trace!("Client disconnected");
                Err(AuthError::ClientDisconnected)
            }
            Ok(_) => Ok(buf),
            Err(e) => {
                error!("Socket error, closing");
                Err(AuthError::SocketError(e))
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
        self.socket.write_all(writer.get_ref()).await?;
        trace!("Sent auth logon challenge (server)");

        let new_state = AuthState {
            socket: self.socket,
            state: ServerSentLogonChallenge {
                proof,
                account_name: cmd_auth_logon_challenge_client.account_name.to_string(),
            },
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
            CmdAuthLogonProofServer::new(cmd_auth_logon_proof_client, self.state.proof);

        // Save the session key to the database
        save_session_key(
            conn,
            self.state.account_name,
            server_proof.session_key().encode_hex::<String>(),
        )
        .unwrap();

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&cmd_auth_logon_proof_server)?;
        self.socket.write_all(writer.get_ref()).await?;
        trace!("Sent auth logon proof (server)");

        let new_state = AuthState {
            socket: self.socket,
            state: ClientAuthenticated,
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
            opcode: packets::Opcode::CmdRealmList,
            size: 8 + realms.iter().fold(0, |acc, r| acc + r.size()),
            padding: 0,
            num_realms: realms.len().try_into().unwrap(),
            realms,
            padding_footer: 0,
        };

        let mut writer = Cursor::new(Vec::new());
        writer.write_le(&cmd_realm_list_server)?;
        self.socket.write_all(writer.get_ref()).await?;
        trace!("Sent realm list (server)");

        Ok(())
    }
}

// TODO: Improve with https://docs.rs/wow_srp/latest/wow_srp/server/index.html
pub async fn process(
    socket: TcpStream,
    realms: Arc<Vec<Realm>>,
    db_pool: Arc<Pool<SqliteConnectionManager>>,
) -> Result<(), AuthError> {
    let mut conn = db_pool.get()?;
    let mut authenticated_state = AuthState {
        socket,
        state: SocketOpened,
    }
    .handle_challenge()
    .await?
    .handle_proof(&mut conn)
    .await?;

    loop {
        authenticated_state.handle_realm_list(&realms).await?;
    }
}

fn save_session_key(
    conn: &mut Connection,
    account_name: String,
    session_key: String,
) -> Result<(), rusqlite::Error> {
    let mut stmt =
        conn.prepare_cached("UPDATE accounts SET session_key = ? WHERE UPPER(username) = ?")?;
    stmt.execute([session_key, account_name])?;

    Ok(())
}
