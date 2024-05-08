use binrw::{binread, binwrite, BinWrite, NullString};
use enumn::N;
use opcode_derive::server_opcode;
use rusqlite::types::{FromSql, FromSqlError};

use crate::{
    datastore::data_types::ItemTemplate,
    entities::object_guid::ObjectGuid,
    protocol::{opcodes::Opcode, server::ServerMessagePayload},
    shared::constants::{AttributeModifier, InventoryResult},
};

#[binread]
pub struct CmsgItemQuerySingle {
    pub item_id: u32,
}

#[binread]
pub struct CmsgItemNameQuery {
    pub item_id: u32,
    pub item_guid: u64,
}

#[binwrite]
pub struct ItemTemplateStat {
    pub stat_type: ItemTemplateStatType,
    pub stat_value: i32,
}

#[derive(N, Clone, Copy)]
pub enum ItemTemplateStatType {
    NoStat = 0,
    Health = 1,
    Agility = 3,
    Strength = 4,
    Intellect = 5,
    Spirit = 6,
    Stamina = 7,
    DefenseSkillRating = 12,
    DodgeRating = 13,
    ParryRating = 14,
    BlockRating = 15,
    HitMeleeRating = 16,
    HitRangedRating = 17,
    HitSpellRating = 18,
    CritMeleeRating = 19,
    CritRangedRating = 20,
    CritSpellRating = 21,
    HitTakenMeleeRating = 22,
    HitTakenRangedRating = 23,
    HitTakenSpellRating = 24,
    CritTakenMeleeRating = 25,
    CritTakenRangedRating = 26,
    CritTakenSpellRating = 27,
    HasteMeleeRating = 28,
    HasteRangedRating = 29,
    HasteSpellRating = 30,
    HitRating = 31,
    CritRating = 32,
    HitTakenRating = 33,
    CritTakenRating = 34,
    ResilienceRating = 35,
    HasteRating = 36,
    ExpertiseRating = 37,
}

impl ItemTemplateStatType {
    pub fn as_attribute_modifier(&self) -> Option<AttributeModifier> {
        use AttributeModifier::*;

        match self {
            ItemTemplateStatType::Health => Some(Health),
            ItemTemplateStatType::Agility => Some(StatAgility),
            ItemTemplateStatType::Strength => Some(StatStrength),
            ItemTemplateStatType::Intellect => Some(StatIntellect),
            ItemTemplateStatType::Spirit => Some(StatSpirit),
            ItemTemplateStatType::Stamina => Some(StatStamina),
            ItemTemplateStatType::NoStat => None,
            _ => None,
        }
    }
}

impl FromSql for ItemTemplateStatType {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let value = value.as_i64()?;
        Self::n(value).map_or(
            Err(FromSqlError::Other(
                "invalid stat type on item template".into(),
            )),
            Ok,
        )
    }
}

impl BinWrite for ItemTemplateStatType {
    type Args<'a> = ();

    fn write_options<W: std::io::prelude::Write + std::io::prelude::Seek>(
        &self,
        writer: &mut W,
        endian: binrw::Endian,
        args: Self::Args<'_>,
    ) -> binrw::prelude::BinResult<()> {
        <u32>::write_options(&(*self as u32), writer, endian, args)
    }
}

#[binwrite]
pub struct ItemTemplateDamage {
    pub damage_min: f32,
    pub damage_max: f32,
    pub damage_type: u32,
}

#[binwrite]
pub struct ItemTemplateSpell {
    pub id: u32,
    pub trigger_id: u32,
    pub charges: i32,
    #[bw(ignore)]
    pub ppm_rate: f32,
    pub cooldown: i32, // default -1
    pub category: u32,
    pub category_cooldown: i32, // default -1
}

#[binwrite]
pub struct ItemTemplateSocket {
    pub color: u32,
    pub content: u32,
}

