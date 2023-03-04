use packets::CmdAuthLogonChallenge;

use tokio::io::AsyncReadExt;
use tokio::net::TcpStream;

mod packets;

#[derive(PartialEq)]
enum AuthState {
    Init,
    LogonChallenge
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
                let cmd_auth_logon_challenge = CmdAuthLogonChallenge::new(&buf);
                println!("{:?}", cmd_auth_logon_challenge);
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
