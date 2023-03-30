use std::collections::HashMap;

use crate::{config::DataSection, datastore::dbc::Dbc};

use self::data_types::{CharStartOutfitRecord, ChrClassesRecord, ChrRacesRecord, ItemRecord};

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;

pub struct DataStore {
    chr_races: DbcStore<ChrRacesRecord>,
    chr_classes: DbcStore<ChrClassesRecord>,
    char_start_outfit: DbcStore<CharStartOutfitRecord>,
    item: DbcStore<ItemRecord>,
}

macro_rules! parse_dbc {
    ($config_dir:expr, $dbc_name:expr) => {
        Dbc::parse(format!("{}/dbcs/{}.dbc", $config_dir, $dbc_name))?.as_store()
    };
}

impl DataStore {
    pub fn load_dbcs(config: &DataSection) -> Result<DataStore, std::io::Error> {
        let chr_races = parse_dbc!(config.directory, "ChrRaces");
        let chr_classes = parse_dbc!(config.directory, "ChrClasses");
        let char_start_outfit = parse_dbc!(config.directory, "CharStartOutfit");
        let item = parse_dbc!(config.directory, "Item");

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
            item,
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

    pub fn get_item(&self, entry: u32) -> Option<&ItemRecord> {
        self.item.get(&entry)
    }
}
