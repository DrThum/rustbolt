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

impl DataStore {
    pub fn load_dbcs(config: &DataSection) -> Result<DataStore, std::io::Error> {
        let chr_races = Dbc::parse(format!("{}/dbcs/ChrRaces.dbc", config.directory))?.as_store();
        let chr_classes =
            Dbc::parse(format!("{}/dbcs/ChrClasses.dbc", config.directory))?.as_store();
        let char_start_outfit =
            Dbc::parse(format!("{}/dbcs/CharStartOutfit.dbc", config.directory))?.as_store();

        Ok(DataStore {
            chr_races,
            chr_classes,
            char_start_outfit,
        })
    }
}
