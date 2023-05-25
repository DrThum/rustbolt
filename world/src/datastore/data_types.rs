use crate::{
    protocol::packets::{
        ItemTemplateDamage, ItemTemplateSocket, ItemTemplateSpell, ItemTemplateStat,
    },
    shared::constants::{InventoryType, MapType},
};

use super::dbc::{DbcRecord, DbcStringBlock};

pub trait DbcTypedRecord {
    fn from_record(record: &DbcRecord, string: &DbcStringBlock) -> (u32, Self);
}

#[derive(Debug)]
pub struct ChrRacesRecord {
    // _flags: u32,
    pub faction_id: u32,
    // _exploration_sound_id: u32,
    pub male_display_id: u32,
    pub female_display_id: u32,
    // _client_prefix: String, // stringref (offset into the String block of the DBC file)
    // _mount_scale: f32,
    // _base_language: u32,         // 1 = Horde, 7 = Alliance & Not Playable
    // _creature_type: u32,         // Always 7 (humanoid)
    // _res_sickness_spell_id: u32, // Always 15007
    // _splash_sound_id: u32,
    // _client_file_string: String,
    // _opening_cinematic_id: u32, // Ref to another DBC
    // _race_name_neutral: LocalizedString,
    // _race_name_female: LocalizedString,
    // _race_name_male: LocalizedString,
    // _facial_hair_customization_internal: String,
    // _facial_hair_customization_lua: String,
    // _hair_customization: String,
    // _required_expansion: i32,
}

impl DbcTypedRecord for ChrRacesRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = ChrRacesRecord {
                faction_id: record.fields[2].as_u32,
                male_display_id: record.fields[4].as_u32,
                female_display_id: record.fields[5].as_u32,
            };

            (key, record)
        }
    }
}

pub struct ChrClassesRecord {
    // _unk: u32,
    pub power_type: u32, // See enum PowerType
                         // _pet_name_token: String, // string ref
                         // _name: LocalizedString,
                         // _name_female: LocalizedString,
                         // _name_male: LocalizedString,
                         // _file_name: String,
                         // _spell_class_set: u32, // https://wowdev.wiki/DB/ChrClasses#spellClassSet
                         // _flags: u32, // https://wowdev.wiki/DB/ChrClasses#Flags
}

impl DbcTypedRecord for ChrClassesRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = ChrClassesRecord {
                power_type: record.fields[2].as_u32,
            };

            (key, record)
        }
    }
}

pub const MAX_OUTFIT_ITEMS: usize = 12;
#[derive(Clone, Copy)]
pub struct CharStartItem {
    pub id: u32,
    pub display_id: u32,
    pub inventory_type: InventoryType,
}

pub struct CharStartOutfitRecord {
    pub race_class_gender: u32, // 0x00GGCCRR (G = gender, C = class, R = race)
    pub items: Vec<CharStartItem>,
}

impl DbcTypedRecord for CharStartOutfitRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            // (outfit_id << 24 | gender << 16 | class << 8 | race)
            // where outfit_id is always 0
            let key = record.fields[1].as_u32;

            let mut items: Vec<CharStartItem> = Vec::new();
            items.reserve(MAX_OUTFIT_ITEMS);
            for index in 0..MAX_OUTFIT_ITEMS {
                if record.fields[2 + index].as_i32 < 1 {
                    // 0 and -1 represent empty slots
                    continue;
                }

                let id = record.fields[2 + index].as_u32;
                let display_id = record.fields[2 + MAX_OUTFIT_ITEMS + index].as_u32;
                let inventory_type =
                    InventoryType::n(record.fields[2 + (2 * MAX_OUTFIT_ITEMS) + index].as_u32)
                        .expect("Invalid inventory type found in CharStartOutfit.dbc");

                items.push(CharStartItem {
                    id,
                    display_id,
                    inventory_type,
                })
            }

            let record = CharStartOutfitRecord {
                race_class_gender: key,
                items,
            };

            (key, record)
        }
    }
}

#[derive(Clone, Copy)]
pub struct ItemRecord {
    pub id: u32,
    pub display_id: u32,
    pub inventory_type: InventoryType,
    // _sheathe_type: u32,
}

impl DbcTypedRecord for ItemRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = ItemRecord {
                id: record.fields[0].as_u32,
                display_id: record.fields[1].as_u32,
                inventory_type: InventoryType::n(record.fields[2].as_u32)
                    .expect("Invalid InventoryType found in Item.dbc"),
            };

            (key, record)
        }
    }
}

