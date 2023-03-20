use binrw::io::Cursor;
use binrw::BinReaderExt;
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use wow_srp::tbc_header::HeaderCrypto;

use crate::protocol::packets::{CmsgPing, CmsgRealmSplit, SmsgCharEnum, SmsgPong, SmsgRealmSplit};
use crate::protocol::server::ServerMessage;

use super::opcodes::Opcode;

macro_rules! define_handler {
    ($opcode:expr, $handler:expr) => {
        (
            $opcode as u32,
            Box::new(
                |data, crypto: Arc<Mutex<HeaderCrypto>>, socket: Arc<Mutex<TcpStream>>| {
                    $handler(data, crypto, socket).boxed()
                },
            ) as PacketHandler,
        )
    };
}

type PacketHandler = Box<
    dyn Send
        + Sync
        + Fn(Vec<u8>, Arc<Mutex<HeaderCrypto>>, Arc<Mutex<TcpStream>>) -> BoxFuture<'static, ()>,
>;
lazy_static! {
    static ref HANDLERS: HashMap<u32, PacketHandler> = HashMap::from([
        define_handler!(Opcode::MsgNullAction, unhandled),
        define_handler!(Opcode::CmsgCharEnum, handle_cmsg_char_enum),
        define_handler!(Opcode::CmsgPing, handle_cmsg_ping),
        define_handler!(Opcode::CmsgRealmSplit, handle_cmsg_realm_split),
    ]);
}

pub fn get_handler(opcode: u32) -> &'static PacketHandler {
    HANDLERS.get(&opcode).unwrap_or_else(|| {
        error!("Received unhandled opcode {:#X}", opcode);
        HANDLERS.get(&(Opcode::MsgNullAction as u32)).unwrap()
    })
}

async fn handle_cmsg_char_enum(
    _data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    socket: Arc<Mutex<TcpStream>>,
) {
    trace!("Received CMSG_CHAR_ENUM");
    // TEMP: Send SMSG_CHAR_ENUM with a level 70 T6 undead priest

    let packet = ServerMessage::new(SmsgCharEnum {
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
        character_first_bag_display_id: 0,     // Always 0
        character_first_bag_inventory_type: 0, // Always 0
        unk_0: 0,
    });

    let mut socket = socket.lock().await;
    let mut encryption = encryption.lock().await;
    packet.send(&mut socket, &mut encryption).await.unwrap();
    trace!("Sent SMSG_CHAR_ENUM");
}

async fn unhandled(
    _data: Vec<u8>,
    _encryption: Arc<Mutex<HeaderCrypto>>,
    _socket: Arc<Mutex<TcpStream>>,
) {
}

async fn handle_cmsg_realm_split(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    socket: Arc<Mutex<TcpStream>>,
) {
    trace!("Received CMSG_REALM_SPLIT");
    let mut reader = Cursor::new(data);
    let cmsg_realm_split: CmsgRealmSplit = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgRealmSplit {
        client_state: cmsg_realm_split.client_state,
        realm_state: 0x00,
        split_date: binrw::NullString::from("01/01/01"),
    });

    let mut socket = socket.lock().await;
    let mut encryption = encryption.lock().await;
    packet.send(&mut socket, &mut encryption).await.unwrap();
    trace!("Sent SMSG_REALM_SPLIT");
}

async fn handle_cmsg_ping(
    data: Vec<u8>,
    encryption: Arc<Mutex<HeaderCrypto>>,
    socket: Arc<Mutex<TcpStream>>,
) {
    trace!("Received CMSG_PING");
    let mut reader = Cursor::new(data);
    let cmsg_ping: CmsgPing = reader.read_le().unwrap();

    let packet = ServerMessage::new(SmsgPong {
        ping: cmsg_ping.ping,
    });

    let mut socket = socket.lock().await;
    let mut encryption = encryption.lock().await;
    packet.send(&mut socket, &mut encryption).await.unwrap();
    trace!("Sent SMSG_PONG");
}
