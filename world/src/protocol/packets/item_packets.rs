use binrw::{binread, binwrite, NullString};

use crate::protocol::{opcodes::Opcode, server::ServerMessagePayload};

#[binread]
pub struct CmsgItemQuerySingle {
    pub item_id: u32,
}

#[binwrite]
pub struct ItemTemplateStat {
    pub stat_type: u32,
    pub stat_value: i32,
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