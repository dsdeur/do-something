use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Result;
use shell_escape::escape;

use crate::{
    commands::{CommandConfig, CommandDefinition, Group, GroupMode},
    dir::resolve_path,
};

/// Get the command keys for a given command definition
/// - This function collects the keys from the command and its parent groups,
/// - Resolves aliases if they exist
/// - Returns a vector of vectors, where each represents one level of aliases
fn get_command_keys<'a>(
    keys: &[&'a str],
    command: &'a CommandDefinition,
    parents: &[&'a Group],
) -> Vec<Vec<&'a str>> {
    let mut parent_keys = Vec::with_capacity(parents.len() + 1);

    // Collect all parent keys
    for (i, group) in parents.iter().enumerate() {
        if i == 0 {
            continue;
        }

        let key = keys[i - 1];

        match group.mode {
            // Only collect group aliases if the group is namespaced (default)
            Some(GroupMode::Namespaced) | None => {
                if let Some(aliases) = &group.aliases {
                    let mut keys = Vec::with_capacity(1 + aliases.len());

                    // Add the group key, and its aliases
                    keys.push(key);

                    for alias in aliases {
                        keys.push(alias);
                    }

                    // Add to the parent keys
                    parent_keys.push(keys);
                } else {
                    parent_keys.push(vec![key]);
                }
            }
            Some(GroupMode::Flattened) => {
                continue;
            }
        }
    }

    // Add the command key
    let last_key = keys.last().unwrap_or(&"");
    let mut command_keys = vec![*last_key];

    // Add the command aliases if they exist
    match command {
        CommandDefinition::Command(_) => (),
        CommandDefinition::CommandConfig(command) => {
            if let Some(aliases) = &command.aliases {
                for alias in aliases {
                    command_keys.push(alias);
                }
            }
        }
        CommandDefinition::Group(group) => {
            if let Some(aliases) = &group.aliases {
                for alias in aliases {
                    command_keys.push(alias);
                }
            }
        }
    }

    // Combine the parent keys with the command keys
    parent_keys.push(command_keys);
    parent_keys
}

/// Calculate the match score for a command based on the provided matches
/// - The score is the number of levels that match
/// - If `include_nested` is false, the command keys will not be allowed to be longer than the matches
fn get_match_score(command_keys: &Vec<Vec<&str>>, matches: &[&str], include_nested: bool) -> usize {
    let mut score = 0;

    for (i, key) in matches.iter().enumerate() {
        // Rest params, we are not interested in them
        if i >= command_keys.len() {
            break;
        }

        // Check if the key matches any of the command keys
        if command_keys[i].contains(key) {
            score += 1;
        } else {
            // If it doesn't match, we stop scoring
            break;
        }
    }

    // If we are not including nested commands, the key can only be smaller (rest args) or equal to the matches
    if !include_nested && command_keys.len() > matches.len() {
        return 0;
    }

    score
}

