use binrw::{binread, binrw, binwrite, NullString};
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

#[binread]
pub struct CmsgNameQuery {
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgNameQueryResponse {
    pub guid: u64,
    pub name: NullString,
    pub realm_name: u8, // Use 0, intended for cross-realm battlegrounds
    pub race: u32,
    pub class: u32,
    pub gender: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub is_name_declined: bool, // use false
                                // pub declined_names: [NullString, 5],
}

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

#[binread]
pub struct CmsgCreatureQuery {
    pub entry: u32,
    pub guid: u64,
}

#[binwrite]
#[server_opcode]
pub struct SmsgCreatureQueryResponse {
    pub entry: u32,
    pub name: NullString,
    pub name2: u8, // 0
    pub name3: u8, // 0
    pub name4: u8, // 0
    pub sub_name: NullString,
    pub icon_name: NullString,
    pub type_flags: u32,
    pub type_id: u32,
    pub family: u32,
    pub rank: u32,
    pub unk: u32, // 0
    pub pet_spell_data_id: u32,
    pub model_ids: Vec<u32>,
    pub health_multiplier: f32,
    pub power_multiplier: f32,
    pub racial_leader: u8,
}

#[binwrite]
pub struct SmsgCreatureQueryResponseUnknownTemplate {
    pub masked_entry: u32,
}
impl ServerMessagePayload<{ Opcode::SmsgCreatureQueryResponse as u16 }>
    for SmsgCreatureQueryResponseUnknownTemplate
{
}
