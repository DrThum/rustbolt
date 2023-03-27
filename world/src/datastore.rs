use std::collections::HashMap;

use crate::{config::DataSection, datastore::dbc::Dbc};

use self::data_types::ChrRacesRecord;

pub mod data_types;
pub mod dbc;

pub type DbcStore<T> = HashMap<u32, T>;

pub struct DataStore {
    pub chr_races: HashMap<u32, ChrRacesRecord>,
}

impl DataStore {
    pub fn load_dbcs(config: &DataSection) -> Result<DataStore, std::io::Error> {
        let dbc = Dbc::parse(format!("{}/dbcs/ChrRaces.dbc", config.directory))?;
        let chr_races = dbc.as_store();

        Ok(DataStore { chr_races })
    }
}
