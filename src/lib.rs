use std::sync::Arc;

use crate::packets::{
    CmdAuthLogonChallengeClient, CmdAuthLogonChallengeServer, CmdAuthLogonProofClient,
    CmdAuthLogonProofServer, CmdRealmListClient, CmdRealmListServer,
};
pub use crate::packets::{Realm, RealmType};
use binrw::io::Cursor;
use binrw::{BinReaderExt, BinWriterExt};

use log::{error, trace};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wow_srp::server::SrpProof;

mod packets;

#[derive(PartialEq)]
enum AuthState {
    Init,
    LogonChallenge(SrpProof),
    LogonProof,
}

pub async fn process(mut socket: TcpStream, realms: Arc<Vec<Realm>>) -> Result<(), binrw::Error> {
    let mut buf = [0_u8; 1024];
    let mut state = AuthState::Init;

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                trace!("Client disconnected");
                return Ok(());
            }
            Ok(_) => match state {
                AuthState::Init => {
                    let mut reader = Cursor::new(buf);
                    let cmd_auth_logon_challenge_client: CmdAuthLogonChallengeClient =
                        reader.read_le()?;
                    trace!("Received {:?}", cmd_auth_logon_challenge_client);

                    let (cmd_auth_logon_challenge_server, proof) = CmdAuthLogonChallengeServer::new(
                        &cmd_auth_logon_challenge_client.account_name,
                    );

                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&cmd_auth_logon_challenge_server)?;
                    socket.write(writer.get_ref()).await?;
                    trace!("Sent auth logon challenge (server)");

                    state = AuthState::LogonChallenge(proof);
                }
                AuthState::LogonChallenge(proof) => {
                    let mut reader = Cursor::new(buf);
                    let cmd_auth_logon_proof_client: CmdAuthLogonProofClient = reader.read_le()?;
                    trace!("Received {:?}", cmd_auth_logon_proof_client);

                    let cmd_auth_logon_proof_server =
                        CmdAuthLogonProofServer::new(cmd_auth_logon_proof_client, proof);

                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&cmd_auth_logon_proof_server)?;
                    socket.write(writer.get_ref()).await?;
                    trace!("Sent auth logon proof (server)");

                    state = AuthState::LogonProof;
                }
                AuthState::LogonProof => {
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
                    socket.write(writer.get_ref()).await?;
                    trace!("Sent realm list (server)");
                }
            },
            Err(_) => {
                error!("Socket error, closing");
                return Ok(());
            }
        }
    }
}
