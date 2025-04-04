#![allow(non_snake_case)]

use data_types::AreaTableRecord;
use indicatif::ProgressBar;
use log::info;
use multimap::MultiMap;
use shared::{models::loot::LootTable, repositories::loot::LootRepository};
use std::{
    collections::{hash_map::Values, HashMap},
    sync::Arc,
};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{
    config::WorldConfig,
    datastore::{
        data_types::{CreatureTemplate, NpcTextDbRecord, PlayerCreatePosition, QuestActorType},
        dbc::Dbc,
    },
    repositories::{
        creature::{CreatureModelInfo, CreatureRepository},
        creature_static_data::{
            CreatureBaseAttributesPerLevelDbRecord, CreatureStaticDataRepository,
        },
        game_object::GameObjectRepository,
        gossip::GossipRepository,
        item::ItemRepository,
        player_static_data::{
            PlayerBaseAttributesPerLevelDbRecord, PlayerBaseHealthManaPerLevelDbRecord,
            PlayerExperiencePerLevel, PlayerStaticDataRepository,
        },
        quest::QuestRepository,
    },
    shared::constants::{
        CharacterClass, CharacterClassBit, CharacterRace, CharacterRaceBit, PowerType,
        MAX_BASE_POWER_ENERGY, MAX_BASE_POWER_FOCUS, MAX_BASE_POWER_PET_HAPPINESS,
        MAX_BASE_POWER_RAGE,
    },
};

use self::data_types::{
    CharStartOutfitRecord, ChrClassesRecord, ChrRacesRecord, EmotesTextRecord, FactionRecord,
    FactionTemplateRecord, GameObjectTemplate, GameTableOCTRegenHPRecord,
    GameTableRegenHPPerSptRecord, GameTableRegenMPPerSptRecord, GossipMenuDbRecord, ItemRecord,
    ItemTemplate, MapRecord, PlayerCreateActionButton, QuestRelation, QuestTemplate,
    SkillLineAbilityRecord, SkillLineRecord, SpellCastTimeRecord, SpellDurationRecord, SpellRecord,
};

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;
pub type DbcMultiStore<T> = MultiMap<u32, T>;
pub type GameTableStore<T> = Vec<T>;
pub type SqlStore<T> = HashMap<u32, T>;
pub type SqlMultiStore<T> = MultiMap<u32, T>;

pub struct DataStore {
    // DBCs
    chr_races: DbcStore<ChrRacesRecord>,
    chr_classes: DbcStore<ChrClassesRecord>,
    char_start_outfit: DbcStore<CharStartOutfitRecord>,
    item: DbcStore<ItemRecord>,
    map: DbcStore<MapRecord>,
    emotes_text: DbcStore<EmotesTextRecord>,
    spell: DbcStore<SpellRecord>,
    spells_by_category: HashMap<u32, Vec<u32>>,
    spell_duration: DbcStore<SpellDurationRecord>,
    spell_cast_times: DbcStore<SpellCastTimeRecord>,
    skill_line: DbcStore<SkillLineRecord>,
    skill_line_ability: DbcStore<SkillLineAbilityRecord>,
    skill_line_ability_by_spell: DbcMultiStore<SkillLineAbilityRecord>,
    faction: DbcStore<FactionRecord>,
    faction_template: DbcStore<FactionTemplateRecord>,
    area_table: DbcStore<AreaTableRecord>,
    // SQL tables
    item_templates: SqlStore<ItemTemplate>,
    player_create_positions: SqlStore<PlayerCreatePosition>,
    player_create_spells: SqlMultiStore<u32>,
    player_create_action_buttons: SqlMultiStore<PlayerCreateActionButton>,
    creature_templates: SqlStore<CreatureTemplate>,
    quest_templates: SqlStore<QuestTemplate>,
    quest_relations_by_creature: SqlMultiStore<QuestRelation>,
    quest_relations_by_game_object: SqlMultiStore<QuestRelation>,
    npc_texts: SqlStore<NpcTextDbRecord>,
    gossip_menus: SqlMultiStore<GossipMenuDbRecord>,
    creature_model_info: SqlStore<CreatureModelInfo>,
    player_base_health_mana: SqlStore<PlayerBaseHealthManaPerLevelDbRecord>,
    player_base_attributes: SqlStore<PlayerBaseAttributesPerLevelDbRecord>,
    creature_base_attributes: SqlStore<CreatureBaseAttributesPerLevelDbRecord>,
    player_experience_per_level: SqlStore<PlayerExperiencePerLevel>,
    loot_tables: SqlStore<LootTable>,
    game_object_templates: SqlStore<GameObjectTemplate>,
    // GameTables (DBC files with name starting with gtXXX)
    gt_OCTRegenHP: GameTableStore<GameTableOCTRegenHPRecord>,
    gt_RegenHPPerSpt: GameTableStore<GameTableRegenHPPerSptRecord>,
    gt_RegenMPPerSpt: GameTableStore<GameTableRegenMPPerSptRecord>,
}

macro_rules! parse_dbc {
    ($config_dir:expr, $dbc_name:expr) => {{
        info!("{}", format!("Loading {}.dbc...", $dbc_name));
        let dbc = Dbc::parse(format!("{}/dbcs/{}.dbc", $config_dir, $dbc_name))?;
        let bar = ProgressBar::new(dbc.length() as u64);
        let store = dbc.as_store(&bar);
        bar.finish();
        store
    }};
}

macro_rules! parse_game_table {
    ($config_dir:expr, $dbc_name:expr) => {{
        info!("{}", format!("Loading GameTable {}.dbc...", $dbc_name));
        let dbc = Dbc::parse(format!("{}/dbcs/{}.dbc", $config_dir, $dbc_name))?;
        let bar = ProgressBar::new(dbc.length() as u64);
        let store = dbc.as_gt_store(&bar);
        bar.finish();
        store
    }};
}

impl DataStore {
    pub fn load_data(
        config: Arc<WorldConfig>,
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Result<DataStore, std::io::Error> {
        // DBC stores
        let chr_races = parse_dbc!(config.common.data.directory, "ChrRaces");
        let chr_classes = parse_dbc!(config.common.data.directory, "ChrClasses");
        let char_start_outfit = parse_dbc!(config.common.data.directory, "CharStartOutfit");
        let item = parse_dbc!(config.common.data.directory, "Item");
        let map = parse_dbc!(config.common.data.directory, "Map");
        let emotes_text = parse_dbc!(config.common.data.directory, "EmotesText");
        let spell = parse_dbc!(config.common.data.directory, "Spell");
        let spells_by_category = Self::build_spells_by_category_index(&spell);
        let spell_duration = parse_dbc!(config.common.data.directory, "SpellDuration");
        let spell_cast_times = parse_dbc!(config.common.data.directory, "SpellCastTimes");
        let skill_line = parse_dbc!(config.common.data.directory, "SkillLine");
        let skill_line_ability: HashMap<u32, SkillLineAbilityRecord> =
            parse_dbc!(config.common.data.directory, "SkillLineAbility");
        let skill_line_ability_by_spell = {
            let mut multimap: MultiMap<u32, SkillLineAbilityRecord> = MultiMap::new();
            for record in skill_line_ability.values() {
                multimap.insert(record.spell_id, (*record).clone());
            }

            multimap
        };
        let faction = parse_dbc!(config.common.data.directory, "Faction");
        let faction_template = parse_dbc!(config.common.data.directory, "FactionTemplate");
        let area_table = parse_dbc!(config.common.data.directory, "AreaTable");

        // GameTable stores
        let gt_OCTRegenHP = parse_game_table!(config.common.data.directory, "gtOCTRegenHP");
        let gt_RegenHPPerSpt = parse_game_table!(config.common.data.directory, "gtRegenHPPerSpt");
        let gt_RegenMPPerSpt = parse_game_table!(config.common.data.directory, "gtRegenMPPerSpt");

        // SQL stores
        let item_templates = {
            info!("Loading item templates...");
            let item_templates = ItemRepository::load_templates(conn);
            let item_templates: SqlStore<ItemTemplate> = item_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
            item_templates
        };

        let creature_templates = if config.world.dev.load_creature_templates {
            info!("Loading creature templates...");
            let creature_templates = CreatureRepository::load_templates(conn);
            let creature_templates: SqlStore<CreatureTemplate> = creature_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
            creature_templates
        } else {
            info!("Creature templates loading disabled in configuration");
            HashMap::new()
        };

        let quest_templates = {
            info!("Loading quest templates...");
            let quest_templates = QuestRepository::load_templates(conn);
            let quest_templates: SqlStore<QuestTemplate> = quest_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
            quest_templates
        };

        let quest_relations_by_creature = {
            info!("Loading creatures quest relations...");
            let quest_relations = QuestRepository::load_relations(conn, QuestActorType::Creature);
            let mut multimap: MultiMap<u32, QuestRelation> = MultiMap::new();
            for relation in quest_relations {
                let key = relation.actor_entry;
                multimap.insert(key, relation);
            }
            multimap
        };

        let quest_relations_by_game_object = {
            info!("Loading game object quest relations...");
            let quest_relations = QuestRepository::load_relations(conn, QuestActorType::GameObject);
            let mut multimap: MultiMap<u32, QuestRelation> = MultiMap::new();
            for relation in quest_relations {
                let key = relation.actor_entry;
                multimap.insert(key, relation);
            }
            multimap
        };

        info!("Loading player creation positions...");
        let player_create_positions = PlayerStaticDataRepository::load_positions(conn);
        let player_create_positions: SqlStore<PlayerCreatePosition> = player_create_positions
            .into_iter()
            .map(|pcp| {
                let key: u32 = (pcp.race << 8) | pcp.class;

                (key, pcp)
            })
            .collect();

        info!("Loading player creation spells...");
        let player_create_spells = PlayerStaticDataRepository::load_spells(conn);
        let player_create_spells: SqlMultiStore<u32> = {
            let mut multimap: MultiMap<u32, u32> = MultiMap::new();

            for pcs in player_create_spells {
                let key: u32 = (pcs.race << 8) | pcs.class;
                multimap.insert(key, pcs.spell_id);
            }

            multimap
        };

        info!("Loading player creation action buttons...");
        let player_create_action_buttons = PlayerStaticDataRepository::load_action_buttons(conn);
        let player_create_action_buttons: SqlMultiStore<PlayerCreateActionButton> = {
            let mut multimap: MultiMap<u32, PlayerCreateActionButton> = MultiMap::new();
            for action_button in player_create_action_buttons {
                let key = (action_button.race << 8) | action_button.class;
                multimap.insert(key, action_button);
            }

            multimap
        };

        let npc_texts = {
            info!("Loading npc texts...");
            let npc_texts = GossipRepository::load_npc_text(conn);
            let npc_texts: SqlStore<NpcTextDbRecord> =
                npc_texts.into_iter().map(|text| (text.id, text)).collect();
            npc_texts
        };

        let gossip_menus = {
            info!("Loading gossip menus...");
            let gossip_menus = GossipRepository::load_gossip_menus(conn);
            let mut multimap: MultiMap<u32, GossipMenuDbRecord> = MultiMap::new();

            for menu in gossip_menus {
                multimap.insert(menu.id, menu);
            }

            multimap
        };

        let creature_model_info = {
            info!("Loading creature models info...");
            let creature_model_info = CreatureRepository::load_creature_model_info(conn);
            let creature_model_info: SqlStore<CreatureModelInfo> = creature_model_info
                .into_iter()
                .map(|cmi| (cmi.model_id, cmi))
                .collect();
            creature_model_info
        };

        let player_base_health_mana = {
            info!("Loading player base health and mana per level...");
            let player_base_health_mana =
                PlayerStaticDataRepository::load_base_health_mana_per_level(conn);
            let player_base_health_mana: SqlStore<PlayerBaseHealthManaPerLevelDbRecord> =
                player_base_health_mana
                    .into_iter()
                    .map(|pbhm| (pbhm.key(), pbhm))
                    .collect();
            player_base_health_mana
        };

        let player_base_attributes = {
            info!("Loading player base attributes per level...");
            let player_base_attributes =
                PlayerStaticDataRepository::load_base_attributes_per_level(conn);
            let player_base_attributes: SqlStore<PlayerBaseAttributesPerLevelDbRecord> =
                player_base_attributes
                    .into_iter()
                    .map(|pba| (pba.key(), pba))
                    .collect();
            player_base_attributes
        };

        let creature_base_attributes = {
            info!("Loading creature base attributes per level...");
            let creature_base_attributes =
                CreatureStaticDataRepository::load_base_attributes_per_level(conn);
            let creature_base_attributes: SqlStore<CreatureBaseAttributesPerLevelDbRecord> =
                creature_base_attributes
                    .into_iter()
                    .map(|cba| (cba.key(), cba))
                    .collect();
            creature_base_attributes
        };

        let player_experience_per_level = {
            info!("Loading player required experience per level...");
            let player_experience_per_level =
                PlayerStaticDataRepository::load_experience_per_level(conn);
            let player_experience_per_level: SqlStore<PlayerExperiencePerLevel> =
                player_experience_per_level
                    .into_iter()
                    .map(|pe| (pe.level, pe))
                    .collect();
            player_experience_per_level
        };

        let loot_tables = {
            info!("Loading loot tables...");
            let loot_tables = LootRepository::load_loot_tables(conn);
            let loot_tables: SqlStore<LootTable> = loot_tables
                .unwrap()
                .into_iter()
                .map(|clt| (clt.id, clt))
                .collect();
            loot_tables
        };

        let game_object_templates = if config.world.dev.load_creature_templates {
            info!("Loading game object templates...");
            let game_object_templates = GameObjectRepository::load_templates(
                conn,
                &quest_templates,
                &quest_relations_by_game_object,
            );
            let game_object_templates: SqlStore<GameObjectTemplate> = game_object_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
            game_object_templates
        } else {
            info!("Game object templates loading disabled in configuration");
            HashMap::new()
        };

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
            item,
            map,
            emotes_text,
            spell,
            spells_by_category,
            spell_duration,
            spell_cast_times,
            skill_line,
            skill_line_ability,
            skill_line_ability_by_spell,
            faction,
            faction_template,
            area_table,
            item_templates,
            player_create_positions,
            player_create_spells,
            player_create_action_buttons,
            creature_templates,
            quest_templates,
            quest_relations_by_creature,
            quest_relations_by_game_object,
            npc_texts,
            gossip_menus,
            creature_model_info,
            player_base_health_mana,
            player_base_attributes,
            creature_base_attributes,
            player_experience_per_level,
            loot_tables,
            game_object_templates,
            gt_OCTRegenHP,
            gt_RegenHPPerSpt,
            gt_RegenMPPerSpt,
        })
    }

    pub fn get_race_record(&self, id: u32) -> Option<&ChrRacesRecord> {
        self.chr_races.get(&id)
    }

    pub fn get_class_record(&self, id: u32) -> Option<&ChrClassesRecord> {
        self.chr_classes.get(&id)
    }

    pub fn get_char_start_outfit(
        &self,
        race: u8,
        class: u8,
        gender: u8,
    ) -> Option<&CharStartOutfitRecord> {
        let key: u32 = race as u32 | ((class as u32) << 8) | ((gender as u32) << 16);
        self.char_start_outfit.get(&key)
    }

    pub fn get_item_record(&self, entry: u32) -> Option<&ItemRecord> {
        self.item.get(&entry)
    }

    pub fn get_map_record(&self, id: u32) -> Option<&MapRecord> {
        self.map.get(&id)
    }

    pub fn get_all_map_records(&self) -> Values<u32, MapRecord> {
        self.map.values()
    }

    pub fn get_text_emote_record(&self, id: u32) -> Option<&EmotesTextRecord> {
        self.emotes_text.get(&id)
    }

    pub fn get_spell_record(&self, id: u32) -> Option<&SpellRecord> {
        self.spell.get(&id)
    }

    pub fn get_spells_by_category(&self, category: u32) -> Option<&Vec<u32>> {
        self.spells_by_category.get(&category)
    }

    pub fn get_spell_duration_record(&self, id: u32) -> Option<&SpellDurationRecord> {
        self.spell_duration.get(&id)
    }

    pub fn get_spell_cast_times(&self, id: u32) -> Option<&SpellCastTimeRecord> {
        self.spell_cast_times.get(&id)
    }

    pub fn get_skill_line_record(&self, id: u32) -> Option<&SkillLineRecord> {
        self.skill_line.get(&id)
    }

    pub fn get_skill_line_ability_record(&self, id: u32) -> Option<&SkillLineAbilityRecord> {
        self.skill_line_ability.get(&id)
    }

    pub fn get_skill_line_ability_by_spell(
        &self,
        spell_id: u32,
    ) -> Option<&Vec<SkillLineAbilityRecord>> {
        self.skill_line_ability_by_spell.get_vec(&spell_id)
    }

    pub fn get_faction_record(&self, id: u32) -> Option<&FactionRecord> {
        self.faction.get(&id)
    }

    pub fn get_faction_template_record(&self, faction_id: u32) -> Option<&FactionTemplateRecord> {
        self.faction_template.get(&faction_id)
    }

    pub fn get_area_table_by_area_bit(&self, area_bit: u32) -> Option<&AreaTableRecord> {
        self.area_table.get(&area_bit)
    }

    pub fn get_area_table_by_area_id(&self, area_id: u32) -> Option<&AreaTableRecord> {
        self.area_table
            .iter()
            .find(|record| record.1.area_id == area_id)
            .map(|record| record.1)
    }

    pub fn get_starting_factions(
        &self,
        race: CharacterRaceBit,
        class: CharacterClassBit,
    ) -> Vec<(u32, u32)> {
        let mut result: Vec<(u32, u32)> = Vec::new();
        for (id, faction) in self.faction.iter() {
            match (
                faction.base_reputation_standing(race, class),
                faction.reputation_flags(race, class),
            ) {
                // TODO: enum for the flags
                (Some(_standing), Some(flags)) if flags & 0x01 != 0 => result.push((*id, flags)),
                _ => (),
            }
        }

        result
    }

    pub fn get_item_template(&self, entry: u32) -> Option<&ItemTemplate> {
        self.item_templates.get(&entry)
    }

    pub fn get_player_create_position(
        &self,
        race: u32,
        class: u32,
    ) -> Option<&PlayerCreatePosition> {
        let key: u32 = (race << 8) | class;

        self.player_create_positions.get(&key)
    }

    pub fn get_player_create_spells(&self, race: u32, class: u32) -> Option<&Vec<u32>> {
        let key: u32 = (race << 8) | class;

        self.player_create_spells.get_vec(&key)
    }

    pub fn get_player_create_action_buttons(
        &self,
        race: u32,
        class: u32,
    ) -> Option<&Vec<PlayerCreateActionButton>> {
        let key: u32 = (race << 8) | class;

        self.player_create_action_buttons.get_vec(&key)
    }

    pub fn get_creature_template(&self, entry: u32) -> Option<&CreatureTemplate> {
        self.creature_templates.get(&entry)
    }

    pub fn get_game_object_template(&self, entry: u32) -> Option<&GameObjectTemplate> {
        self.game_object_templates.get(&entry)
    }

    pub fn get_quest_template(&self, entry: u32) -> Option<&QuestTemplate> {
        self.quest_templates.get(&entry)
    }

    pub fn get_quest_relations_for_creature(&self, entry: u32) -> Option<&Vec<QuestRelation>> {
        self.quest_relations_by_creature.get_vec(&entry)
    }

    pub fn get_quest_relations_for_game_object(&self, entry: u32) -> Option<&Vec<QuestRelation>> {
        self.quest_relations_by_game_object.get_vec(&entry)
    }

    pub fn get_npc_text(&self, id: u32) -> Option<&NpcTextDbRecord> {
        self.npc_texts.get(&id)
    }

    pub fn get_gossip_menu(&self, id: u32) -> Option<&GossipMenuDbRecord> {
        self.gossip_menus.get(&id) // TODO: Select the best menu depending on conditions
    }

    pub fn get_creature_model_info(&self, model_id: u32) -> Option<&CreatureModelInfo> {
        self.creature_model_info.get(&model_id)
    }

    pub fn get_player_base_health_mana(
        &self,
        class: CharacterClass,
        level: u32,
    ) -> Option<&PlayerBaseHealthManaPerLevelDbRecord> {
        self.player_base_health_mana
            .get(&(((class as u32) << 8) | level))
    }

    pub fn get_player_base_attributes(
        &self,
        race: CharacterRace,
        class: CharacterClass,
        level: u32,
    ) -> Option<&PlayerBaseAttributesPerLevelDbRecord> {
        self.player_base_attributes
            .get(&(((race as u32) << 16) | ((class as u32) << 8) | level))
    }

    pub fn get_creature_base_attributes(
        &self,
        class: CharacterClass,
        level: u32,
    ) -> Option<&CreatureBaseAttributesPerLevelDbRecord> {
        self.creature_base_attributes
            .get(&(((class as u32) << 16) | level))
    }

    pub fn get_player_max_base_power(
        &self,
        power_type: PowerType,
        class: CharacterClass,
        level: u32,
        is_hunter_pet: bool,
    ) -> u32 {
        let base_health_mana = self
            .get_player_base_health_mana(class, level)
            .expect("unable to find base health/mana for this race/level combination");

        match power_type {
            PowerType::Health => base_health_mana.base_health,
            PowerType::Mana => base_health_mana.base_mana,
            PowerType::Rage => MAX_BASE_POWER_RAGE,
            PowerType::Focus => {
                if is_hunter_pet {
                    MAX_BASE_POWER_FOCUS
                } else {
                    0
                }
            }
            PowerType::Energy => MAX_BASE_POWER_ENERGY,
            PowerType::PetHappiness => {
                if is_hunter_pet {
                    MAX_BASE_POWER_PET_HAPPINESS
                } else {
                    0
                }
            }
        }
    }

    pub fn get_player_required_experience_at_level(&self, level: u32) -> u32 {
        self.player_experience_per_level
            .get(&level)
            .map(|pe| pe.required_experience)
            .unwrap_or(0)
    }

    pub fn get_loot_table(&self, id: u32) -> Option<&LootTable> {
        self.loot_tables.get(&id)
    }

    pub fn get_gtOCTRegenHP(&self, index: usize) -> Option<&GameTableOCTRegenHPRecord> {
        self.gt_OCTRegenHP.get(index)
    }

    pub fn get_gtRegenHPPerSpt(&self, index: usize) -> Option<&GameTableRegenHPPerSptRecord> {
        self.gt_RegenHPPerSpt.get(index)
    }

    pub fn get_gtRegenMPPerSpt(&self, index: usize) -> Option<&GameTableRegenMPPerSptRecord> {
        self.gt_RegenMPPerSpt.get(index)
    }

    fn build_spells_by_category_index(
        spells: &HashMap<u32, SpellRecord>,
    ) -> HashMap<u32, Vec<u32>> {
        let mut spells_by_category: HashMap<u32, Vec<u32>> = HashMap::new();

        for (id, spell) in spells.iter() {
            if spell.category != 0 {
                spells_by_category
                    .entry(spell.category)
                    .or_insert_with(Vec::new)
                    .push(*id);
            }
        }

        spells_by_category
    }
}