/// Get the root path and configuration for a command
/// - Returns a tuple of the root configuration and the resolved path
/// - Looks at the command first, then at the parent groups
///
/// This one is different from CommandDefinition::get_command_root, as it looks
/// at all parents, not just the immediate parent group.
pub fn get_command_root_path<'a>(
    command: &'a CommandDefinition,
    parents: &[&'a Group],
) -> Result<Option<PathBuf>> {
    let command_root = match command {
        CommandDefinition::CommandConfig(cmd) => cmd.root.as_ref(),
        CommandDefinition::Group(group) => group.root.as_ref(),
        _ => None,
    };

    if let Some(root) = command_root.or(parents.iter().rev().find_map(|g| g.root.as_ref())) {
        let path = resolve_path(&root.path)?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn create_command(command: &str, work_dir: Option<&PathBuf>, args: &[&str]) -> Result<Command> {
    let mut command_str = command.to_string();

    for arg in args {
        command_str.push(' ');
        command_str.push_str(&escape((*arg).into()));
    }

    let mut cmd = std::process::Command::new("sh");

    cmd.arg("-c");
    cmd.arg(command_str);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }

    if std::io::stdout().is_terminal() {
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("FORCE_COLOR", "1");
    }

    Ok(cmd)
}

/// Enum representing the type of command runner
/// - `Command` is a command to run
/// - `Help` is a help group that provides information about commands
#[derive(Debug)]
pub enum Runner {
    Command(Command),
    Help(Group),
}

/// Get the command runner for a given command definition
/// - Returns a `Runner` enum that can either be a command to run or a help group
/// - If a group has a default command, it will create a command runner for that
/// - It handles root paths and arguments
pub fn get_runner(
    command: &CommandDefinition,
    parents: &[&Group],
    args: &[&str],
) -> Result<Runner> {
    let path = get_command_root_path(command, parents)?;

    let runner = match command {
        CommandDefinition::Group(Group {
            default: Some(cmd), ..
        }) => Runner::Command(create_command(cmd, path.as_ref(), args)?),
        CommandDefinition::Command(cmd) => {
            Runner::Command(create_command(cmd, path.as_ref(), args)?)
        }
        CommandDefinition::CommandConfig(CommandConfig { command: cmd, .. }) => {
            Runner::Command(create_command(cmd, path.as_ref(), args)?)
        }
        CommandDefinition::Group(group) => Runner::Help(group.clone()),
    };

    Ok(runner)
}

#[derive(PartialEq, Eq)]
pub enum Walk {
    Continue,
    Skip,
    Stop,
}

impl Group {
    fn walk_tree<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        on_command: &mut dyn FnMut(&[&str], &CommandDefinition, &[&'a Group]) -> Walk,
    ) -> Walk {
        parents.push(self);

        for (key, command) in self.commands.iter() {
            keys.push(key);

            match on_command(&keys, command, parents) {
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

            if let CommandDefinition::Group(group) = command {
                // If the command is a group, walk through its tree
                // If the walk returns Stop, we stop processing
                if group.walk_tree(keys, parents, on_command) == Walk::Stop {
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

    /// Walk through all commands in the group and its subgroups
    /// - Calls `on_command` for each command with the current path, command definition, and parent groups
    /// - The path is a vector of strings representing the command keys
    /// - The command definition is the current command being processed
    /// - The parent groups are the groups that lead to the current command
    pub fn walk_commands<'a>(
        &'a self,
        on_command: &mut dyn FnMut(&[&str], &CommandDefinition, &[&'a Group]) -> Walk,
    ) {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.walk_tree(&mut keys, &mut parents, on_command);
    }

    /// Get the commands that match the provided matches
    /// - `matches` is a vector of strings representing the command path
    /// - `include_nested` determines if nested commands should be included in the match
    /// - Returns a vector of tuples containing the match score, command keys, command definition,
    ///   and parent groups for each matching command
    pub fn get_matches(
        &self,
        matches: Vec<&str>,
        include_nested: bool,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<Vec<(usize, Vec<String>, CommandDefinition, Vec<&Group>)>> {
        let mut commands = Vec::new();
        let mut err = None;

        self.walk_commands(&mut |key, cmd, parents| {
            let is_in_scope = cmd.is_in_scope(current_dir.as_ref(), git_root);

            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match is_in_scope {
                Err(_) => {
                    // Store the error and stop processing
                    err = Some(anyhow::anyhow!("Command {} is not in scope", key.join(" ")));
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            // Calculate the match score
            let command_keys = get_command_keys(key, cmd, parents);
            let score = get_match_score(&command_keys, &matches, include_nested);

            if score > 0 {
                commands.push((
                    score,
                    key.iter().map(|s| s.to_string()).collect(),
                    cmd.clone(),
                    parents.iter().copied().collect(),
                ));
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
            .map(|(score, _, _, _)| *score)
            .max()
            .unwrap_or(0);

        // Filter the most deeply matching commands
        let res = commands
            .into_iter()
            .filter(|(score, _, _, _)| *score == max_depth)
            .collect();

        Ok(res)
    }
}
