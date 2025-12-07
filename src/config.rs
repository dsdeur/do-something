use crate::dir::git_root;
use anyhow::Result;
use glob::glob;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, path::PathBuf};

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
    /// Optional list of files to to collect commands from
    pub ds_files: Option<Vec<String>>,
}

/// Get the configuration directory path, typically ~/.config/dosomething
pub fn get_config_dir() -> Option<std::path::PathBuf> {
    env::home_dir().map(|f| f.join(".config").join("do-something"))
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            on_conflict: OnConflict::Error,
            resolution: Resolution::Recursive,
            ds_files: None,
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

    /// Get the paths to all ds.json files based on the configuration
    /// The order matters, as that's how commands get merged:
    /// 1. Config ds.json file, ~/.config/dosomething/ds.json
    /// 2. Files/glob patterns specified in the config under `ds_files`
    /// 3. ds.json in the Git root directory
    /// 4. ds.json in the current directory
    pub fn get_command_paths(&self) -> Result<Vec<std::path::PathBuf>> {
        let mut paths = Vec::new();

        // Get the config location
        let config_dir =
            get_config_dir().ok_or(anyhow::anyhow!("Could not find config directory"))?;

        // Add the main config file
        let config_file = config_dir.join("config.json");
        paths.push(config_dir.join("ds.json"));

        // If ds_files is specified, expand and resolve each file
        if let Some(ds_files) = &self.ds_files {
            for file in ds_files {
                let expanded = shellexpand::tilde(file);
                let path = PathBuf::from(expanded.as_ref());

                // If the path is absolute, use it directly; otherwise, resolve relative to config_dir
                let resolved = if path.is_absolute() {
                    path
                } else {
                    config_dir.join(path)
                };

                // If the resolved path is a directory, glob it
                for entry in glob(&resolved.to_string_lossy())? {
                    let path = entry?;

                    if path != config_file {
                        paths.push(path);
                    }
                }
            }
        }

        // Add the ds.json file in the git root dir if it exists
        if let Some(path) = git_root() {
            paths.push(path.join("ds.json"));
        }

        // Add the ds.json file in the current directory
        if let Ok(dir) = env::current_dir() {
            paths.push(dir.join("ds.json"));
        }

        // Remove duplicates while preserving order
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();

        for p in paths {
            if seen.insert(p.clone()) {
                if p.exists() {
                    deduped.push(p);
                }
            }
        }

        Ok(deduped)
    }
}
