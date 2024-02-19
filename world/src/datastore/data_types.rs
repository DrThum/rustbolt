use std::{sync::Arc, time::Duration};

use binrw::binwrite;
use enumflags2::BitFlags;
use enumn::N;
use rand::distributions::WeightedIndex;

use crate::{
    ecs::components::movement::MovementKind,
    game::value_range::ValueRange,
    protocol::packets::{
        ItemTemplateDamage, ItemTemplateSocket, ItemTemplateSpell, ItemTemplateStat,
    },
    shared::constants::{
        AbilityLearnType, ActionButtonType, CharacterClass, CharacterClassBit, CharacterRaceBit,
        CreatureRank, Expansion, InventoryType, MapType, QuestFlag, SkillCategory, SkillRangeType,
        SkillType, SpellEffect, FACTION_NUMBER_BASE_REPUTATION_MASKS,
        MAX_QUEST_CHOICE_REWARDS_COUNT, MAX_QUEST_OBJECTIVES_COUNT, MAX_QUEST_REWARDS_COUNT,
        MAX_QUEST_REWARDS_REPUT_COUNT, MAX_SPELL_EFFECTS, MAX_SPELL_REAGENTS, MAX_SPELL_TOTEMS,
        NPC_TEXT_EMOTE_COUNT, NPC_TEXT_TEXT_COUNT,
    },
    DataStore,
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

pub struct PlayerCreateSpell {
    pub race: u32,
    pub class: u32,
    pub spell_id: u32,
}

pub struct PlayerCreateActionButton {
    pub race: u32,
    pub class: u32,
    pub position: u32,
    pub action_type: ActionButtonType,
    pub action_value: u32,
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
    pub expansion: Expansion,
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
                expansion: Expansion::n(record.fields[124].as_u32)
                    .expect("Invalid expansion found in Map.dbc"),
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

    pub fn is_continent(&self) -> bool {
        match self.id {
            0 | 1 | 530 => true,
            _ => false,
        }
    }

    pub fn is_dungeon(&self) -> bool {
        match self.map_type {
            MapType::Instance | MapType::Raid => true,
            _ => false,
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
#[derive(Clone)]
pub struct CreatureTemplate {
    pub entry: u32,
    pub name: String,
    pub sub_name: Option<String>,
    pub icon_name: Option<String>,
    pub expansion: usize,
    pub unit_class: CharacterClass, // Can only be Warrior (1), Paladin (2), Rogue (4) or Mage (8)
    pub min_level: u32,
    pub max_level: u32,
    pub health_multiplier: f32,
    pub power_multiplier: f32,
    pub damage_multiplier: f32,
    pub armor_multiplier: f32,
    pub experience_multiplier: f32,
    pub melee_base_attack_time: Duration,
    pub ranged_base_attack_time: Duration,
    pub base_damage_variance: f32,
    pub model_ids: Vec<u32>,
    pub scale: f32,
    pub speed_walk: f32,
    pub speed_run: f32,
    pub family: u32,  // CreatureFamily.dbc
    pub type_id: u32, // CreatureType.dbc
    pub type_flags: u32,
    pub rank: CreatureRank,
    pub racial_leader: u8,        // bool
    pub pet_spell_data_id: u32,   // CreatureSpellData.dbc
    pub faction_template_id: u32, // FactionTemplate.dbc
    pub npc_flags: u32,
    pub unit_flags: u32,
    pub dynamic_flags: u32,
    pub gossip_menu_id: Option<u32>,
    pub movement_type: MovementKind,
    pub min_money_loot: u32,
    pub max_money_loot: u32,
    pub loot_table_id: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
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
    pub effect: [i32; MAX_SPELL_EFFECTS],
    effect_die_sides: [i32; MAX_SPELL_EFFECTS], // Number of side of dices rolled for random value
    effect_base_dice: [u32; MAX_SPELL_EFFECTS],
    effect_dice_per_level: [f32; MAX_SPELL_EFFECTS],
    effect_real_points_per_level: [f32; MAX_SPELL_EFFECTS],
    effect_base_points: [i32; MAX_SPELL_EFFECTS],
    effect_mechanic: [u32; MAX_SPELL_EFFECTS],
    effect_implicit_target_a: [u32; MAX_SPELL_EFFECTS],
    effect_implicit_target_b: [u32; MAX_SPELL_EFFECTS],
    effect_radius_index: [u32; MAX_SPELL_EFFECTS], // SpellRadius.dbc
    effect_apply_aura_name: [u32; MAX_SPELL_EFFECTS],
    effect_amplitude: [u32; MAX_SPELL_EFFECTS],
    effect_multiple_value: [f32; MAX_SPELL_EFFECTS],
    effect_chain_target: [u32; MAX_SPELL_EFFECTS],
    effect_item_type: [u32; MAX_SPELL_EFFECTS],
    effect_misc_value: [i32; MAX_SPELL_EFFECTS],
    effect_misc_value_b: [i32; MAX_SPELL_EFFECTS],
    effect_trigger_spell: [u32; MAX_SPELL_EFFECTS],
    effect_points_per_combo_point: [f32; MAX_SPELL_EFFECTS],
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
    damage_multiplier: [f32; MAX_SPELL_EFFECTS],
    // MinFactionId: u32
    // MinReputation: u32
    // RequiredAuraVision: u32
    totem_category: [u32; MAX_SPELL_TOTEMS],
    area_id: u32,
    school_mask: u32,
}

impl SpellRecord {
    pub fn learnable_skill(&self) -> Option<LearnableSkillFromSpell> {
        for index in 0..MAX_SPELL_EFFECTS {
            if SpellEffect::n(self.effect[index]) == Some(SpellEffect::Skill) {
                let skill_id = self.effect_misc_value[index] as u32;
                let step = self.calc_simple_value(index) as u32;
                let value = match SkillType::n(skill_id) {
                    Some(SkillType::Riding) => step * 75,
                    _ => 1,
                };

                return Some(LearnableSkillFromSpell {
                    skill_id,
                    step,
                    value,
                    max_value: step * 75,
                });
            }
        }

        None
    }

    pub fn calc_simple_value(&self, effect_index: usize) -> i32 {
        assert!(
            effect_index < MAX_SPELL_EFFECTS,
            "effect_index must be [0; MAX_SPELL_EFFECT_INDEX["
        );

        self.effect_base_points[effect_index] + (self.effect_base_dice[effect_index] as i32)
    }

    pub fn base_duration(&self, data_store: Arc<DataStore>) -> Option<Duration> {
        data_store
            .get_spell_duration_record(self.duration_index)
            .map(|rec| rec.base)
    }

    // TODO: Improve this by loading a ref to the SpellCastTimeRecord directly
    pub fn base_cast_time(&self, data_store: Arc<DataStore>) -> Option<Duration> {
        data_store
            .get_spell_cast_times(self.casting_time_index)
            .map(|rec| rec.base)
    }
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

#[allow(dead_code)]
pub struct SpellDurationRecord {
    pub base: Duration,
    pub per_level: Duration,
    pub max: Duration,
}

impl DbcTypedRecord for SpellDurationRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;
            let record = SpellDurationRecord {
                base: Duration::from_millis(record.fields[1].as_u32 as u64),
                per_level: Duration::from_millis(record.fields[2].as_u32 as u64),
                max: Duration::from_millis(record.fields[3].as_u32 as u64),
            };

            (key, record)
        }
    }
}

#[allow(dead_code)]
pub struct SpellCastTimeRecord {
    pub base: Duration,
    pub per_level: Duration,
    pub min: Duration,
}

impl DbcTypedRecord for SpellCastTimeRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;
            let record = SpellCastTimeRecord {
                base: Duration::from_millis(record.fields[1].as_u32 as u64),
                per_level: Duration::from_millis(record.fields[2].as_u32 as u64),
                min: Duration::from_millis(record.fields[3].as_u32 as u64),
            };

            (key, record)
        }
    }
}

