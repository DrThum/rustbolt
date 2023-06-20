use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::protocol::server::ServerMessagePayload;
use crate::{protocol::opcodes::Opcode, shared::constants::InventoryType};

#[binwrite]
pub struct CharEnumData {
    pub guid: u64,
    pub name: NullString,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hairstyle: u8,
    pub haircolor: u8,
    pub facialstyle: u8,
    pub level: u8,
    pub zone: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: u32,
    pub flags: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub first_login: bool,
    pub pet_display_id: u32,
    pub pet_level: u32,
    pub pet_family: u32,
    pub equipment: Vec<CharEnumEquip>,
}

#[binwrite]
pub struct CharEnumEquip {
    pub display_id: u32,
    pub slot: u8,
    pub enchant_id: u32,
}

impl CharEnumEquip {
    pub fn none(inv_type: InventoryType) -> CharEnumEquip {
        // TEMP until we implement gear persistence
        CharEnumEquip {
            display_id: 0,
            slot: inv_type as u8,
            enchant_id: 0,
        }
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgCharEnum {
    pub number_of_characters: u8,
    pub character_data: Vec<CharEnumData>,
}

#[binread]
pub struct CmsgRealmSplit {
    pub client_state: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgRealmSplit {
    pub client_state: u32,
    pub realm_state: u32, // 0x0 - normal; 0x1: realm split; 0x2 realm split pending
    pub split_date: NullString, // "01/01/01"
}

#[binread]
#[derive(Debug)]
pub struct CmsgCharCreate {
    pub name: NullString,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hairstyle: u8,
    pub haircolor: u8,
    pub facialstyle: u8,
}

#[binwrite]
#[server_opcode]
pub struct SmsgCharCreate {
    pub result: u8, // https://github.com/mangosone/server/blob/d62fdfe93b96bef5daa36433116d2f0eeb9fc3d0/src/game/Server/SharedDefines.h#L250
}

#[binread]
pub struct CmsgCharDelete {
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgCharDelete {
    pub result: u8, // Enum see SmsgCharCreate
}

#[binread]
pub struct CmsgPlayerLogin {
    pub guid: u64,
}