#[binwrite]
pub struct ItemQueryResponse<'a> {
    pub item_id: u32,
    pub item_class: u32,
    pub item_subclass: u32,
    pub item_unk: i32, // -1
    pub name: NullString,
    pub name2: u8, // 0
    pub name3: u8, // 0
    pub name4: u8, // 0
    pub display_id: u32,
    pub quality: u32,
    pub flags: u32,
    pub buy_price: u32,
    pub sell_price: u32,
    pub inventory_type: u32,
    pub allowable_class: i32,
    pub allowable_race: i32,
    pub item_level: u32,
    pub required_level: u32,
    pub required_skill: u32,
    pub required_skill_rank: u32,
    pub required_spell: u32,
    pub required_honor_rank: u32,
    pub required_city_rank: u32,
    pub required_reputation_faction: u32,
    pub required_reputation_rank: u32,
    pub max_count: u32,
    pub max_stack_count: u32,
    pub container_slots: u32,
    pub stats: &'a Vec<ItemTemplateStat>,
    pub damages: &'a Vec<ItemTemplateDamage>,
    pub armor: u32,
    pub resist_holy: u32,
    pub resist_fire: u32,
    pub resist_nature: u32,
    pub resist_frost: u32,
    pub resist_shadow: u32,
    pub resist_arcane: u32,
    pub delay: u32,
    pub ammo_type: u32,
    pub ranged_mod_range: f32,
    pub spells: &'a Vec<ItemTemplateSpell>,
    pub bonding: u32,
    pub description: NullString,
    pub page_text: u32,
    pub language_id: u32,
    pub page_material: u32,
    pub start_quest: u32,
    pub lock_id: u32,
    pub material: i32,
    pub sheath: u32,
    pub random_property: u32,
    pub random_suffix: u32,
    pub block: u32,
    pub item_set: u32,
    pub max_durability: u32,
    pub area: u32,
    pub map: u32,
    pub bag_family: u32,
    pub totem_category: u32,
    pub sockets: &'a Vec<ItemTemplateSocket>,
    pub socket_bonus: u32,
    pub gem_properties: u32,
    pub required_enchantment_skill: i32,
    pub armor_damage_modifier: f32,
    pub duration: u32,
}

#[binwrite]
pub struct SmsgItemQuerySingleResponse<'a> {
    pub result: Option<u32>,
    pub template: Option<ItemQueryResponse<'a>>,
}

impl ServerMessagePayload<{ Opcode::SmsgItemQuerySingleResponse as u16 }>
    for SmsgItemQuerySingleResponse<'_>
{
}

#[binwrite]
#[server_opcode]
pub struct SmsgItemNameQueryResponse {
    pub item_id: u32,
    pub name: NullString,
    pub inventory_type: u32,
}

#[binwrite]
#[server_opcode]
pub struct SmsgItemPushResult {
    pub player_guid: ObjectGuid,
    pub loot_source: u32,        // 0 = looted, 1 = received from NPC
    pub is_created: u32,         // 0 = received, 1 = created
    pub is_visible_in_chat: u32, // boolean
    pub bag_slot: u8,
    // item slot or 0xFFFFFFFF if the item is added to an existing stack
    pub item_slot: u32,
    pub item_id: u32,
    pub item_suffix_factor: u32,      // SuffixFactor (?)
    pub item_random_property_id: u32, // TODO
    pub count: u32,
    pub total_count_of_this_item_in_inventory: u32,
}

#[binread]
pub struct CmsgDestroyItem {
    pub bag: u8,
    pub slot: u8,
    pub amount: u8,
    pub unk1: u8,
    pub unk2: u8,
    pub unk3: u8,
}

#[binread]
pub struct CmsgAutoEquipItem {
    pub bag: u8,
    pub slot: u8,
}

#[binread]
pub struct CmsgSwapInvItem {
    pub from_slot: u8,
    pub to_slot: u8,
}

#[binwrite]
#[server_opcode]
pub struct SmsgInventoryChangeFailure {
    result: u8,
    required_level: Option<u32>, // Only if InventoryResult::CantEquipLevelI
    moved_item_guid: Option<ObjectGuid>,
    target_item_guid: Option<ObjectGuid>,
    bag_type_subclass: Option<u8>, // 0 unless AutoequipBindConfirm or ItemDoesntGoIntoBag2
}

impl SmsgInventoryChangeFailure {
    pub fn build(
        result: InventoryResult,
        moved_item_guid: Option<ObjectGuid>,
        moved_item_template: Option<&ItemTemplate>,
        target_item_guid: Option<ObjectGuid>,
    ) -> Self {
        let required_level = if result == InventoryResult::CantEquipLevelI {
            moved_item_template.map(|template| template.required_level)
        } else {
            None
        };

        let moved_item_guid = moved_item_guid.filter(|_| result != InventoryResult::Ok);
        let target_item_guid = target_item_guid.filter(|_| result != InventoryResult::Ok);

        Self {
            result: result as u8,
            required_level,
            moved_item_guid,
            target_item_guid,
            bag_type_subclass: Some(0),
        }
    }
}

#[binread]
pub struct CmsgSplitItem {
    pub source_bag: u8,
    pub source_slot: u8,
    pub destination_bag: u8,
    pub destination_slot: u8,
    pub count: u8,
}
