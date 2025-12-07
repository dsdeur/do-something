use crate::{
    commands::{Command, Group},
    dir::collapse_to_tilde,
};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct DsFile {
    pub group: Group,
}

/// Util for tree walking to control the flow of the walk.
#[derive(PartialEq, Eq)]
pub enum Walk {
    Continue,
    Skip,
    Stop,
}

#[derive(Debug)]
pub struct Match {
    pub score: usize,
    pub keys: Vec<String>,
    pub alias_keys: Vec<Vec<String>>,
    pub command: Command,
}

#[derive(PartialEq, Eq, Debug)]
pub enum NestingMode {
    Include,
    Exclude,
}

impl Match {
    /// Calculate the match score for a command based on the provided matches
    /// - The score is the number of levels that match
    /// - If `include_nested` is false, the command keys will not be allowed to be longer than the matches
    pub fn from_command(
        command: &Command,
        keys: &[&str],
        alias_keys: &Vec<Vec<&str>>,
        target: Vec<&str>,
        nesting_mode: &NestingMode,
    ) -> Option<Self> {
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
        let is_nested = nesting_mode == &NestingMode::Include || alias_keys.len() > target.len();

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
            command: command.clone(),
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

    fn walk_tree_rev<'a>(
        group: &'a Group,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        on_command: &mut dyn FnMut(&[&str], &Command, &[&Group]) -> Walk,
    ) -> Walk {
        parents.push(group);

        for (key, command) in group.commands.iter().rev() {
            keys.push(key);

            match on_command(&keys, command, &parents) {
                Walk::Continue => (),
                // Skip the current command, meaning don't process the group
                Walk::Skip => {
                    keys.pop();
                    continue;
                }
                // Stop the walk, meaning don't process any more commands
                Walk::Stop => {
                    keys.pop();
                    parents.pop();
                    return Walk::Stop;
                }
            };

            if let Command::Group(group) = command {
                // If the command is a group, walk through its tree
                // If the walk returns Stop, we stop processing
                if DsFile::walk_tree_rev(group, keys, parents, on_command) == Walk::Stop {
                    keys.pop();
                    parents.pop();
                    return Walk::Stop;
                }
            }

            keys.pop();
        }

        parents.pop();
        Walk::Continue
    }

    /// Walk through all commands in the group and its subgroups in reverse order.
    /// - Calls `on_command` for each command with the current path, command definition, and parent groups
    /// - The path is a vector of strings representing the command keys
    /// - The command definition is the current command being processed
    /// - The parent groups are the groups that lead to the current command
    pub fn walk_commands<'a>(
        &'a self,
        on_command: &mut dyn FnMut(&[&str], &Command, &[&Group]) -> Walk,
    ) {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        DsFile::walk_tree_rev(&self.group, &mut keys, &mut parents, on_command);
    }

    /// Get all parents for the given keys
    pub fn groups_from_keys(&self, keys: &Vec<String>) -> Vec<&Group> {
        let mut parents = Vec::new();
        parents.push(&self.group);

        for key in keys {
            parents
                .last()
                .and_then(|g| g.get_command_by_key(key))
                .and_then(|cmd| {
                    if let Command::Group(group) = cmd {
                        parents.push(group);
                        Some(())
                    } else {
                        None
                    }
                });
        }

        parents
    }

    /// Get the commands that match the provided matches
    /// - `matches` is a vector of strings representing the command path
    /// - `include_nested` determines if nested commands should be included in the match
    /// - Returns a vector of tuples containing the match score, command keys, command definition,
    ///   and parent groups for each matching command
    pub fn get_matches(
        &self,
        matches: &Vec<&str>,
        nesting_mode: &NestingMode,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<Vec<Match>> {
        let mut commands = Vec::new();
        let mut err = None;

        self.walk_commands(&mut |keys, cmd, parents| {
            let is_in_scope = cmd.is_in_scope(current_dir.as_ref(), git_root);

            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match is_in_scope {
                Err(_) => {
                    // Store the error and stop processing
                    err = Some(anyhow::anyhow!(
                        "Command {} is not in scope",
                        keys.join(" ")
                    ));
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            // Calculate the match score
            let command_keys = cmd.get_keys(keys, parents);
            let m = Match::from_command(cmd, keys, &command_keys, matches.clone(), &nesting_mode);

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
}
