use crate::{
    protocol::packets::{
        ItemTemplateDamage, ItemTemplateSocket, ItemTemplateSpell, ItemTemplateStat,
    },
    shared::constants::{
        InventoryType, MapType, MAX_SPELL_EFFECT_INDEX, MAX_SPELL_REAGENTS, MAX_SPELL_TOTEMS,
    },
};

use super::dbc::{DbcRecord, DbcStringBlock};

pub trait DbcTypedRecord {
    fn from_record(record: &DbcRecord, strings: &DbcStringBlock) -> (u32, Self);
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
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
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

#[allow(dead_code)]
pub struct SpellRecord {
    // id: u32,
    category: u32,
    // castUI: u32
    dispel_type: u32,
    mechanic: u32,
    attributes: u32,
    attributes_ex: u32,
    attributes_ex_b: u32,          // attributesEx2
    attributes_ex_c: u32,          // attributesEx3
    attributes_ex_d: u32,          // attributesEx4
    attributes_ex_e: u32,          // attributesEx5
    attributes_ex_f: u32,          // attributesEx6
    shapeshift_mask: u32,          // Stances
    shapeshift_mask_excluded: u32, // StancesNot
    targets: u32,
    target_creature_type_mask: u32,
    spell_focus_object: u32, // RequiresSpellFocus (GameObject type 8)
    facing_caster_flags: u32,
    caster_aura_state: u32,
    target_aura_state: u32,
    caster_aura_state_not: u32,
    target_aura_state_not: u32,
    casting_time_index: u32, // SpellCastTimes.dbc
    recovery_time: u32,
    category_recovery_time: u32,
    interrupt_flags: u32,
    aura_interrupt_flags: u32,
    channel_interrupt_flags: u32,
    proc_flags: u32,
    proc_chance: u32, // In percent - 101 means 100
    proc_charges: u32,
    max_level: u32,
    base_level: u32,
    spell_level: u32,
    duration_index: u32, // SpellDuration.dbc
    power_type: u32,
    mana_cost: u32,
    mana_cost_perlevel: u32,
    mana_per_second: u32,
    mana_per_second_per_level: u32,
    range_index: u32, // SpellRange.dbc
    speed: f32,
    // modalNextSpell
    stack_amount: u32,
    totem: [u32; MAX_SPELL_TOTEMS], // Non-consumable items required to cast the spell (e.g. Blacksmith Hammer)
    reagent: [i32; MAX_SPELL_REAGENTS],
    reagent_count: [u32; MAX_SPELL_REAGENTS],
    equipped_item_class: i32, // ItemTemplate.class that caster must have in hand
    equipped_item_sub_class_mask: i32, // Same with ItemTemplate.subclass (mask)
    equipped_item_inventory_type_mask: i32, // Same with ItemTemplate.inventory_type (mask)
    effect: [i32; MAX_SPELL_EFFECT_INDEX],
    effect_die_sides: [i32; MAX_SPELL_EFFECT_INDEX], // Number of side of dices rolled for random value
    effect_base_dice: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_dice_per_level: [f32; MAX_SPELL_EFFECT_INDEX],
    effect_real_points_per_level: [f32; MAX_SPELL_EFFECT_INDEX],
    effect_base_points: [i32; MAX_SPELL_EFFECT_INDEX],
    effect_mechanic: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_implicit_target_a: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_implicit_target_b: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_radius_index: [u32; MAX_SPELL_EFFECT_INDEX], // SpellRadius.dbc
    effect_apply_aura_name: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_amplitude: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_multiple_value: [f32; MAX_SPELL_EFFECT_INDEX],
    effect_chain_target: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_item_type: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_misc_value: [i32; MAX_SPELL_EFFECT_INDEX],
    effect_misc_value_b: [i32; MAX_SPELL_EFFECT_INDEX],
    effect_trigger_spell: [u32; MAX_SPELL_EFFECT_INDEX],
    effect_points_per_combo_point: [f32; MAX_SPELL_EFFECT_INDEX],
    spell_visual: u32, // SpellVisual.dbc
    // SpellVisual2: u32
    spell_icon_id: u32,
    active_icon_id: u32,
    // spellPriority: u32
    name: String,
    // SpellNameFlag: u32
    rank: String,
    // RankFlags: u32
    // Description: String
    // DescriptionFlags: u32
    // ToolTip: String,
    // ToolTipFlags: u32
    mana_cost_percentage: u32,
    start_recovery_category: u32,
    start_recovery_time: u32,
    max_target_level: u32,
    spell_family_name: u32,
    spell_family_flags: u64,
    max_affected_targets: u32,
    damage_class: u32,
    prevention_type: u32,
    // StanceBarOrder: u32
    damage_multiplier: [f32; MAX_SPELL_EFFECT_INDEX],
    // MinFactionId: u32
    // MinReputation: u32
    // RequiredAuraVision: u32
    totem_category: [u32; MAX_SPELL_TOTEMS],
    area_id: u32,
    school_mask: u32, // 215      m_schoolMask
}

impl DbcTypedRecord for SpellRecord {
    fn from_record(record: &DbcRecord, strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = SpellRecord {
                category: record.fields[1].as_u32,
                dispel_type: record.fields[3].as_u32,
                mechanic: record.fields[4].as_u32,
                attributes: record.fields[5].as_u32,
                attributes_ex: record.fields[6].as_u32,
                attributes_ex_b: record.fields[7].as_u32,
                attributes_ex_c: record.fields[8].as_u32,
                attributes_ex_d: record.fields[9].as_u32,
                attributes_ex_e: record.fields[10].as_u32,
                attributes_ex_f: record.fields[11].as_u32,
                shapeshift_mask: record.fields[12].as_u32,
                shapeshift_mask_excluded: record.fields[13].as_u32,
                targets: record.fields[14].as_u32,
                target_creature_type_mask: record.fields[15].as_u32,
                spell_focus_object: record.fields[16].as_u32,
                facing_caster_flags: record.fields[17].as_u32,
                caster_aura_state: record.fields[18].as_u32,
                target_aura_state: record.fields[19].as_u32,
                caster_aura_state_not: record.fields[20].as_u32,
                target_aura_state_not: record.fields[21].as_u32,
                casting_time_index: record.fields[22].as_u32,
                recovery_time: record.fields[23].as_u32,
                category_recovery_time: record.fields[24].as_u32,
                interrupt_flags: record.fields[25].as_u32,
                aura_interrupt_flags: record.fields[26].as_u32,
                channel_interrupt_flags: record.fields[27].as_u32,
                proc_flags: record.fields[28].as_u32,
                proc_chance: record.fields[29].as_u32,
                proc_charges: record.fields[30].as_u32,
                max_level: record.fields[31].as_u32,
                base_level: record.fields[32].as_u32,
                spell_level: record.fields[33].as_u32,
                duration_index: record.fields[34].as_u32,
                power_type: record.fields[35].as_u32,
                mana_cost: record.fields[36].as_u32,
                mana_cost_perlevel: record.fields[37].as_u32,
                mana_per_second: record.fields[38].as_u32,
                mana_per_second_per_level: record.fields[39].as_u32,
                range_index: record.fields[40].as_u32,
                speed: record.fields[41].as_f32,
                stack_amount: record.fields[43].as_u32,
                totem: [record.fields[44].as_u32, record.fields[45].as_u32],
                reagent: [
                    record.fields[46].as_i32,
                    record.fields[47].as_i32,
                    record.fields[48].as_i32,
                    record.fields[49].as_i32,
                    record.fields[50].as_i32,
                    record.fields[51].as_i32,
                    record.fields[52].as_i32,
                    record.fields[53].as_i32,
                ],
                reagent_count: [
                    record.fields[54].as_u32,
                    record.fields[55].as_u32,
                    record.fields[56].as_u32,
                    record.fields[57].as_u32,
                    record.fields[58].as_u32,
                    record.fields[59].as_u32,
                    record.fields[60].as_u32,
                    record.fields[61].as_u32,
                ],
                equipped_item_class: record.fields[62].as_i32,
                equipped_item_sub_class_mask: record.fields[63].as_i32,
                equipped_item_inventory_type_mask: record.fields[64].as_i32,
                effect: [
                    record.fields[65].as_i32,
                    record.fields[66].as_i32,
                    record.fields[67].as_i32,
                ],
                effect_die_sides: [
                    record.fields[68].as_i32,
                    record.fields[69].as_i32,
                    record.fields[70].as_i32,
                ],
                effect_base_dice: [
                    record.fields[71].as_u32,
                    record.fields[72].as_u32,
                    record.fields[73].as_u32,
                ],
                effect_dice_per_level: [
                    record.fields[74].as_f32,
                    record.fields[75].as_f32,
                    record.fields[76].as_f32,
                ],
                effect_real_points_per_level: [
                    record.fields[77].as_f32,
                    record.fields[78].as_f32,
                    record.fields[79].as_f32,
                ],
                effect_base_points: [
                    record.fields[80].as_i32,
                    record.fields[81].as_i32,
                    record.fields[82].as_i32,
                ],
                effect_mechanic: [
                    record.fields[83].as_u32,
                    record.fields[84].as_u32,
                    record.fields[85].as_u32,
                ],
                effect_implicit_target_a: [
                    record.fields[86].as_u32,
                    record.fields[87].as_u32,
                    record.fields[88].as_u32,
                ],
                effect_implicit_target_b: [
                    record.fields[89].as_u32,
                    record.fields[90].as_u32,
                    record.fields[91].as_u32,
                ],
                effect_radius_index: [
                    record.fields[92].as_u32,
                    record.fields[93].as_u32,
                    record.fields[94].as_u32,
                ],
                effect_apply_aura_name: [
                    record.fields[95].as_u32,
                    record.fields[96].as_u32,
                    record.fields[97].as_u32,
                ],
                effect_amplitude: [
                    record.fields[98].as_u32,
                    record.fields[99].as_u32,
                    record.fields[100].as_u32,
                ],
                effect_multiple_value: [
                    record.fields[101].as_f32,
                    record.fields[102].as_f32,
                    record.fields[103].as_f32,
                ],
                effect_chain_target: [
                    record.fields[104].as_u32,
                    record.fields[105].as_u32,
                    record.fields[106].as_u32,
                ],
                effect_item_type: [
                    record.fields[107].as_u32,
                    record.fields[108].as_u32,
                    record.fields[109].as_u32,
                ],
                effect_misc_value: [
                    record.fields[110].as_i32,
                    record.fields[111].as_i32,
                    record.fields[112].as_i32,
                ],
                effect_misc_value_b: [
                    record.fields[113].as_i32,
                    record.fields[114].as_i32,
                    record.fields[115].as_i32,
                ],
                effect_trigger_spell: [
                    record.fields[116].as_u32,
                    record.fields[117].as_u32,
                    record.fields[118].as_u32,
                ],
                effect_points_per_combo_point: [
                    record.fields[119].as_f32,
                    record.fields[120].as_f32,
                    record.fields[121].as_f32,
                ],
                spell_visual: record.fields[122].as_u32,
                spell_icon_id: record.fields[124].as_u32,
                active_icon_id: record.fields[125].as_u32,
                name: strings
                    .get(record.fields[127].as_u32 as usize)
                    .expect("name string not found in Spell.dbc"),
                rank: strings
                    .get(record.fields[144].as_u32 as usize)
                    .expect("rank string not found in Spell.dbc"),
                mana_cost_percentage: record.fields[195].as_u32,
                start_recovery_category: record.fields[196].as_u32,
                start_recovery_time: record.fields[197].as_u32,
                max_target_level: record.fields[198].as_u32,
                spell_family_name: record.fields[199].as_u32,
                spell_family_flags: record.fields[200].as_u32 as u64
                    | ((record.fields[201].as_u32 as u64) << 32),
                max_affected_targets: record.fields[202].as_u32,
                damage_class: record.fields[203].as_u32,
                prevention_type: record.fields[204].as_u32,
                damage_multiplier: [
                    record.fields[206].as_f32,
                    record.fields[207].as_f32,
                    record.fields[208].as_f32,
                ],
                totem_category: [record.fields[212].as_u32, record.fields[213].as_u32],
                area_id: record.fields[214].as_u32,
                school_mask: record.fields[215].as_u32,
            };

            (key, record)
        }
    }
}
