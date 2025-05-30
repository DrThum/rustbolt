use config::{Config, ConfigError, File};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub auth: AuthSection,
    pub common: CommonSection, // TODO: Move to a common lib
}

impl AuthConfig {
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
pub struct AuthSection {
    pub network: NetworkSection,
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
