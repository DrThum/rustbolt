use crate::packets::{
    CmdAuthLogonChallengeClient, CmdAuthLogonChallengeServer, CmdAuthLogonProofClient,
    CmdAuthLogonProofServer,
};

use tokio::io::AsyncReadExt;
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
                    println!("received {:?}", &buf);
                } /*                 _ => {
                      println!("received unexpected {:?}", &buf);
                  } */
            },
            Err(_) => {
                println!("Socket error, closing");
                return;
            }
        }
    }
}
