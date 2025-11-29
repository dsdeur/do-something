use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub enum OnConflict {
    Override,
    Error,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Resolution {
    CurrentFolder,
    Recursive,
    GitRoot,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub on_conflict: OnConflict,
    pub resolution: Resolution,
}

pub fn get_config_dir() -> Option<std::path::PathBuf> {
    env::home_dir().map(|f| f.join(".config").join("dosomething"))
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            on_conflict: OnConflict::Error,
            resolution: Resolution::Recursive,
        }
    }
}

impl GlobalConfig {
    pub fn load() -> Result<Self> {
        let dir = get_config_dir();

        if let Some(dir) = dir {
            let path = dir.join("config.json");
            if !path.exists() {
                return Ok(GlobalConfig::default());
            }

            let content = std::fs::read_to_string(path)?;
            let config: GlobalConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(GlobalConfig::default())
        }
    }
}
