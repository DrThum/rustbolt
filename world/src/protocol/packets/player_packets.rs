use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::ObjectGuid;
use crate::entities::position::WorldPosition;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binwrite]
#[server_opcode]
pub struct SmsgSetRestStart {
    pub rest_start: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgBindpointUpdate {
    pub homebind_x: f32,
    pub homebind_y: f32,
    pub homebind_z: f32,
    pub homebind_map_id: u32,
    pub homebind_area_id: u32,
}

impl SmsgBindpointUpdate {
    pub fn from_position(position: &WorldPosition) -> Self {
        Self {
            homebind_x: position.x,
            homebind_y: position.y,
            homebind_z: position.z,
            homebind_map_id: position.map_key.map_id,
            homebind_area_id: position.zone,
        }
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgPlayerBound {
    pub caster_guid: ObjectGuid, // The innkeeper's guid (that cast the Bind spell)
    pub area_id: u32,
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

#[binwrite]
#[server_opcode]
pub struct SmsgLogXpGain {
    pub victim_guid: u64,
    pub given_xp: u32,
    #[bw(map = |b: &bool| if *b { 1_u8 } else { 0_u8 })]
    pub from_kill: bool,
    #[bw(if(!from_kill))]
    pub xp_without_rested_bonus: Option<u32>,
    #[bw(if(!from_kill))]
    pub group_bonus: Option<u32>,
    pub unk: u8, // Always 0
}

impl SmsgLogXpGain {
    pub fn build(victim_guid: Option<ObjectGuid>, experience: u32) -> Self {
        Self {
            victim_guid: victim_guid.map(|g| g.raw()).unwrap_or(0),
            given_xp: experience,
            from_kill: victim_guid.is_some(),
            xp_without_rested_bonus: victim_guid.map(|_| experience), // TODO: Implement rested xp
            group_bonus: victim_guid.map(|_| 0),                      // TODO: Implement groups
            unk: 0,
        }
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgLevelUpInfo {
    pub level: u32,
    pub health_gained: u32,
    pub mana_gained: u32,
    pub powers_gained: [u32; 4], // Filled with zeroes
    pub strength_gained: u32,
    pub agility_gained: u32,
    pub stamina_gained: u32,
    pub intellect_gained: u32,
    pub spirit_gained: u32,
}

impl SmsgLevelUpInfo {
    // TODO: enrich this when we have a better stats system
    pub fn build(
        level: u32,
        health_gained: u32,
        mana_gained: u32,
        strength_gained: u32,
        agility_gained: u32,
        stamina_gained: u32,
        intellect_gained: u32,
        spirit_gained: u32,
    ) -> Self {
        Self {
            level,
            health_gained,
            mana_gained,
            powers_gained: [0, 0, 0, 0],
            strength_gained,
            agility_gained,
            stamina_gained,
            intellect_gained,
            spirit_gained,
        }
    }
}