pub struct LearnableSkillFromSpell {
    pub skill_id: u32,
    pub step: u32,
    pub value: u32,
    pub max_value: u32,
}

pub struct SkillLineRecord {
    pub id: SkillType,
    pub category: SkillCategory,
    // skill_cost_id: u32
    pub name: String,
    pub spell_icon: u32,
}

impl SkillLineRecord {
    pub fn range_type(&self) -> SkillRangeType {
        match self.category {
            SkillCategory::Languages => SkillRangeType::Language,
            SkillCategory::Weapon if self.id == SkillType::FistWeapons => SkillRangeType::Mono,
            SkillCategory::Weapon => SkillRangeType::Level,
            SkillCategory::Armor | SkillCategory::Class
                if self.id == SkillType::Poisons || self.id == SkillType::Lockpicking =>
            {
                SkillRangeType::Level
            }
            SkillCategory::Armor | SkillCategory::Class => SkillRangeType::Mono,
            // MaNGOS does some weird calculation for the next one, check
            // ObjectMgr::GetSkillRangeType if in doubt
            SkillCategory::PrimaryProfession | SkillCategory::SecondaryProfession => {
                SkillRangeType::Rank
            }
            _ => SkillRangeType::None,
        }
    }
}

impl DbcTypedRecord for SkillLineRecord {
    fn from_record(record: &DbcRecord, strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = SkillLineRecord {
                id: SkillType::n(record.fields[0].as_u32)
                    .expect("invalid skill_id in SkillLine.dbc"),
                category: SkillCategory::n(record.fields[1].as_i32).expect(&format!(
                    "invalid skill_category_id {} in SkillLine.dbc (record {})",
                    record.fields[1].as_i32, record.fields[0].as_u32
                )),
                name: strings
                    .get(record.fields[3].as_u32 as usize)
                    .expect("invalid name found in SkillLine.dbc"),
                spell_icon: record.fields[37].as_u32,
            };

            (key, record)
        }
    }
}

