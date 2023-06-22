use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::QuestGiverStatus;

#[binread]
pub struct CmsgQuestGiverStatusQuery {
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQuestGiverStatus {
    pub guid: u64,
    #[bw(map = |status: &QuestGiverStatus| *status as u8)]
    pub status: QuestGiverStatus,
}

#[binread]
pub struct CmsgQuestGiverHello {
    pub guid: u64,
}
