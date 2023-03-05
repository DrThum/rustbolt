use packets::CmdAuthLogonChallengeClient;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

use crate::packets::CmdAuthLogonChallengeServer;

mod packets;

#[derive(PartialEq)]
enum AuthState {
    Init,
    LogonChallenge,
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
            Ok(_) if state == AuthState::Init => {
                let cmd_auth_logon_challenge_client = CmdAuthLogonChallengeClient::new(&buf);
                println!("{:?}", cmd_auth_logon_challenge_client);
                let cmd_auth_logon_challenge_server =
                    CmdAuthLogonChallengeServer::new(&cmd_auth_logon_challenge_client.account_name);
                cmd_auth_logon_challenge_server
                    .write(&mut socket)
                    .await
                    .unwrap(); // FIXME
                println!("sent auth logon challenge (server)");
                state = AuthState::LogonChallenge;
            }
            Ok(_) if state == AuthState::LogonChallenge => {
                println!("received {:?}", &buf);
            }
            Ok(_) => {
                println!("received unexpected {:?}", &buf);
            }
            Err(_) => {
                println!("Socket error, closing");
                return;
            }
        }
    }
}
