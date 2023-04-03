use indicatif::ProgressBar;
use log::info;
use std::collections::HashMap;

use r2d2::PooledConnection;
use r2d2_sqlite::SqliteConnectionManager;

use crate::{config::DataSection, datastore::dbc::Dbc, repositories::item::ItemRepository};

use self::data_types::{
    CharStartOutfitRecord, ChrClassesRecord, ChrRacesRecord, ItemRecord, ItemTemplate,
};

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;
pub type SqlStore<T> = HashMap<u32, T>;

pub struct DataStore {
    chr_races: DbcStore<ChrRacesRecord>,
    chr_classes: DbcStore<ChrClassesRecord>,
    char_start_outfit: DbcStore<CharStartOutfitRecord>,
    item: DbcStore<ItemRecord>,
    item_templates: SqlStore<ItemTemplate>,
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
        config: &DataSection,
        conn: &PooledConnection<SqliteConnectionManager>,
    ) -> Result<DataStore, std::io::Error> {
        // DBC stores
        let chr_races = parse_dbc!(config.directory, "ChrRaces");
        let chr_classes = parse_dbc!(config.directory, "ChrClasses");
        let char_start_outfit = parse_dbc!(config.directory, "CharStartOutfit");
        let item = parse_dbc!(config.directory, "Item");

        // SQL stores
        info!("Loading item templates...");
        let item_templates = ItemRepository::load_templates(conn);
        let item_templates: SqlStore<ItemTemplate> = item_templates
            .into_iter()
            .map(|template| (template.entry, template))
            .collect();

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
            item,
            item_templates,
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

    pub fn get_item_template(&self, entry: u32) -> Option<&ItemTemplate> {
        self.item_templates.get(&entry)
    }
}
