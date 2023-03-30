use std::collections::HashMap;

use crate::{config::DataSection, datastore::dbc::Dbc};

use self::data_types::{CharStartOutfitRecord, ChrClassesRecord, ChrRacesRecord};

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;

pub struct DataStore {
    pub chr_races: DbcStore<ChrRacesRecord>,
    pub chr_classes: DbcStore<ChrClassesRecord>,
    pub char_start_outfit: DbcStore<CharStartOutfitRecord>,
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

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
        })
    }
}
