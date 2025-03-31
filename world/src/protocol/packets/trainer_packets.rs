use binrw::binwrite;
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binwrite]
#[server_opcode]
pub struct SmsgTrainerBuySucceeded {
    pub trainer_guid: ObjectGuid,
    pub spell_id: u32,
}