#[allow(dead_code)]
pub struct ItemTemplate {
    pub entry: u32,
    pub class: u32,
    pub subclass: u32,
    pub unk0: i32,
    pub name: String,
    pub display_id: u32,
    pub quality: u32,
    pub flags: u32,
    pub buy_count: u32,
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
    pub stats: Vec<ItemTemplateStat>,
    pub damages: Vec<ItemTemplateDamage>,
    pub armor: u32,
    pub holy_res: u32,
    pub fire_res: u32,
    pub nature_res: u32,
    pub frost_res: u32,
    pub shadow_res: u32,
    pub arcane_res: u32,
    pub delay: u32,
    pub ammo_type: u32,
    pub ranged_mod_range: f32,
    pub spells: Vec<ItemTemplateSpell>,
    pub bonding: u32,
    pub description: String,
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
    pub itemset: u32,
    pub max_durability: u32,
    pub area: u32,
    pub map: u32,
    pub bag_family: u32,
    pub totem_category: u32,
    pub sockets: Vec<ItemTemplateSocket>,
    pub socket_bonus: u32,
    pub gem_properties: u32,
    pub required_disenchant_skill: i32,
    pub armor_damage_modifier: f32,
    pub disenchant_id: u32,
    pub food_type: u32,
    pub min_money_loot: u32,
    pub max_money_loot: u32,
    pub duration: u32,
}

pub struct PlayerCreatePosition {
    pub race: u32,
    pub class: u32,
    pub map: u32,
    pub zone: u32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
}

#[derive(Debug)]
pub struct MapRecord {
    pub id: u32,
    pub internal_name: String,
    pub map_type: MapType,
    // is_pvp: u32 // 0 or 1 (for battlegrounds only)
    // name: LocalizedString [4-20]
    // min_level: u32
    // max_level: u32
    // max_players: u32
    // unk [24-26]
    // linked_zone_id: u32 // ref to AreaTable.dbc
    // description_horde: LocalizedString [28-44]
    // description_alliance: LocalizedString [45-61]
    // loading_screen_id: u32
    // unk [63-64]
    // minimap_icon_scale: f32
    // unk: LocalizedString [66-82] (unused)
    // heroic_requirement: LocalizedString [83-99]
    // unk: LocalizedString [100-116]
    // ghost_entrance_map_id: u32
    // ghost_entrance_x: f32
    // ghost_instance_y: f32
    // reset_time_raid: u32 // in seconds
    // reset_time_heroic: u32 // in seconds
    // unk (unused)
    // time_of_day_override: i32 // always -1
    // expansion_id: u32 (0 = Vanilla, 1 = TBC)
}

impl DbcTypedRecord for MapRecord {
    fn from_record(record: &DbcRecord, strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = MapRecord {
                id: record.fields[0].as_u32,
                internal_name: strings
                    .get(record.fields[1].as_u32 as usize)
                    .expect("string not found in Map.dbc"),
                map_type: MapType::n(record.fields[2].as_u32)
                    .expect("Invalid map type found in Map.dbc"),
            };

            (key, record)
        }
    }
}

impl MapRecord {
    pub fn is_instanceable(&self) -> bool {
        match self.map_type {
            MapType::Common => false,
            _ => true,
        }
    }
}

pub struct EmotesTextRecord {
    pub text_id: u32,
}

impl DbcTypedRecord for EmotesTextRecord {
    fn from_record(record: &DbcRecord, _string: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = EmotesTextRecord {
                text_id: record.fields[2].as_u32,
            };

            (key, record)
        }
    }
}

#[allow(dead_code)]
pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub sub_name: Option<String>,
    pub icon_name: Option<String>,
    pub min_level: u32,
    pub max_level: u32,
    pub min_level_health: u32,
    pub max_level_health: u32,
    pub min_level_mana: u32,
    pub max_level_mana: u32,
    pub model_ids: Vec<u32>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub family: u32,  // CreatureFamily.dbc
    pub type_id: u32, // CreatureType.dbc
    pub type_flags: u32,
    pub rank: u32,
    pub racial_leader: u8, // bool
    pub health_multiplier: f32,
    pub power_multiplier: f32,
    pub pet_spell_data_id: u32,   // CreatureSpellData.dbc
    pub faction_template_id: u32, // FactionTemplate.dbc
}
