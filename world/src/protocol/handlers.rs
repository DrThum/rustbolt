use binrw::io::Cursor;
use binrw::BinWriterExt;
use futures::future::{BoxFuture, FutureExt};
use lazy_static::lazy_static;
use log::{error, trace};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpStream;
use tokio::{io::AsyncWriteExt, sync::Mutex};
use wow_srp::tbc_header::HeaderCrypto;

use crate::protocol::packets::SmsgCharEnum;

use super::opcodes::Opcode;

type PacketHandler = Box<
    dyn Send
        + Sync
        + Fn(Vec<u8>, Arc<Mutex<HeaderCrypto>>, Arc<Mutex<TcpStream>>) -> BoxFuture<'static, ()>,
>;
lazy_static! {
    static ref HANDLERS: HashMap<u32, PacketHandler> = HashMap::from([
        (
            Opcode::MsgNullAction as u32,
            Box::new(
                |data, crypto: Arc<Mutex<HeaderCrypto>>, socket: Arc<Mutex<TcpStream>>| {
                    unhandled(data, crypto, socket).boxed()
                }
            ) as PacketHandler
        ),
        (
            Opcode::CmsgCharEnum as u32,
            Box::new(
                |data, crypto: Arc<Mutex<HeaderCrypto>>, socket: Arc<Mutex<TcpStream>>| {
                    handle_cmsg_char_enum(data, crypto, socket).boxed()
                }
            ) as PacketHandler
        )
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
    let mut encrypted_header: Vec<u8> = Vec::new();
    let mut encryption = encryption.lock().await;
    encryption
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
        character_first_bag_display_id: 0,     // Always 0
        character_first_bag_inventory_type: 0, // Always 0
        unk_0: 0,
    };

    let mut writer = Cursor::new(Vec::new());
    writer.write_le(&smsg_char_enum).unwrap();
    let mut socket = socket.lock().await;
    socket.write(writer.get_ref()).await.unwrap();
    trace!("Sent SMSG_CHAR_ENUM");
}

async fn unhandled(
    _data: Vec<u8>,
    _encryption: Arc<Mutex<HeaderCrypto>>,
    _socket: Arc<Mutex<TcpStream>>,
) {
}
