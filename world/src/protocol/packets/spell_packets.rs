use std::time::Duration;

use binrw::{binread, binwrite, BinRead, BinWrite, NullString};
use enumflags2::{make_bitflags, BitFlags};
use opcode_derive::server_opcode;

use crate::entities::object_guid::{ObjectGuid, PackedObjectGuid};
use crate::entities::position::Position;
use crate::game::spell_cast_target::SpellCastTargets;
use crate::protocol::opcodes::Opcode;
use crate::protocol::server::ServerMessagePayload;
use crate::shared::constants::{SpellCastTargetFlags, SpellFailReason};

impl BinRead for SpellCastTargets {
    type Args<'a> = ();

    fn read_options<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<Self> {
        let target_mask = <u32>::read_options(reader, endian, args)?;
        let target_mask: BitFlags<SpellCastTargetFlags> =
            BitFlags::from_bits(target_mask).expect("failed to parse CMSG_CAST_SPELL target_mask");

        if target_mask.is_empty() {
            return Ok(SpellCastTargets::new_self());
        };

        let mut unit_guid: Option<ObjectGuid> = None;
        if target_mask.intersects(make_bitflags!(SpellCastTargetFlags::{Unit | Unk2})) {
            unit_guid = <PackedObjectGuid>::read_options(reader, endian, args)
                .ok()
                .and_then(ObjectGuid::from_packed);
        }

        let mut game_object_guid: Option<ObjectGuid> = None;
        if target_mask
            .intersects(make_bitflags!(SpellCastTargetFlags::{Object | ObjectUnk | GameobjectItem}))
        {
            game_object_guid = <PackedObjectGuid>::read_options(reader, endian, args)
                .ok()
                .and_then(ObjectGuid::from_packed);
        }

        let mut item_guid: Option<ObjectGuid> = None;
        if target_mask.intersects(make_bitflags!(SpellCastTargetFlags::{Item | TradeItem})) {
            item_guid = <PackedObjectGuid>::read_options(reader, endian, args)
                .ok()
                .and_then(ObjectGuid::from_packed);
        }

        let mut source_position: Option<Position> = None;
        if target_mask.intersects(make_bitflags!(SpellCastTargetFlags::{SourceLocation})) {
            let x = <f32>::read_options(reader, endian, args)?;
            let y = <f32>::read_options(reader, endian, args)?;
            let z = <f32>::read_options(reader, endian, args)?;

            source_position = Some(Position { x, y, z, o: 0. });
        }

        let mut destination_position: Option<Position> = None;
        if target_mask.intersects(make_bitflags!(SpellCastTargetFlags::{DestLocation})) {
            let x = <f32>::read_options(reader, endian, args)?;
            let y = <f32>::read_options(reader, endian, args)?;
            let z = <f32>::read_options(reader, endian, args)?;

            destination_position = Some(Position { x, y, z, o: 0. });
        }

        let mut string: Option<String> = None;
        if target_mask.intersects(make_bitflags!(SpellCastTargetFlags::{String})) {
            string = <NullString>::read_options(reader, endian, args)
                .ok()
                .map(|null_string| null_string.to_string());
        }

        Ok(SpellCastTargets::new(
            unit_guid,
            game_object_guid,
            item_guid,
            source_position,
            destination_position,
            string,
        ))
    }
}

#[binread]
pub struct CmsgCastSpell {
    pub spell_id: u32,
    pub cast_count: u8,
    pub cast_targets: SpellCastTargets,
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

#[binwrite]
#[server_opcode]
pub struct SmsgSpellCooldown {
    pub guid: ObjectGuid,
    pub flags: u8, // TODO: https://github.com/TrinityCore/TrinityCore/blob/e57b0296d65446e358ead632750c4ae0c5249631/src/server/game/Spells/SpellHistory.h#L49
    pub cooldowns: Vec<ClientSpellCooldown>,
}

#[derive(Debug)]
pub struct ClientSpellCooldown {
    pub spell_id: u32,
    pub cooldown_ms: u32,
}

impl BinWrite for ClientSpellCooldown {
    type Args<'a> = ();

    fn write_options<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<()> {
        self.spell_id.write_options(writer, endian, args)?;
        self.cooldown_ms.write_options(writer, endian, args)?;
        Ok(())
    }
}

#[binwrite]
#[server_opcode]
pub struct SmsgPlaySpellVisual {
    pub caster_guid: ObjectGuid,
    pub spell_art_kit: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgPlaySpellImpact {
    pub caster_guid: ObjectGuid,
    pub spell_art_kit: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgLearnedSpell {
    pub spell_id: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgUpdateAuraDuration {
    pub slot: u8,
    pub duration_ms: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgSetExtraAuraInfo {
    pub target_guid: PackedObjectGuid,
    pub slot: u8,
    pub spell_id: u32,
    pub max_duration_ms: u32,
    pub duration_ms: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgSetExtraAuraInfoNeedUpdate {
    pub target_guid: PackedObjectGuid,
    pub slot: u8,
    pub spell_id: u32,
    pub max_duration_ms: u32,
    pub duration_ms: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgClearExtraAuraInfo {
    pub target_guid: PackedObjectGuid,
    pub spell_id: u32,
}