#[derive(Clone)]
pub struct SkillLineAbilityRecord {
    pub id: u32,
    pub skill_id: SkillType,
    pub spell_id: u32,
    pub race_mask: BitFlags<CharacterRaceBit>,
    pub class_mask: BitFlags<CharacterClassBit>,
    pub required_skill_value: u32,
    pub forward_spell_id: u32,
    pub learn_on_get_skill: AbilityLearnType,
    pub max_value: u32,
    pub min_value: u32,
    pub required_train_points: u32,
}

impl DbcTypedRecord for SkillLineAbilityRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let mut record = SkillLineAbilityRecord {
                id: record.fields[0].as_u32,
                skill_id: SkillType::n(record.fields[1].as_u32)
                    .expect("invalid skill_type in SkillLineAbility.dbc"),
                spell_id: record.fields[2].as_u32,
                race_mask: BitFlags::from_bits_unchecked(record.fields[3].as_u32),
                class_mask: BitFlags::from_bits_unchecked(record.fields[4].as_u32),
                required_skill_value: record.fields[7].as_u32,
                forward_spell_id: record.fields[8].as_u32,
                learn_on_get_skill: AbilityLearnType::n(record.fields[9].as_u32).expect(&format!(
                    "invalid learn_on_get_skill {} found in SkillLineAbility.dbc",
                    record.fields[9].as_u32
                )),
                max_value: record.fields[10].as_u32,
                min_value: record.fields[11].as_u32,
                required_train_points: record.fields[14].as_u32,
            };

            // Client is missing some data
            if record.skill_id == SkillType::Poisons && record.max_value == 0 {
                record.learn_on_get_skill = AbilityLearnType::LearnedOnGetRaceOrClassSkill;
            }
            if record.skill_id == SkillType::Lockpicking && record.max_value == 0 {
                record.learn_on_get_skill = AbilityLearnType::LearnedOnGetRaceOrClassSkill;
            }

            (key, record)
        }
    }
}

pub struct FactionRecord {
    pub position_in_reputation_list: i32,
    base_reputation_race_mask: [BitFlags<CharacterRaceBit>; FACTION_NUMBER_BASE_REPUTATION_MASKS],
    base_reputation_class_mask: [BitFlags<CharacterClassBit>; FACTION_NUMBER_BASE_REPUTATION_MASKS],
    base_reputation_standing: [i32; FACTION_NUMBER_BASE_REPUTATION_MASKS],
    reputation_flags: [u32; FACTION_NUMBER_BASE_REPUTATION_MASKS],
    pub team: u32,
    pub name: String,
}

