use binrw::binwrite;
use opcode_derive::server_opcode;

use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binwrite]
#[server_opcode]
pub struct SmsgInitWorldStates {
    pub map_id: u32,
    pub zone_id: u32,
    pub area_id: u32,
    pub block_count: u16, // 0 for now
}
