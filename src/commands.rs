use crate::{dir::resolve_path, tui::help::HelpRow};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Configures when a command or group is available to run.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RootScope {
    /// The command is always in scope
    Global,
    /// The current path must be inside the git root path
    GitRoot,
    /// The current folder must match the root path exactly
    Exact,
}

/// Defining where the command or group is run from, and configure its scope.
///
/// - Used to run commands from a different directory.
/// - Used to limit commands to specific directories.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RootConfig {
    pub path: String,
    pub scope: Option<RootScope>,
}

/// Allows to flatten groups into their parent namespace
///
/// Useful to organize commands without adding extra nesting in the CLI.
///
/// For example, if you can't introduce a `ds.json` file in a project, you can define
/// the commands in a group in your global config:
/// - Set the root path to the project git root folder, so the commands are run from there.
/// - Set root scope to `GitRoot` so the commands are only available inside that project.
/// - Set group mode to Flattened, so the commands are available without the extra step
///   (e.g. `ds command` instead of `ds group command`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupMode {
    Namespaced,
    Flattened,
}

/// Configuration for a single command.
///
/// There is a lot of overlap with the group configuration,
/// these override the group settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandConfig {
    /// Optional name for the command, used in help messages.
    pub name: Option<String>,
    /// Optional longer description for the command, used in help messages.
    pub description: Option<String>,
    /// The command to run.
    pub command: String,
    /// Optional environment keys (not yet implemented).
    pub envs: Option<Vec<String>>,
    /// Optional root configuration, to define where the command is run from.
    pub root: Option<RootConfig>,
    /// Optional aliases for the command, used to run it with different names.
    pub aliases: Option<Vec<String>>,
}

/// Util for tree walking to control the flow of the walk.
#[derive(PartialEq, Eq)]
pub enum Walk {
    Continue,
    Skip,
    Stop,
}

/// A group of commands, that share common configuration.
///
/// This is the top-level structure of a `ds.json` file, and can be nested.
/// If there are multiple files, they are merged together
/// (configured in `on_conflict` in global config).
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Group {
    /// Optional name for the group, used in help messages.
    pub name: Option<String>,
    /// Optional longer description for the group, used in help messages.
    pub description: Option<String>,
    /// Optional default command for the group, if no sub-command is provided.
    /// If not provided, it will show help for the group.
    pub default: Option<String>,
    /// Commands within the group. Can be commands or sub-groups.
    pub commands: BTreeMap<String, Command>,
    /// Optional environment keys (not yet implemented).
    pub envs: Option<Vec<String>>,
    /// Optional dotenv files options (not yet implemented).
    pub dotenv_files: Option<BTreeMap<String, String>>,
    /// Optional root configuration, to define where the group is run from.
    pub root: Option<RootConfig>,
    /// Optional group mode, to define if it is namespaced or flattened.
    pub mode: Option<GroupMode>,
    /// Optional aliases for the group, used to run it with different names.
    pub aliases: Option<Vec<String>>,
}

