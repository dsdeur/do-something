use crate::{
    command::Command,
    config::{GlobalConfig, OnConflict},
    dir::git_root,
    ds_file::{DsFile, Match},
    group::Group,
    help::{HelpGroup, HelpRow},
    runner::Runner,
    tui::run_tui,
};
use anyhow::Result;
use crossterm::style::Stylize;
use std::io::IsTerminal;
use std::{collections::BTreeMap, path::PathBuf};

/// Collection of loaded ds_files, to avoid reloading them multiple times
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

/// Main DoSomething structure, holding loaded files and configuration
pub struct DoSomething {
    pub ds_files: DsFiles,
    pub config: GlobalConfig,
    pub paths: Vec<PathBuf>,
    pub current_dir: PathBuf,
    pub git_root: Option<PathBuf>,
}

impl DoSomething {
    /// Create a new DoSomething instance, loading configuration and file paths
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
                OnConflict::Override if !matches.is_empty() => break,
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

    /// Get the command and its parents from a match
    pub fn command_from_match(&mut self, match_: &Match) -> Result<(&Command, Vec<&Group>)> {
        let file = self.ds_files.load_file(&match_.file_path)?;
        file.command_from_keys(&match_.keys)
    }

    pub fn command_from_help_row(&mut self, row: &HelpRow) -> Result<(&Command, Vec<&Group>)> {
        let file = self.ds_files.load_file(&row.file_path)?;
        file.command_from_keys(&row.key)
    }

    /// Get help rows for a specific match
    pub fn help_rows_for_match(&mut self, match_: &Match) -> Result<Vec<HelpRow>> {
        let file = self.ds_files.load_file(&match_.file_path)?;
        file.help_rows_for_match(match_, &self.current_dir, self.git_root.as_ref())
    }

    /// Get the DsFile from a match
    pub fn file_from_match(&mut self, match_: &Match) -> Result<&DsFile> {
        self.ds_files.load_file(&match_.file_path)
    }

    /// Get help groups for all loaded files
    pub fn help_groups(&mut self) -> Result<(Vec<HelpGroup>, usize)> {
        let mut groups = Vec::new();
        let mut max_size = 0;

        for path in &self.paths {
            let file = self.ds_files.load_file(path)?;
            let rows = file.help_rows(&self.current_dir, self.git_root.as_ref())?;

            // If the group has no commands, we skip it
            if rows.is_empty() {
                continue;
            }

            for row in &rows {
                let len = row.len();
                if len > max_size {
                    max_size = len;
                }
            }

            groups.push(file.help_group(rows));
        }

        Ok((groups, max_size))
    }

    /// Run the command, if it is a command runner
    /// - If it is a help runner, it does nothing
    pub fn run(&self, runner: Runner) -> Result<()> {
        if let Runner::Command(cmd_str, mut command) = runner {
            println!("{}", cmd_str.dim());
            let status = command.spawn()?.wait()?;
            std::process::exit(status.code().unwrap_or(1));
        }

        Ok(())
    }

    pub fn run_help_row(&mut self, row: Option<HelpRow>) -> Result<()> {
        if let Some(row) = row {
            let (command, parents) = self.command_from_help_row(&row)?;
            let mut args = vec![];

            // Add the environment if any, so the matching logic can pick it up
            if let Some(env) = &row.env {
                args.push(env.as_str());
            }

            let runner = command.runner(&parents, args.as_slice())?;
            self.run(runner)?;
        }

        Ok(())
    }

    pub fn render_tui(&mut self) -> Result<()> {
        let (groups, max_size) = self.help_groups()?;

        // If not a terminal, we just print the help
        if !std::io::stdout().is_terminal() {
            for group in groups {
                group.print(max_size);
            }
            return Ok(());
        }

        // Otherwise, we run the TUI
        let row = run_tui(groups, max_size).unwrap();
        self.run_help_row(row)
    }

    pub fn run_match(&mut self, args_str: &[&str]) -> Result<()> {
        // Get the runner based on the provided arguments
        let match_ = self.match_command(args_str)?;
        let (command, parents) = self.command_from_match(&match_)?;
        let runner = command.runner(&parents, &args_str[match_.score..])?;

        // Execute the runner
        match runner {
            Runner::Command(_, _) => self.run(runner),
            Runner::Help => {
                let lines = self.help_rows_for_match(&match_)?;
                let file = self.file_from_match(&match_)?;
                let max_size = lines.iter().map(HelpRow::len).max().unwrap_or(0);
                let row = run_tui(vec![(file.help_group(lines))], max_size);
                self.run_help_row(row.unwrap())?;

                std::process::exit(0);
            }
        }
    }
}
