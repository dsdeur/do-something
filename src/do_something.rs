use crate::{
    config::{GlobalConfig, OnConflict},
    dir::git_root,
    ds_file::{DsFile, Match},
};
use anyhow::Result;

use std::{collections::BTreeMap, path::PathBuf};

pub struct DoSomething {
    pub ds_files: BTreeMap<PathBuf, DsFile>,
    pub config: GlobalConfig,
    pub paths: Vec<PathBuf>,
    pub current_dir: PathBuf,
    pub git_root: Option<PathBuf>,
}

impl DoSomething {
    pub fn new() -> Result<Self> {
        let config = GlobalConfig::load()?;
        let paths = config.get_command_paths()?;

        Ok(DoSomething {
            ds_files: BTreeMap::new(),
            config,
            paths,
            current_dir: std::env::current_dir()?,
            git_root: git_root(),
        })
    }

    /// Load a ds_file by path
    /// If already loaded, returns the existing one
    fn load_file(&mut self, path: &PathBuf) -> Result<&DsFile> {
        if !self.ds_files.contains_key(path) {
            let ds_file = DsFile::from_file(path)?;
            self.ds_files.insert(path.clone(), ds_file);
        }

        Ok(self.ds_files.get(path).unwrap())
    }

    /// Find and match a command in the provided paths
    pub fn match_command(&mut self, target: &[&str]) -> Result<Match> {
        let mut matches = Vec::new();

        // Clone/copy these values to avoid borrowing self during mutation
        let current_dir = self.current_dir.clone();
        let git_root = self.git_root.clone();
        let on_conflict = self.config.on_conflict.clone();

        // Collect paths first to avoid borrowing self.paths during mutation
        let paths: Vec<PathBuf> = self.paths.clone();

        for path in &paths {
            let file = self.load_file(path)?;
            let file_matches = file.get_matches(target, &current_dir, git_root.as_ref())?;

            // Add matches, last one wins
            matches.extend(file_matches.into_iter().rev());

            match &on_conflict {
                // Since we are reverse iterating, we can break on the first match
                OnConflict::Override if matches.len() > 0 => break,
                // If we have multiple matches, or previous files with matches, and the config is set to error,
                // we return an error
                OnConflict::Error if matches.len() > 1 => {
                    return Err(anyhow::anyhow!("Conflict detected in group"));
                }
                // Otherwise we just continue to collect matches
                _ => {}
            }
        }

        // Return the first match if any
        match matches.into_iter().next() {
            None => Err(anyhow::anyhow!("No matching command found")),
            Some(m) => Ok(m),
        }
    }
}
