use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

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
pub struct MsgSetDungeonDifficulty {
    pub difficulty: u32, // 0 = Normal, 1 = Heroic
    pub unk: u32,        // Always 1
    pub is_in_group: u32,
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

#[binwrite]
#[server_opcode]
pub struct SmsgActionButtons {
    pub buttons_packed: Vec<u32>,
}

#[binwrite]
pub struct FactionInit {
    pub flags: u8,
    pub standing: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgInitializeFactions {
    pub unk: u32, // 0x80
    pub factions: Vec<FactionInit>,
}

#[binread]
pub struct CmsgSetSelection {
    pub guid: u64,
}
