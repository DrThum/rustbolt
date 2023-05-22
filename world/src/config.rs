use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WorldConfig {
    pub world: WorldSection,
    pub common: CommonSection, // TODO: Move to a common lib
}

impl WorldConfig {
    // https://github.com/mehcode/config-rs/blob/master/examples/hierarchical-env/settings.rs
    pub fn load() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("config.template.toml"))
            .add_source(File::with_name("config.toml"))
            .build()?;

        s.try_deserialize()
    }
}

#[derive(Debug, Deserialize)]
pub struct CommonSection {
    pub data: DataSection,
}

#[derive(Debug, Deserialize)]
pub struct WorldSection {
    pub network: NetworkSection,
    pub game: GameSection,
    pub dev: DevSection,
}

#[derive(Debug, Deserialize)]
pub struct DataSection {
    pub directory: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkSection {
    pub host: String,
    pub port: String,
}

#[derive(Debug, Deserialize)]
pub struct GameSection {
    pub player: PlayerSection,
}

#[derive(Debug, Deserialize)]
pub struct PlayerSection {
    pub maxlevel: u32,
}

#[derive(Debug, Deserialize)]
pub struct DevSection {
    pub load_terrain: bool,
    pub load_item_templates: bool,
}
