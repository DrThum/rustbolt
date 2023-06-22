use binrw::{binread, binrw, binwrite};
use opcode_derive::server_opcode;

use crate::{
    entities::{
        position::Position,
        update::{CreateData, UpdateData},
    },
    protocol::{opcodes::Opcode, server::ServerMessagePayload},
};

#[binwrite]
#[derive(Clone, Debug)]
pub struct SmsgCreateObject {
    pub updates_count: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub has_transport: bool,
    pub updates: Vec<CreateData>,
}

impl ServerMessagePayload<{ Opcode::SmsgUpdateObject as u16 }> for SmsgCreateObject {}

#[binwrite]
pub struct SmsgUpdateObject {
    pub updates_count: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub has_transport: bool,
    pub updates: Vec<UpdateData>,
}

impl ServerMessagePayload<{ Opcode::SmsgUpdateObject as u16 }> for SmsgUpdateObject {}

#[binrw]
#[derive(Debug)]
pub struct MovementInfo {
    pub movement_flags: u32,
    pub movement_flags2: u8,
    pub timestamp: u32,
    pub position: Position,
    // TODO: Transport stuff
    // pub pitch: f32, // if SWIMMING or FLYING2
    pub fall_time: u32,
    /* if FALLING: JumpInfo
    velocity: f32,
    sinAngle: f32,
    cosAngle: f32,
    xyspeed: f32
    */
    // pub unk: f32 // if SPLINE_ELEVATION
}

#[binread]
pub struct CmsgStandStateChange {
    pub animstate: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgStandStateUpdate {
    pub animstate: u8,
}

#[binread]
pub struct CmsgSetSheathed {
    pub sheath_state: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgDestroyObject {
    pub guid: u64,
}