impl FactionRecord {
    pub fn base_reputation_standing(
        &self,
        race: CharacterRaceBit,
        class: CharacterClassBit,
    ) -> Option<i32> {
        for index in 0..FACTION_NUMBER_BASE_REPUTATION_MASKS {
            let race_ok = self.base_reputation_race_mask[index].intersects(race);
            let class_ok = self.base_reputation_class_mask[index].intersects(class);

            if race_ok || class_ok {
                return Some(self.base_reputation_standing[index]);
            }
        }

        None
    }

    pub fn reputation_flags(
        &self,
        race: CharacterRaceBit,
        class: CharacterClassBit,
    ) -> Option<u32> {
        for index in 0..FACTION_NUMBER_BASE_REPUTATION_MASKS {
            let race_ok = self.base_reputation_race_mask[index].intersects(race);
            let class_ok = self.base_reputation_class_mask[index].intersects(class);

            if race_ok || class_ok {
                return Some(self.reputation_flags[index]);
            }
        }

        None
    }
}

impl DbcTypedRecord for FactionRecord {
    fn from_record(record: &DbcRecord, strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = FactionRecord {
                position_in_reputation_list: record.fields[1].as_i32,
                base_reputation_race_mask: [
                    BitFlags::from_bits_unchecked(record.fields[2].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[3].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[4].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[5].as_u32),
                ],
                base_reputation_class_mask: [
                    BitFlags::from_bits_unchecked(record.fields[6].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[7].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[8].as_u32),
                    BitFlags::from_bits_unchecked(record.fields[9].as_u32),
                ],
                base_reputation_standing: [
                    record.fields[10].as_i32,
                    record.fields[11].as_i32,
                    record.fields[12].as_i32,
                    record.fields[13].as_i32,
                ],
                reputation_flags: [
                    record.fields[14].as_u32,
                    record.fields[15].as_u32,
                    record.fields[16].as_u32,
                    record.fields[17].as_u32,
                ],
                team: record.fields[18].as_u32,
                name: strings
                    .get(record.fields[19].as_u32 as usize)
                    .expect("invalid name found in Faction.dbc"),
            };

            (key, record)
        }
    }
}

// https://www.azerothcore.org/wiki/factiontemplate
#[allow(dead_code)]
pub struct FactionTemplateRecord {
    pub id: u32,
    faction_id: u32,
    faction_flags: u32, // TODO: BitFlags
    faction_group_mask: u32,
    friend_group_mask: u32,
    enemy_group_mask: u32,
    enemies: [u32; 4],
    friends: [u32; 4],
}

#[allow(dead_code)]
impl FactionTemplateRecord {
    pub fn is_friendly_to(&self, other: &Self) -> bool {
        if other.faction_id != 0 {
            if self.enemies.contains(&other.faction_id) {
                return false;
            } else if self.friends.contains(&other.faction_id) {
                return true;
            }
        }

        let other_is_friend_with_us = (self.friend_group_mask & other.faction_group_mask) != 0;
        let we_are_friends_with_other = (self.faction_group_mask & other.friend_group_mask) != 0;

        other_is_friend_with_us || we_are_friends_with_other
    }

    pub fn is_hostile_to(&self, other: &Self) -> bool {
        if other.faction_id != 0 {
            if self.enemies.contains(&other.faction_id) {
                return true;
            } else if self.friends.contains(&other.faction_id) {
                return false;
            }
        }

        (self.enemy_group_mask & other.faction_group_mask) != 0
    }

    // TODO: a faction is not neutral to all if it has a position_in_reputation_list
    pub fn is_neutral_to_all(&self) -> bool {
        if self.enemies.iter().any(|&enemy| enemy != 0) {
            return false;
        }

        self.faction_group_mask == 0 && self.enemy_group_mask == 0
    }
}

