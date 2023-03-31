use crate::{entities::object_guid::PackedObjectGuid, shared::constants::InventoryType};

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
    pub username: NullString,
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
    pub billing_time: u32,
    pub billing_flags: u8,
    pub billing_rested: u32,
    pub expansion: u8, // 0 = Vanilla, 1 = TBC
    pub position_in_queue: u32,
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
    pub data: [u32; 32], // All 0
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
pub struct ObjectUpdate {
    pub update_type: u8,
    pub packed_guid: PackedObjectGuid,
    pub object_type: u8,
    pub flags: u8,
    pub movement_flags: u32,
    pub movement_flags2: u8, // Always 0 in 2.4.3
    pub timestamp: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub fall_time: u32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub speed_run_backward: f32,
    pub speed_swim: f32,
    pub speed_swim_backward: f32,
    pub speed_flight: f32,
    pub speed_flight_backward: f32,
    pub speed_turn: f32,
    pub unk_highguid: Option<u32>,
    pub num_mask_blocks: u8,
    pub mask_blocks: Vec<u32>,
    pub data: Vec<[u8; 4]>,
}

#[binwrite]
#[server_opcode]
pub struct SmsgUpdateObject {
    pub updates_count: u32,
    pub has_transport: u8,
    pub updates: Vec<ObjectUpdate>,
}

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

#[binwrite]
#[server_opcode]
pub struct MsgSetDungeonDifficulty {
    pub difficulty: u32, // 0 = Normal, 1 = Heroic
    pub unk: u32,        // Always 1
    pub is_in_group: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgInitWorldStates {
    pub map_id: u32,
    pub zone_id: u32,
    pub area_id: u32,
    pub block_count: u16, // 0 for now
}

#[binread]
pub struct CmsgLogoutRequest {}

#[binwrite]
#[server_opcode]
pub struct SmsgLogoutResponse {
    pub reason: u32, // 0 for success, anything else will show "You can't logout right now"
    pub is_instant_logout: u8, // Boolean
}

#[binwrite]
#[server_opcode]
pub struct SmsgLogoutComplete {}
