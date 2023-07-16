use binrw::binwrite;
use opcode_derive::server_opcode;

use crate::entities::object_guid::{ObjectGuid, PackedObjectGuid};
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binwrite]
#[server_opcode]
pub struct SmsgMoveSetCanFly {
    pub guid: PackedObjectGuid,
    pub counter: u32,
}

impl SmsgMoveSetCanFly {
    pub fn build(guid: &ObjectGuid) -> Self {
        Self {
            guid: guid.as_packed(),
            counter: 0,
        } // TODO: Implement ACK etc
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgMoveUnsetCanFly {
    pub guid: PackedObjectGuid,
    pub counter: u32,
}

impl SmsgMoveUnsetCanFly {
    pub fn build(guid: &ObjectGuid) -> Self {
        Self {
            guid: guid.as_packed(),
            counter: 0,
        } // TODO: Implement ACK etc
    }
}
