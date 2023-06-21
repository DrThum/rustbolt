use std::time::Duration;

use binrw::{binread, binwrite};
use opcode_derive::server_opcode;

use crate::entities::object_guid::PackedObjectGuid;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::SpellFailReason;

#[binread]
pub struct CmsgCastSpell {
    pub spell_id: u32,
    pub cast_count: u8,
}

#[binwrite]
#[server_opcode]
pub struct SmsgClearExtraAuraInfo {
    pub caster_guid: PackedObjectGuid,
    pub spell_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgSpellStart {
    pub caster_entity_guid: PackedObjectGuid, // Can be an item for example
    pub caster_unit_guid: PackedObjectGuid,
    pub spell_id: u32,
    pub cast_id: u8,
    pub cast_flags: u16, // TODO: BitFlags
    #[bw(map = |dur: &Duration| dur.as_millis() as u32)]
    pub cast_time: Duration,
    // TODO: Target guid and hit status (optional)
    // BEGIN target
    pub target_flags: u32, // 0 for now
                           // pub target_unit_guid: Option<u64>,
                           // pub target_item_guid: Option<u64>,
                           // pub source_position: Option<Position>,
                           // pub dest_position: Option<Position>,
                           // pub name: Option<String>,
                           // END target
                           // TODO: Ammo (optional)
}

#[binwrite]
#[server_opcode]
pub struct SmsgSpellGo {
    pub caster_entity_guid: PackedObjectGuid, // Can be an item for example
    pub caster_unit_guid: PackedObjectGuid,
    pub spell_id: u32,
    pub cast_flags: u16, // TODO: BitFlags
    pub timestamp: u32,
    pub target_count: u8,
    // TODO: target data
    // TODO: optional ammo if ranged spell
}

#[binwrite]
#[server_opcode]
pub struct SmsgCastFailed {
    pub spell_id: u32,
    #[bw(map = |sfr: &SpellFailReason| (*sfr) as u8)]
    pub result: SpellFailReason,
    pub cast_count: u8,
    // requires_spell_focus: u32 // if RequiresSpellFocus
    // requires_area_id: u32 // if RequiresArea
    // requires_totem: [u32; MAX_TOTEM] // if Totems
    // requires_totem_category: [u32; MAX_TOTEM_CATEGORY // if TotemCategory
    // { // if EquippedItemClass
    //   item_class: u32,
    //   item_sub_class_mask: u32,
    //   item_inventory_type_mask: u32,
    // }
}

#[binread]
pub struct CmsgCancelCast {
    pub spell_id: u32,
}

#[binwrite]
pub struct InitialSpell {
    pub spell_id: u16,
    pub unk: u16, // 0
}

#[binwrite]
pub struct InitialSpellCooldown {
    pub spell_id: u16,
    pub cast_item_id: u16,
    pub spell_category: u16,
    pub cooldown_millis: u32,
    pub category_cooldown: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgInitialSpells {
    unk: u8, // 0
    spell_count: u16,
    spells: Vec<InitialSpell>,
    cooldown_count: u16,
    cooldowns: Vec<InitialSpellCooldown>,
}

impl SmsgInitialSpells {
    pub fn new(spells: Vec<u32>, cooldowns: Vec<InitialSpellCooldown>) -> Self {
        SmsgInitialSpells {
            unk: 0,
            spell_count: spells.len() as u16,
            spells: spells
                .iter()
                .map(|&spell_id| InitialSpell {
                    spell_id: spell_id as u16,
                    unk: 0,
                })
                .collect(),
            cooldown_count: cooldowns.len() as u16,
            cooldowns,
        }
    }
}
