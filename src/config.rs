use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

/// Configure how to handle commands with the same key
#[derive(Debug, Serialize, Deserialize)]
pub enum OnConflict {
    /// Keep the last defined command
    Override,
    /// Error on conflict
    Error,
}

/// How to find ds.json config files (Not yet implemented)
#[derive(Debug, Serialize, Deserialize)]
pub enum Resolution {
    /// Only look at the current folder where the `ds` command is run from
    CurrentFolder,
    /// Look recursively in parent directories
    Recursive,
    /// Look in the nearest Git root directory
    GitRoot,
}

/// Global configuration for the application
/// Loaded from ~/.config/dosomething/config.json
#[derive(Debug, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// Behavior on command key conflicts
    pub on_conflict: OnConflict,
    /// Resolution strategy for finding configuration files
    pub resolution: Resolution,
}

/// Get the configuration directory path, typically ~/.config/dosomething
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
    /// Load the global configuration from the config file, or return default if not found.
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
