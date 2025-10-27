use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Destination {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub remote_path: String,
    pub password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub ssh_key_path: String,
    pub destinations: Vec<Destination>,
}

impl Config {
    pub fn config_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".config").join("arkv"))
    }

    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Result<Option<Self>> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)
            .context("Failed to read config file")?;
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        Ok(Some(config))
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir()?;
        fs::create_dir_all(&dir)
            .context("Failed to create config directory")?;

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;
        
        let path = Self::config_path()?;
        fs::write(&path, content)
            .context("Failed to write config file")?;
        
        Ok(())
    }
}
