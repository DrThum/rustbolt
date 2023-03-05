use crate::packets::{
    CmdAuthLogonChallengeClient, CmdAuthLogonChallengeServer, CmdAuthLogonProofClient,
    CmdAuthLogonProofServer,
};

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

pub async fn process(mut socket: TcpStream) {
    let mut buf = vec![0; 1024];
    let mut state = AuthState::Init;

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                println!("Socket closed");
                return;
            }
            Ok(_) => match state {
                AuthState::Init => {
                    let cmd_auth_logon_challenge_client = CmdAuthLogonChallengeClient::new(&buf);
                    println!("{:?}", cmd_auth_logon_challenge_client);
                    let (cmd_auth_logon_challenge_server, proof) = CmdAuthLogonChallengeServer::new(
                        &cmd_auth_logon_challenge_client.account_name,
                    );
                    cmd_auth_logon_challenge_server
                        .write(&mut socket)
                        .await
                        .unwrap(); // FIXME
                    println!("sent auth logon challenge (server)");
                    state = AuthState::LogonChallenge(proof);
                }
                AuthState::LogonChallenge(proof) => {
                    let cmd_auth_logon_proof_client = CmdAuthLogonProofClient::new(&buf);
                    println!("{:?}", cmd_auth_logon_proof_client);
                    let cmd_auth_logon_proof_server =
                        CmdAuthLogonProofServer::new(cmd_auth_logon_proof_client, proof);
                    cmd_auth_logon_proof_server
                        .write(&mut socket)
                        .await
                        .unwrap();
                    println!("sent auth logon proof (server)");
                    state = AuthState::LogonProof;
                }
                AuthState::LogonProof => {
                    println!("received realm list (client)");
                    // TEMP: refactor with CmdRealmListClient and CmdRealmListServer
                    socket.write(&0x10_u8.to_le_bytes()).await.unwrap(); // opcode
                    socket.write(&42_u16.to_le_bytes()).await.unwrap(); // TODO: calculate size
                    socket.write(&0_u32.to_le_bytes()).await.unwrap(); // padding
                    socket.write(&1_u16.to_le_bytes()).await.unwrap(); // num_realms
                    socket.write(&1_u8.to_le_bytes()).await.unwrap(); // realm_type
                    socket.write(&0_u8.to_le_bytes()).await.unwrap(); // locked
                    socket.write(&0x40_u8.to_le_bytes()).await.unwrap(); // realm flags
                    socket.write("Rustbolt\0".as_bytes()).await.unwrap();
                    socket.write("127.0.0.1:8085\0".as_bytes()).await.unwrap();
                    socket.write(&200_f32.to_le_bytes()).await.unwrap(); // population
                    socket.write(&1_u8.to_le_bytes()).await.unwrap(); // num_chars
                    socket.write(&3_u8.to_le_bytes()).await.unwrap(); // locale
                    socket.write(&1_u8.to_le_bytes()).await.unwrap(); // realm_id
                    socket.write(&0_u16.to_le_bytes()).await.unwrap(); // padding
                    println!("sent realm list (server)");
                }
            },
            Err(_) => {
                println!("Socket error, closing");
                return;
            }
        }
    }
}
