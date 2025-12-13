use crate::{
    dir::resolve_path,
    env::Envs,
    group::{Group, GroupMode},
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    pub envs: Option<Envs>,
    /// Optional root configuration, to define where the command is run from.
    pub root: Option<RootConfig>,
    /// Optional aliases for the command, used to run it with different names.
    pub aliases: Option<Vec<String>>,
}

/// A command definition in a group commands field.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Command {
    /// A simple command string.
    Inline(String),
    /// A command with additional configuration.
    Config(CommandConfig),
    /// A nested group of commands.
    Group(Group),
}

impl Command {
    /// Get the root path and configuration for the command
    /// - Returns a tuple of the root configuration and the resolved path
    /// - Looks at the command first, then at the parent groups
    pub fn get_command_root_path<'a>(&'a self, parents: &[&'a Group]) -> Result<Option<PathBuf>> {
        let command_root = match self {
            Command::Config(cmd) => cmd.root.as_ref(),
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
            Command::Config(cmd) => cmd.root.clone(),
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
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<bool> {
        let root = self.get_root()?;

        if let (Some(root_config), Some(target_path)) = root {
            match root_config.scope {
                Some(RootScope::Exact) => Ok(current_dir.as_ref() == target_path),
                Some(RootScope::GitRoot) => {
                    if let Some(git_root) = git_root {
                        Ok(current_dir.as_ref().starts_with(git_root.as_ref())
                            && git_root.as_ref() == target_path)
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
            Command::Inline(_) => (),
            Command::Config(command) => {
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
    pub fn get_command(&self) -> Option<&str> {
        // Resolve a group with a default, to it's default command
        let command = if let Command::Group(group) = self {
            group.get_default_command().unwrap_or(self)
        } else {
            self
        };

        match command {
            Command::Inline(cmd) => Some(cmd),
            Command::Config(cmd) => Some(&cmd.command),
            Command::Group(_) => None,
        }
    }
}
