use crate::constants::InventoryType;

use super::opcodes::Opcode;
use super::server::ServerMessagePayload;
use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

#[binwrite]
#[server_opcode]
pub struct SmsgAuthChallenge {
    pub server_seed: u32,
}

#[binread]
#[derive(Debug)]
pub struct CmsgAuthSession {
    pub _build: u32,
    pub _server_id: u32,
    pub _username: NullString,
    pub _client_seed: u32,
    pub _client_proof: [u8; 20],
    pub _decompressed_addon_info_size: u32,
    // #[br(count = _size - (4 + 4 + 4 + (_username.len() - 1) + 4 + 20 + 4) as u16)]
    // pub _compressed_addon_info: Vec<u8>,
}

#[binwrite]
#[server_opcode]
pub struct SmsgAuthResponse {
    pub result: u8,
    pub _billing_time: u32,
    pub _billing_flags: u8,
    pub _billing_rested: u32,
}

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
    pub area: u32,
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub guild_id: u32,
    pub flags: u32,
    pub first_login: u8, // FIXME: bool
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
pub struct CmsgPing {
    pub ping: u32,
    _latency: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgPong {
    pub ping: u32,
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

#[binwrite]
#[server_opcode]
pub struct SmsgLoginVerifyWorld {
    pub map: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgTutorialFlags {
    pub tutorial_data0: u32,
    pub tutorial_data1: u32,
    pub tutorial_data2: u32,
    pub tutorial_data3: u32,
    pub tutorial_data4: u32,
    pub tutorial_data5: u32,
    pub tutorial_data6: u32,
    pub tutorial_data7: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgAccountDataTimes {
    pub data: [u8; 32], // All 0
}

#[binwrite]
#[server_opcode]
pub struct SmsgFeatureSystemStatus {
    pub unk: u8,
    pub voice_chat_enabled: u8, // 1 = enabled, 0 = disabled
}

#[binwrite]
#[server_opcode]
pub struct SmsgMotd {
    pub line_count: u32,
    pub message: NullString,
}

#[binwrite]
#[server_opcode]
pub struct SmsgUpdateObject {
    pub block_count: u32,
    pub has_transport: u8,    // FIXME: bool
    pub update_type: u8,      // CREATE_NEW_OBJECT2 = 3
    pub packed_guid_mask: u8, // FIXME
    pub packed_guid_guid: u8, // FIXME
    pub object_type: u8,      // PLAYER = 4
    pub flags: u8,            // 0x71 for now
    pub movement_flags: u32,
    pub movement_flags2: u8, // 0
    pub timestamp: u32,      // 0 for now
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub fall_time: u32,             // 0 for now
    pub speed_walk: f32,            // 1.0
    pub speed_run: f32,             // 70.0
    pub speed_run_backward: f32,    // 4.5
    pub speed_swim: f32,            // 0.0
    pub speed_swim_backward: f32,   // 0.0
    pub speed_flight: f32,          // 70.0 ?
    pub speed_flight_backward: f32, // 4.5 ?
    pub speed_turn: f32,            // PI (3.1415)
    pub unk_highguid: u32,
    pub num_mask_blocks: u8,
    pub mask_blocks: Vec<u32>,
    pub data: Vec<u32>,
}

/*
    pub object_field_guid: u64,
    pub object_field_type: u32, // 25 for now
    pub unit_field_scale: f32,  // 1.0f
    pub object_health: u32,     // 100 for now
    pub object_power: u32,      // mana, 23, 100
    pub object_max_health: u32, // 100 | 28
    pub object_max_power: u32,  // mana, 29, 100
    pub level: u32,             // 70
    pub faction_template: u32,  // 469
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub power: u8,
    pub bounding_radius: f32,   // 1.0f
    pub combat_reach: f32,      // 1.5f
    pub display_id: u32,        // 1478
    pub native_display_id: u32, // 1478
    pub base_attack_time_mainhand: f32,
    pub base_attack_time_offhand: f32,
    pub ranged_attack_time: f32,
    pub unit_mod_cast_speed: f32, // 1.0f
    pub unit_field_base_mana: u32,
    pub unit_field_base_health: u32,
    pub unit_field_bytes_2: u32, // 0x2800
    pub player_bytes: u32,       // see below
    pub player_bytes_2: u32,     // see below
    pub player_bytes_3: u32,     // gender
    pub player_xp: u32,          // 0
    pub player_field_max_level: u32, // 70

                                 // OBJECT_FIELD_SCALE_X: 1.0f - 4
                                 // UNIT_FIELD_LEVEL: 70 - 34
                                 // UNIT_FIELD_FACTIONTEMPLATE: 469 - 35
                                 // -- unit_bytes_0 here (race/class/gender/power)
                                 // UNIT_FIELD_BOUNDINGRADIUS: 1.0f ? - 96
                                 // UNIT_FIELD_COMBATREACH: 1.5f 97
                                 // UNIT_FIELD_DISPLAYID: 1478 - 98
                                 // UNIT_FIELD_NATIVEDISPLAYID: 1478 - 99
                                 // UNIT_FIELD_BASEATTACKTIME: 2000.f | 147
                                 // UNIT_FIELD_BASEATTACKTIME (offhand): 2000.f | 148
                                 // UNIT_FIELD_RANGEDATTACKTIME: 2000.f | 149
                                 // UNIT_MOD_CAST_SPEED 1.0f | 166
                                 // UNIT_FIELD_BASE_MANA: 100 | 207
                                 // UNIT_FIELD_BASE_HEALTH: 100 | 208
                                 // UNIT_FIELD_BYTES_2 - offset 1 0x28 - 209
                                 // PLAYER_BYTES: haircolor - hairstyle - face - skin | 239
                                 // PLAYER_BYTES_2: 0x02 - 0 - 0 - facialHair | 280
                                 // PLAYER_BYTES_3 : gender | 281
                                 // PLAYER_XP: 0 - 926
                                 // PLAYER_FIELD_MAX_LEVEL: 70 | 1566
}
*/
#[binwrite]
#[server_opcode]
pub struct SmsgTimeSyncReq {
    pub sync_counter: u32,
}

#[binread]
pub struct CmsgUpdateAccountData {
    pub account_data_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgUpdateAccountData {
    pub account_data_id: u32,
    pub data: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgSetRestStart {
    pub rest_start: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgBindpointupdate {
    pub homebind_x: f32,
    pub homebind_y: f32,
    pub homebind_z: f32,
    pub homebind_map_id: u32,
    pub homebind_area_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLoginSettimespeed {
    pub timestamp: u32,
    pub game_speed: f32,
}
