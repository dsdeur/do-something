use crate::{
    command::Command,
    config::{GlobalConfig, OnConflict},
    dir::git_root,
    ds_file::{DsFile, Match},
    group::Group,
    help::HelpRow,
};
use anyhow::Result;

use std::{collections::BTreeMap, path::PathBuf};

#[derive(Default)]
pub struct DsFiles {
    pub files: BTreeMap<PathBuf, DsFile>,
}

impl DsFiles {
    /// Load a ds_file by path
    /// If already loaded, returns the existing one
    fn load_file(&mut self, path: &PathBuf) -> Result<&DsFile> {
        if !self.files.contains_key(path) {
            let ds_file = DsFile::from_file(path)?;
            self.files.insert(path.clone(), ds_file);
        }

        Ok(self.files.get(path).unwrap())
    }
}

pub struct DoSomething {
    pub ds_files: DsFiles,
    pub config: GlobalConfig,
    pub paths: Vec<PathBuf>,
    pub current_dir: PathBuf,
    pub git_root: Option<PathBuf>,
}

impl DoSomething {
    pub fn new() -> Result<Self> {
        let config = GlobalConfig::load()?;
        let paths = config.file_paths()?;

        Ok(DoSomething {
            ds_files: DsFiles::default(),
            config,
            paths,
            current_dir: std::env::current_dir()?,
            git_root: git_root(),
        })
    }

    /// Find and match a command in the provided paths
    pub fn match_command(&mut self, target: &[&str]) -> Result<Match> {
        let mut matches = Vec::new();

        for path in &self.paths {
            let file = self.ds_files.load_file(path)?;
            let file_matches = file.matches(target, &self.current_dir, self.git_root.as_ref())?;

            // Add matches, last one wins
            matches.extend(file_matches.into_iter().rev());

            match &self.config.on_conflict {
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

    pub fn command_from_match(&mut self, match_: &Match) -> Result<(&Command, Vec<&Group>)> {
        let file = self.ds_files.load_file(&match_.file_path)?;
        file.command_from_keys(&match_.keys)
    }

    pub fn help_rows_for_match(&mut self, match_: &Match) -> Result<Vec<HelpRow>> {
        let file = self.ds_files.load_file(&match_.file_path)?;
        file.help_rows_for_match(match_, &self.current_dir, self.git_root.as_ref())
    }

    pub fn file_from_match(&mut self, match_: &Match) -> Result<&DsFile> {
        self.ds_files.load_file(&match_.file_path)
    }
}