impl DbcTypedRecord for FactionTemplateRecord {
    fn from_record(record: &DbcRecord, _strings: &DbcStringBlock) -> (u32, Self) {
        unsafe {
            let key = record.fields[0].as_u32;

            let record = FactionTemplateRecord {
                id: record.fields[0].as_u32,
                faction_id: record.fields[1].as_u32,
                faction_flags: record.fields[2].as_u32,
                faction_group_mask: record.fields[3].as_u32,
                friend_group_mask: record.fields[4].as_u32,
                enemy_group_mask: record.fields[5].as_u32,
                enemies: [
                    record.fields[6].as_u32,
                    record.fields[7].as_u32,
                    record.fields[8].as_u32,
                    record.fields[9].as_u32,
                ],
                friends: [
                    record.fields[10].as_u32,
                    record.fields[11].as_u32,
                    record.fields[12].as_u32,
                    record.fields[13].as_u32,
                ],
            };

            (key, record)
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct QuestTemplate {
    pub entry: u32,
    pub method: u32, // 0, 1 or 2 - not used server-side
    pub zone_or_sort: i32,
    pub min_level: u32,
    pub level: i32, // Quest level
    pub type_: u32,
    pub required_classes: BitFlags<CharacterClassBit>,
    pub required_races: BitFlags<CharacterRaceBit>,
    pub required_skill: u32,
    pub required_skill_value: u32,
    pub rep_objective_faction: u32,
    pub rep_objective_value: u32,
    pub required_min_rep_faction: u32,
    pub required_min_rep_value: u32,
    pub required_max_rep_faction: u32,
    pub required_max_rep_value: u32,
    pub suggested_players: u32,
    pub time_limit: Option<Duration>,
    pub flags: BitFlags<QuestFlag>,
    pub special_flags: u32,
    pub character_title: u32,
    pub previous_quest_id: i32,
    pub next_quest_id: i32,
    pub exclusive_group: i32,
    pub next_quest_in_chain: u32,
    pub source_item_id: u32,
    pub source_item_count: u32,
    pub source_spell: u32,
    pub title: Option<String>,
    pub details: Option<String>,
    pub objectives: Option<String>,
    pub offer_reward_text: Option<String>,
    pub request_items_text: Option<String>,
    pub end_text: Option<String>,
    pub objective_text1: Option<String>,
    pub objective_text2: Option<String>,
    pub objective_text3: Option<String>,
    pub objective_text4: Option<String>,
    pub required_item_ids: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    pub required_item_counts: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    pub required_source_item_ids: [u32; MAX_QUEST_OBJECTIVES_COUNT], // Item required to make the req item
    pub required_source_item_counts: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    // Creature or GameObject >0 = creature_template <0 = gameobject_template
    pub required_entity_ids: [i32; MAX_QUEST_OBJECTIVES_COUNT],
    pub required_entity_counts: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    pub required_spell_casts: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    pub reward_choice_item_ids: [u32; MAX_QUEST_CHOICE_REWARDS_COUNT],
    pub reward_choice_item_counts: [u32; MAX_QUEST_CHOICE_REWARDS_COUNT],
    pub reward_item_ids: [u32; MAX_QUEST_REWARDS_COUNT],
    pub reward_item_counts: [u32; MAX_QUEST_REWARDS_COUNT],
    pub reward_rep_factions: [u32; MAX_QUEST_REWARDS_REPUT_COUNT],
    pub reward_rep_values: [i32; MAX_QUEST_REWARDS_REPUT_COUNT],
    pub reward_honorable_kills: u32,
    // >0: rewarded money - <0 money required to start the quest
    pub required_or_reward_money: i32,
    pub reward_money_max_level: u32,
    pub reward_spell: u32,
    pub reward_spell_cast: u32,
    pub reward_mail_template_id: u32,
    pub reward_mail_delay_seconds: u32,
    pub point_map_id: u32, // Point of Interest
    pub point_x: f32,
    pub point_y: f32,
    pub point_opt: u32,
    pub details_emote1: u32,
    pub details_emote2: u32,
    pub details_emote3: u32,
    pub details_emote4: u32,
    pub details_emote_delay1: u32,
    pub details_emote_delay2: u32,
    pub details_emote_delay3: u32,
    pub details_emote_delay4: u32,
    pub incomplete_emote: u32,
    pub complete_emote: u32,
    pub offer_reward_emotes: [u32; MAX_QUEST_OBJECTIVES_COUNT],
    pub offer_reward_emote_delays: [u32; MAX_QUEST_OBJECTIVES_COUNT],
}

impl QuestTemplate {
    pub fn reward_choice_items(&self) -> Vec<(u32, u32)> {
        self.reward_choice_item_ids
            .iter()
            .zip(self.reward_choice_item_counts.iter())
            .map(|(id, count)| (id.clone(), count.clone()))
            .collect()
    }

    pub fn reward_items(&self) -> Vec<(u32, u32)> {
        self.reward_item_ids
            .iter()
            .zip(self.reward_item_counts.iter())
            .map(|(id, count)| (id.clone(), count.clone()))
            .collect()
    }

    pub fn experience_reward_at_level(&self, player_level: u32) -> u32 {
        let rmml = self.reward_money_max_level as f32;
        let quest_level = self.level.max(0) as u32;

        if rmml > 0. {
            let full_xp = match self.level {
                lvl if lvl >= 65 => rmml / 6.,
                64 => rmml / 4.8,
                63 => rmml / 3.6,
                62 => rmml / 2.4,
                61 => rmml / 1.2,
                lvl if lvl > 0 => rmml / 0.6,
                _ => 0.,
            };

            let level_difference =
                player_level.saturating_sub(quest_level).saturating_sub(5) as f32;
            let penalty = (1. - (level_difference * 0.2)).max(0.1);
            (full_xp * penalty).ceil() as u32
        } else {
            0
        }
    }

    // Previous quest id if previous_quest_id > 0
    // Player must have completed the previous quest before accepting this one
    pub fn previous_quest_id(&self) -> Option<u32> {
        match self.previous_quest_id {
            id if id <= 0 => None,
            id => Some(id as u32),
        }
    }

    // Parent quest id if previous_quest_id < 0
    // Player must have the parent quest active to accept this one
    pub fn parent_quest_id(&self) -> Option<u32> {
        match self.previous_quest_id {
            id if id >= 0 => None,
            id => Some(-id as u32),
        }
    }

    pub fn creature_requirements(&self, creature_id: u32) -> Option<(usize, u32)> {
        for (index, req_ent_id) in self.required_entity_ids.iter().enumerate() {
            if *req_ent_id as u32 == creature_id {
                return Some((index, self.required_entity_counts[index]));
            }
        }

        return None;
    }
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(N, Clone, Copy)]
pub enum QuestActorType {
    Creature = 0,
    GameObject = 1,
    AreaTrigger = 2,
}

#[allow(dead_code)]
#[repr(u8)]
#[derive(N, Clone, Copy, PartialEq)]
pub enum QuestActorRole {
    Start = 0,
    End = 1,
}

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct QuestRelation {
    pub actor_type: QuestActorType,
    pub actor_entry: u32,
    pub quest_id: u32,
    pub role: QuestActorRole,
}

#[allow(dead_code)]
pub struct NpcTextDbRecord {
    pub id: u32,
    pub texts: [NpcText; NPC_TEXT_TEXT_COUNT],
}

#[allow(dead_code)]
pub struct NpcText {
    pub text_male: Option<String>,
    pub text_female: Option<String>,
    pub language: u32,
    pub probability: f32,
    pub emotes: [NpcTextEmote; NPC_TEXT_EMOTE_COUNT],
}

#[allow(dead_code)]
#[binwrite]
#[derive(Clone)]
pub struct NpcTextEmote {
    pub delay: u32,
    pub emote: u32,
}

pub struct GossipMenuDbRecord {
    pub id: u32,
    pub text_id: u32,
    pub options: Vec<GossipMenuOption>,
}

pub struct GossipMenuOption {
    pub id: u32,
    pub icon: u32,
    pub text: Option<String>,
    pub option_id: u32,
    pub npc_option_npcflag: u32,
    pub action_menu_id: i32,
    pub action_poi_id: u32,
    pub box_coded: bool,
    pub box_money: u32,
    pub box_text: Option<String>,
}

pub struct LootTable {
    pub id: u32,
    // description: Option<String>,
    pub groups: Vec<LootGroup>,
}

pub struct LootGroup {
    pub chance: f32, // TODO: Make it a type?
    pub num_rolls: ValueRange<u8>,
    pub items: Vec<LootItem>,
    pub condition_id: Option<u32>,
    pub distribution: WeightedIndex<f32>,
}

#[derive(Copy, Clone)]
pub struct LootItem {
    pub item_id: u32, // item_templates.entry
    pub chance: f32,  // TODO: Make it a type?
    pub count: ValueRange<u8>,
    pub condition_id: Option<u32>,
}
