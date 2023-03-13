use binrw::io::Cursor;
use binrw::{BinReaderExt, BinWriterExt};
use hex::FromHex;
use log::{debug, error, trace};
use packets::SmsgAuthChallenge;
use rusqlite::Connection;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wow_srp::normalized_string::NormalizedString;
use wow_srp::tbc_header::{HeaderCrypto, ProofSeed};

use crate::packets::{CmsgAuthSession, SmsgAuthResponse, SmsgCharEnum};

mod packets;

pub async fn process(mut socket: TcpStream) -> Result<(), binrw::Error> {
    // Send SMSG_AUTH_CHALLENGE
    let seed = ProofSeed::new();
    let smsg_auth_challenge = SmsgAuthChallenge {
        _size: 6,
        _opcode: 0x1EC,
        _server_seed: seed.seed(),
    };

    let mut status = 0; // TODO: enum

    let mut writer = Cursor::new(Vec::new());
    writer.write_le(&smsg_auth_challenge)?;
    socket.write(writer.get_ref()).await?;
    trace!("Send SMSG_AUTH_CHALLENGE");

    // TODO: Don't open one connection per socket
    let mut conn = Connection::open("./data/databases/auth.db").unwrap();
    let session_key = fetch_session_key(&mut conn).unwrap();
    debug!("session key {}", session_key);
    let session_key: [u8; 40] = <Vec<u8>>::from_hex(session_key)
        .unwrap()
        .try_into()
        .unwrap();

    let mut buf = [0_u8; 1024];
    let mut encryption: Option<HeaderCrypto> = None;

    loop {
        match socket.read(&mut buf).await {
            Ok(_) if status == 0 => {
                let mut reader = Cursor::new(buf);
                let cmsg_auth_session: CmsgAuthSession = reader.read_le()?;
                let username: String = cmsg_auth_session._username.to_string();
                let username: NormalizedString = NormalizedString::new(username).unwrap();
                debug!("Received {:?}", cmsg_auth_session);
                encryption = Some(
                    seed.into_header_crypto(
                        &username,
                        session_key,
                        cmsg_auth_session._client_proof,
                        cmsg_auth_session._client_seed,
                    )
                    .unwrap(),
                );
                // Send SMSG_AUTH_RESPONSE
                let mut encrypted_header: Vec<u8> = Vec::new();
                encryption
                    .as_mut()
                    .unwrap()
                    .write_encrypted_server_header(&mut encrypted_header, 12, 0x1EE)
                    .unwrap();

                let smsg_auth_response = SmsgAuthResponse {
                    _header: encrypted_header,
                    _result: 0x0C, // AUTH_OK
                    _billing_time: 0,
                    _billing_flags: 0,
                    _billing_rested: 0,
                };

                let mut writer = Cursor::new(Vec::new());
                writer.write_le(&smsg_auth_response)?;
                socket.write(writer.get_ref()).await?;
                debug!("Sent SMSG_AUTH_RESPONSE");
                status = 1;
            }
            Ok(n) if status == 1 && n == 16 => {
                // FIXME: 14 bytes = CMSG_PING, 16 bytes =
                // CMsG_CHAR_ENUM

                debug!("Received CMSG_CHAR_ENUM");
                // Send SMSG_CHAR_ENUM with a level 70 T6 undead priest
                let mut encrypted_header: Vec<u8> = Vec::new();
                encryption
                    .as_mut()
                    .unwrap()
                    .write_encrypted_server_header(&mut encrypted_header, 246, 0x3B)
                    .unwrap();
                let smsg_char_enum = SmsgCharEnum {
                    header: encrypted_header,
                    amount_of_characters: 1,
                    character_guid: 1,
                    character_name: "Thum".try_into().unwrap(),
                    character_race: 5,
                    character_class: 5,
                    character_gender: 0,
                    character_skin: 0,
                    character_face: 0,
                    character_hairstyle: 0,
                    character_haircolor: 0,
                    character_facialstyle: 0,
                    character_level: 70,
                    character_area: 85,
                    character_map: 0,
                    character_position_x: 0.0,
                    character_position_y: 0.0,
                    character_position_z: 0.0,
                    character_guild_id: 0,
                    character_flags: 0,
                    character_first_login: 1, // FIXME: bool
                    character_pet_display_id: 0,
                    character_pet_level: 0,
                    character_pet_family: 0,
                    character_equip_head: 45770,
                    character_equip_head_slot: 1,
                    character_equip_head_enchant: 0,
                    character_equip_neck: 0,
                    character_equip_neck_slot: 2,
                    character_equip_neck_enchant: 0,
                    character_equip_shoulders: 44978,
                    character_equip_shoulders_slot: 3, // 3
                    character_equip_shoulders_enchant: 0,
                    character_equip_body: 0,
                    character_equip_body_slot: 4, // 4
                    character_equip_body_enchant: 0,
                    character_equip_chest: 44979,
                    character_equip_chest_slot: 5, // 5
                    character_equip_chest_enchant: 0,
                    character_equip_waist: 45263,
                    character_equip_waist_slot: 6, // 6
                    character_equip_waist_enchant: 0,
                    character_equip_legs: 44968,
                    character_equip_legs_slot: 7, // 7
                    character_equip_legs_enchant: 0,
                    character_equip_feet: 45737,
                    character_equip_feet_slot: 8, // 8
                    character_equip_feet_enchant: 0,
                    character_equip_wrists: 45359,
                    character_equip_wrists_slot: 9, // 9
                    character_equip_wrists_enchant: 0,
                    character_equip_hands: 44975,
                    character_equip_hands_slot: 10, // 10
                    character_equip_hands_enchant: 0,
                    character_equip_finger1: 0,
                    character_equip_finger1_slot: 11, // 11
                    character_equip_finger1_enchant: 0,
                    character_equip_finger2: 0,
                    character_equip_finger2_slot: 11, // 11
                    character_equip_finger2_enchant: 0,
                    character_equip_trinket1: 0,
                    character_equip_trinket1_slot: 12, // 12
                    character_equip_trinket1_enchant: 0,
                    character_equip_trinket2: 0,
                    character_equip_trinket2_slot: 12, // 12
                    character_equip_trinket2_enchant: 0,
                    character_equip_back: 0,
                    character_equip_back_slot: 16, // 16
                    character_equip_back_enchant: 0,
                    character_equip_mainhand: 31346,
                    character_equip_mainhand_slot: 21, // 21
                    character_equip_mainhand_enchant: 0,
                    character_equip_offhand: 0,
                    character_equip_offhand_slot: 22, // 22
                    character_equip_offhand_enchant: 0,
                    character_equip_ranged: 0,
                    character_equip_ranged_slot: 26, // 26
                    character_equip_ranged_enchant: 0,
                    character_equip_tabard: 0,
                    character_equip_tabard_slot: 19, // 19
                    character_equip_tabard_enchant: 0,
                    character_first_bag_display_id: 0, // Always 0
                    character_first_bag_inventory_type: 0, // Always 0
                    unk_0: 0,
                };

                let mut writer = Cursor::new(Vec::new());
                writer.write_le(&smsg_char_enum)?;
                socket.write(writer.get_ref()).await?;
                debug!("Sent SMSG_CHAR_ENUM");
            }
            Ok(0) => {
                debug!("Socket closed (received 0 byte)");
                return Ok(());
            }
            Ok(n) => debug!("received {} bytes", n),
            Err(_) => {
                error!("Socket error, closing");
                return Ok(());
            }
        }
    }
}

fn fetch_session_key(conn: &mut Connection) -> Option<String> {
    let username = String::from("a");
    let mut stmt = conn
        .prepare("SELECT session_key FROM accounts WHERE username = :username")
        .unwrap();
    let mut rows = stmt.query(&[(":username", &username)]).unwrap();

    if let Some(row) = rows.next().unwrap() {
        Some(row.get("session_key").unwrap())
    } else {
        None
    }
}