impl Group {
    /// Walk the group commands recursively, calling `on_command` for each command.
    pub fn walk_tree<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        on_command: &mut dyn FnMut(&[&str], &Command, &[&'a Group]) -> Walk,
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

            if let Command::Group(group) = command {
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
        on_command: &mut dyn FnMut(&[&str], &Command, &[&'a Group]) -> Walk,
    ) {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.walk_tree(&mut keys, &mut parents, on_command);
    }

    /// Get the help rows for the group and its subgroups
    pub fn get_help_rows<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<Vec<HelpRow>> {
        let mut rows = Vec::new();
        let mut err = None;

        self.walk_tree(keys, parents, &mut |keys, cmd, parents| {
            let is_in_scope = cmd.is_in_scope(current_dir.as_ref(), git_root);

            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match is_in_scope {
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

            if let Some(command) = cmd.get_command() {
                let alias_keys = cmd
                    .get_keys(keys, &parents)
                    .into_iter()
                    .map(|inner| inner.into_iter().map(|s| s.to_string()).collect())
                    .collect::<Vec<Vec<String>>>();

                rows.push(HelpRow::new(alias_keys, command));
            }

            Walk::Continue
        });

        // If there was an error, return it
        if let Some(err) = err {
            return Err(err);
        } else {
            Ok(rows)
        }
    }
}

/// A command definition in a group commands field.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Command {
    /// A simple command string.
    Command(String),
    /// A command with additional configuration.
    CommandConfig(CommandConfig),
    /// A nested group of commands.
    Group(Group),
}

impl Command {
    /// Get the root path and configuration for the command
    /// - Returns a tuple of the root configuration and the resolved path
    /// - Looks at the command first, then at the parent groups
    ///
    /// This one is different from CommandDefinition::get_command_root, as it looks
    /// at all parents, not just the immediate parent group.
    pub fn get_command_root_path<'a>(&'a self, parents: &[&'a Group]) -> Result<Option<PathBuf>> {
        let command_root = match self {
            Command::CommandConfig(cmd) => cmd.root.as_ref(),
            Command::Group(group) => group.root.as_ref(),
            _ => None,
        };

        if let Some(root) = command_root.or(parents.iter().rev().find_map(|g| g.root.as_ref())) {
            let path = resolve_path(&root.path)?;
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    /// Get the root configuration for the command or group,
    ///
    /// IMPORTANT!: Unlike `get_command_root_path`, this does not resolve the parents
    /// this means it can only be used when walking the tree, and handling each group.
    ///
    /// Resolves the root path to an absolute path (including tilde expansion).
    fn get_root(&self) -> Result<(Option<RootConfig>, Option<PathBuf>)> {
        let item_root = match self {
            Command::CommandConfig(cmd) => cmd.root.clone(),
            Command::Group(group) => group.root.clone(),
            _ => None,
        };

        if let Some(root) = item_root {
            let path = resolve_path(&root.path)?;
            Ok((Some(root), Some(path)))
        } else {
            Ok((None, None))
        }
    }

    /// Check if the command or group is in scope for the current directory/git root.
    ///
    /// IMPORTANT!: This does not resolve the parents this means it can
    /// only be used when walking the tree, and handling each group.
    pub fn is_in_scope(
        &self,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<bool> {
        let root = self.get_root()?;

        if let (Some(root_config), Some(target_path)) = root {
            match root_config.scope {
                Some(RootScope::Exact) => Ok(current_dir.as_ref() == target_path),
                Some(RootScope::GitRoot) => {
                    if let Some(git_root) = git_root {
                        Ok(current_dir.as_ref().starts_with(&git_root) && git_root == &target_path)
                    } else {
                        Ok(false)
                    }
                }
                Some(RootScope::Global) => Ok(true),
                None => Ok(true),
            }
        } else {
            Ok(true)
        }
    }

    /// Get the command keys for a given command definition
    /// - This function collects the keys from the command and its parent groups,
    /// - Resolves aliases if they exist
    /// - Returns a vector of vectors, where each represents one level of aliases
    pub fn get_keys<'a>(&'a self, keys: &[&'a str], parents: &[&'a Group]) -> Vec<Vec<&'a str>> {
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
        match self {
            Command::Command(_) => (),
            Command::CommandConfig(command) => {
                if let Some(aliases) = &command.aliases {
                    for alias in aliases {
                        command_keys.push(alias);
                    }
                }
            }
            Command::Group(group) => {
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

    /// Get the command string for the command definition
    pub fn get_command(&self) -> Option<String> {
        match self {
            Command::Command(cmd) => Some(cmd.clone()),
            Command::CommandConfig(cmd) => Some(cmd.command.clone()),
            Command::Group(Group { default, .. }) => default.clone(),
        }
    }
}
