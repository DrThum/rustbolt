use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::{ObjectGuid, PackedObjectGuid};
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;

#[binread]
pub struct CmsgAttackSwing {
    pub guid: ObjectGuid,
}

#[binwrite]
#[server_opcode]
pub struct SmsgAttackStop {
    pub attacker_guid: PackedObjectGuid,
    pub enemy_guid: PackedObjectGuid,
    pub unk: u32, // 0 or 1, "NowDead" in TrinityCore
}

#[binwrite]
#[server_opcode]
pub struct SmsgAttackStart {
    pub attacker_guid: ObjectGuid,
    pub target_guid: ObjectGuid,
}

#[binwrite]
#[server_opcode]
pub struct SmsgAttackerStateUpdate {
    pub hit_info: u32, // TODO: Bitmask
    pub attacker_guid: PackedObjectGuid,
    pub target_guid: PackedObjectGuid,
    pub actual_damage: u32,
    pub sub_damage_count: u8,        // Always 1?
    pub sub_damage_school_mask: u32, // TODO: Bitmask
    pub sub_damage: f32,
    pub sub_damage_rounded: u32, // ?
    pub sub_damage_absorb: u32,
    pub sub_damage_resist: u32,
    pub target_state: u32,
    pub unk1: u32,     // -1, 0 or 1000
    pub spell_id: u32, // Heroic Strike or Disarm for example
    pub damage_blocked_amount: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgAttackSwingNotInRange {}
