use indicatif::ProgressBar;
use log::info;
use multimap::MultiMap;
use std::{
    collections::{hash_map::Values, HashMap},
    sync::Arc,
};

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{
    config::WorldConfig,
    datastore::{
        data_types::{CreatureTemplate, PlayerCreatePosition},
        dbc::Dbc,
    },
    repositories::{
        creature::CreatureRepository, item::ItemRepository,
        player_creation::PlayerCreationRepository,
    },
};

use self::data_types::{
    CharStartOutfitRecord, ChrClassesRecord, ChrRacesRecord, EmotesTextRecord, ItemRecord,
    ItemTemplate, MapRecord, PlayerCreateActionButton, SkillLineAbilityRecord, SkillLineRecord,
    SpellRecord,
};

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;
pub type DbcMultiStore<T> = MultiMap<u32, T>;
pub type SqlStore<T> = HashMap<u32, T>;
pub type SqlMultiStore<T> = MultiMap<u32, T>;

pub struct DataStore {
    chr_races: DbcStore<ChrRacesRecord>,
    chr_classes: DbcStore<ChrClassesRecord>,
    char_start_outfit: DbcStore<CharStartOutfitRecord>,
    item: DbcStore<ItemRecord>,
    map: DbcStore<MapRecord>,
    emotes_text: DbcStore<EmotesTextRecord>,
    spell: DbcStore<SpellRecord>,
    skill_line: DbcStore<SkillLineRecord>,
    skill_line_ability: DbcStore<SkillLineAbilityRecord>,
    skill_line_ability_by_spell: DbcMultiStore<SkillLineAbilityRecord>,
    item_templates: SqlStore<ItemTemplate>,
    player_create_positions: SqlStore<PlayerCreatePosition>,
    player_create_spells: SqlMultiStore<u32>,
    player_create_action_buttons: SqlMultiStore<PlayerCreateActionButton>,
    creature_templates: SqlStore<CreatureTemplate>,
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
        let skill_line = parse_dbc!(config.common.data.directory, "SkillLine");
        let skill_line_ability: HashMap<u32, SkillLineAbilityRecord> =
            parse_dbc!(config.common.data.directory, "SkillLineAbility");
        let skill_line_ability_by_spell = {
            let mut multimap: MultiMap<u32, SkillLineAbilityRecord> = MultiMap::new();
            for (_, record) in &skill_line_ability {
                multimap.insert(record.spell_id, (*record).clone());
            }

            multimap
        };

        // SQL stores
        let item_templates = if config.world.dev.load_item_templates {
            info!("Loading item templates...");
            let item_templates = ItemRepository::load_templates(conn);
            let item_templates: SqlStore<ItemTemplate> = item_templates
                .into_iter()
                .map(|template| (template.entry, template))
                .collect();
            item_templates
        } else {
            info!("Item templates loading disabled in configuration");
            HashMap::new()
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

        info!("Loading player creation positions...");
        let player_create_positions = PlayerCreationRepository::load_positions(conn);
        let player_create_positions: SqlStore<PlayerCreatePosition> = player_create_positions
            .into_iter()
            .map(|pcp| {
                let key: u32 = (pcp.race << 8) | pcp.class;

                (key, pcp)
            })
            .collect();

        info!("Loading player creation spells...");
        let player_create_spells = PlayerCreationRepository::load_spells(conn);
        let player_create_spells: SqlMultiStore<u32> = {
            let mut multimap: MultiMap<u32, u32> = MultiMap::new();

            for pcs in player_create_spells {
                let key: u32 = (pcs.race << 8) | pcs.class;
                multimap.insert(key, pcs.spell_id);
            }

            multimap
        };

        info!("Loading player creation action buttons...");
        let player_create_action_buttons = PlayerCreationRepository::load_action_buttons(conn);
        let player_create_action_buttons: SqlMultiStore<PlayerCreateActionButton> = {
            let mut multimap: MultiMap<u32, PlayerCreateActionButton> = MultiMap::new();
            for action_button in player_create_action_buttons {
                let key = (action_button.race << 8) | action_button.class;
                multimap.insert(key, action_button);
            }

            multimap
        };

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
            item,
            map,
            emotes_text,
            spell,
            skill_line,
            skill_line_ability,
            skill_line_ability_by_spell,
            item_templates,
            player_create_positions,
            player_create_spells,
            player_create_action_buttons,
            creature_templates,
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
}
