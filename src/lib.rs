use crate::packets::{
    CmdAuthLogonChallengeClient, CmdAuthLogonChallengeServer, CmdAuthLogonProofClient,
    CmdAuthLogonProofServer, CmdRealmListClient, CmdRealmListServer, Realm, RealmFlag, RealmType,
    RealmZone,
};
use binrw::io::Cursor;
use binrw::{BinReaderExt, BinWriterExt};

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
    let mut buf = [0_u8; 1024];
    let mut state = AuthState::Init;

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                println!("Socket closed");
                return;
            }
            Ok(_) => match state {
                AuthState::Init => {
                    let mut reader = Cursor::new(buf);
                    let cmd_auth_logon_challenge_client: CmdAuthLogonChallengeClient =
                        reader.read_le().unwrap();
                    println!("{:?}", cmd_auth_logon_challenge_client);
                    let (cmd_auth_logon_challenge_server, proof) = CmdAuthLogonChallengeServer::new(
                        &cmd_auth_logon_challenge_client.account_name,
                    );

                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&cmd_auth_logon_challenge_server).unwrap();
                    socket.write(writer.get_ref()).await.unwrap();
                    println!("sent auth logon challenge (server)");
                    state = AuthState::LogonChallenge(proof);
                }
                AuthState::LogonChallenge(proof) => {
                    let mut reader = Cursor::new(buf);
                    let cmd_auth_logon_proof_client: CmdAuthLogonProofClient =
                        reader.read_le().unwrap();
                    println!("{:?}", cmd_auth_logon_proof_client);
                    let cmd_auth_logon_proof_server =
                        CmdAuthLogonProofServer::new(cmd_auth_logon_proof_client, proof);

                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&cmd_auth_logon_proof_server).unwrap();
                    socket.write(writer.get_ref()).await.unwrap();
                    println!("sent auth logon proof (server)");
                    state = AuthState::LogonProof;
                }
                AuthState::LogonProof => {
                    let mut reader = Cursor::new(buf);
                    let cmd_realm_list_client: CmdRealmListClient = reader.read_le().unwrap();
                    println!("{:?}", cmd_realm_list_client);

                    let realm1 = Realm {
                        _realm_type: RealmType::PvP,
                        _locked: false,
                        _realm_flags: vec![RealmFlag::ForceNewPlayers, RealmFlag::Invalid],
                        _realm_name: "Rustbolt".into(),
                        _address_port: "127.0.0.1:8085".into(),
                        _population: 200_f32,
                        _num_chars: 1,
                        _realm_category: RealmZone::French,
                        _realm_id: 1,
                    };

                    let realm2 = Realm {
                        _realm_type: RealmType::RolePlay,
                        _locked: true,
                        _realm_flags: vec![RealmFlag::Offline],
                        _realm_name: "Rustbolt RP".into(),
                        _address_port: "127.0.0.1:8085".into(),
                        _population: 400_f32,
                        _num_chars: 5,
                        _realm_category: RealmZone::German,
                        _realm_id: 2,
                    };
                    let realms = vec![realm1, realm2];

                    let cmd_realm_list_server = CmdRealmListServer {
                        _opcode: packets::Opcode::CmdRealmList,
                        _size: 8 + realms.iter().fold(0, |acc, r| acc + r.size()),
                        _padding: 0,
                        _num_realms: realms.len().try_into().unwrap(),
                        _realms: realms,
                        _padding_footer: 0,
                    };

                    let mut writer = Cursor::new(Vec::new());
                    writer.write_le(&cmd_realm_list_server).unwrap();
                    socket.write(writer.get_ref()).await.unwrap();
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
