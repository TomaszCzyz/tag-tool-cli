use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Debug)]
pub(crate) struct AppConfig {
    pub database: DatabaseConfig,
}

#[derive(Deserialize, Debug)]
pub struct DatabaseConfig {
    pub name: String,
    pub path: Option<PathBuf>,
}

impl AppConfig {
    pub fn load_from_file<T>(path: T) -> color_eyre::Result<Self>
    where
        T: AsRef<Path>,
    {
        let config_content = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&config_content)?;
        Ok(config)
    }
}
