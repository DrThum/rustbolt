use binrw::{binread, binwrite, NullString};
use opcode_derive::server_opcode;

use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binread]
pub struct CmsgPing {
    pub ping: u32,
    pub latency: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgPong {
    pub ping: u32,
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
pub struct SmsgLoginSetTimeSpeed {
    pub timestamp: u32,
    pub game_speed: f32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgQueryTimeResponse {
    pub seconds_since_epoch: u32,
    pub seconds_until_daily_quests_reset: u32,
}

#[binread]
pub struct CmsgTimeSyncResp {
    pub counter: u32,
    pub ticks: u32, // Seconds since server start
}

#[binread]
pub struct CmsgSetActionBarToggles {
    pub toggles: u8,
}
