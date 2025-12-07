use crate::{
    commands::{Command, CommandConfig, Group, Walk},
    dir::collapse_to_tilde,
    help::HelpRow,
};
use anyhow::Result;
use std::{fs, path::Path};

/// Represents a command file, which contains a group of commands
/// Mainly to have a common interface for loading and matching commands.
pub struct DsFile {
    pub group: Group,
}

/// Represents a match for a command, containing the score and keys
#[derive(Debug)]
pub struct Match {
    /// The score of the match, which is the number of levels that match
    pub score: usize,
    /// The path in the command file that matched
    pub keys: Vec<String>,
    /// All the alias keys of the command (including parents)
    pub alias_keys: Vec<Vec<String>>,
}

impl Match {
    /// Calculate the match score for a command based on the provided matches
    /// - The score is the number of levels that match
    pub fn from_command(keys: &[&str], alias_keys: &[Vec<&str>], target: &[&str]) -> Option<Self> {
        let mut score = 0;

        for (i, key) in target.iter().enumerate() {
            match alias_keys.get(i) {
                Some(keys) if keys.contains(key) => {
                    score += 1;
                }
                _ => {
                    // If the key is not found, we stop scoring
                    break;
                }
            }
        }

        // If we are not including nested commands, the key can only be smaller (rest args) or equal to the matches
        let is_nested = alias_keys.len() > target.len();

        // If the score is 0, or if we are in nested mode and the alias keys are longer than the matches, we return None
        if is_nested || score == 0 {
            return None;
        }

        // If the score is greater than 0, we return a match
        // Only do all the copying if we have an actual match
        Some(Match {
            score,
            keys: keys.iter().map(|s| s.to_string()).collect(),
            alias_keys: alias_keys
                .iter()
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .collect(),
        })
    }
}

impl DsFile {
    /// Load a group configuration from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        if !path.as_ref().exists() {
            return Err(anyhow::anyhow!(
                "File not found: {}",
                path.as_ref().display()
            ));
        }

        let content = fs::read_to_string(&path)?;
        let mut group: Group = serde_json::from_str(&content)?;

        if group.name.is_none() {
            group.name = path
                .as_ref()
                .file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());
        }

        if group.description.is_none() {
            group.description = Some(collapse_to_tilde(path.as_ref()));
        }

        Ok(Self { group })
    }

    /// Get a command (and its parents) from the tree, based on the provided keys
    pub fn command_from_keys(&self, keys: &[String]) -> Result<(&Command, Vec<&Group>)> {
        let mut parents: Vec<&Group> = Vec::new();
        let mut command = None;

        for key in keys {
            let group = parents.last().copied().unwrap_or(&self.group);
            if let Some(cmd) = group.commands.get(key) {
                command = Some(cmd);

                if let Command::Group(group) = cmd {
                    parents.push(group);
                }
            } else {
                return Err(anyhow::anyhow!(
                    "No command found for keys: {}",
                    keys.join(" ")
                ));
            }
        }

        if let Some(command) = command {
            Ok((command, parents))
        } else {
            Err(anyhow::anyhow!(
                "No command found for keys: {}",
                keys.join(" ")
            ))
        }
    }

    /// Get the commands that match the provided matches
    /// - `matches` is a vector of strings representing the command path
    /// - `include_nested` determines if nested commands should be included in the match
    /// - Returns a vector of tuples containing the match score, command keys, command definition,
    ///   and parent groups for each matching command
    pub fn get_matches(
        &self,
        target: &[&str],
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<Match>> {
        let mut commands = Vec::new();
        let mut err = None;

        self.group.walk_commands(&mut |keys, cmd, parents| {
            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match cmd.is_in_scope(current_dir.as_ref(), git_root.as_ref()) {
                Err(_) => {
                    // Store the error and stop processing
                    err = Some(anyhow::anyhow!(
                        "Error determining scope for command: {}",
                        keys.join(" ")
                    ));
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            // Calculate the match score
            let command_keys = cmd.get_keys(keys, parents);
            let m = Match::from_command(keys, &command_keys, target);

            if let Some(m) = m {
                commands.push(m);
            }

            Walk::Continue
        });

        // If there was an error, return it
        if let Some(err) = err {
            return Err(err);
        }

        // Determine the maximum depth of the matching commands
        let max_depth = commands
            .iter()
            .map(|Match { score, .. }| *score)
            .max()
            .unwrap_or(0);

        // Filter the most deeply matching commands
        let res = commands
            .into_iter()
            .filter(|Match { score, .. }| *score == max_depth)
            .collect();

        Ok(res)
    }

    /// Get the help rows for a match in the command file
    pub fn get_help_rows_for_match(
        &self,
        match_: &Match,
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<HelpRow>> {
        let (command, parents) = self.command_from_keys(&match_.keys)?;
        let mut keys = match_.keys.iter().map(|s| s.as_str()).collect();
        let mut parents = parents;

        match command {
            Command::Basic(cmd) => Ok(vec![HelpRow::new(match_.alias_keys.clone(), cmd.clone())]),
            Command::CommandConfig(CommandConfig { command, .. }) => Ok(vec![HelpRow::new(
                match_.alias_keys.clone(),
                command.clone(),
            )]),
            Command::Group(group) => {
                group.get_help_rows(&mut keys, &mut parents, current_dir, git_root)
            }
        }
    }

    /// Get the help rows for the full command file
    pub fn get_help_rows(
        &self,
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<HelpRow>> {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.group
            .get_help_rows(&mut keys, &mut parents, current_dir, git_root.as_ref())
    }
}
