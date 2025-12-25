use crate::{
    dir::resolve_path,
    env::{Env, match_env},
    group::{Group, GroupMode},
    runner::Runner,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Configures when a command or group is available to run.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum RootScope {
    /// The command is always in scope
    #[default]
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
    #[serde(default)]
    pub scope: RootScope,
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
    pub envs: Option<BTreeMap<String, Env>>,
    /// Optional default environment key to use if no specific environment is set.
    pub default_env: Option<String>,
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
    pub fn resolve_root<'a>(&'a self, parents: &[&'a Group]) -> Result<Option<PathBuf>> {
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
    /// IMPORTANT!: Unlike `resolved_root`, this does not resolve the parents
    /// this means it can only be used when walking the tree, and handling each group.
    ///
    /// Resolves the root path to an absolute path (including tilde expansion).
    fn own_root(&self) -> Result<(Option<RootConfig>, Option<PathBuf>)> {
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

    /// Get the environment configuration for the command or group
    fn env(&self) -> Option<&BTreeMap<String, Env>> {
        match self {
            Command::Config(cmd) => cmd.envs.as_ref(),
            Command::Group(group) => group.envs.as_ref(),
            _ => None,
        }
    }

    /// Get the environment configuration for the command or group
    pub fn default_env(&self) -> Option<&String> {
        match self {
            Command::Config(cmd) => cmd.default_env.as_ref(),
            Command::Group(group) => group.default_env.as_ref(),
            _ => None,
        }
    }

    /// Get the command runner for the command definition
    pub fn runner<'a>(&'a self, parents: &[&'a Group], args: &'a [&'a str]) -> Result<Runner> {
        let (envs, default_env) = self.resolve_envs(parents);
        let mut extra_args = args;

        let env = if let Some((matched_env, args)) = match_env(envs, default_env, extra_args)? {
            extra_args = args;
            Some(matched_env)
        } else {
            None
        };

        Runner::from_command(self, &parents, extra_args, env)
    }

    /// Get the merged environment configurations from the command and its parents
    pub fn resolve_envs<'a>(
        &'a self,
        parents: &[&'a Group],
    ) -> (BTreeMap<&'a String, &'a Env>, Option<&'a str>) {
        let mut merged: BTreeMap<&String, &Env> = BTreeMap::new();
        let mut default_env = None;

        let parent_envs = parents.iter().rev().filter_map(|parent| {
            if parent.default_env.is_some() {
                default_env = parent.default_env.as_deref();
            }

            parent.envs.as_ref()
        });

        for envs in parent_envs.chain(self.env().into_iter()) {
            for (key, env) in envs.iter() {
                merged.entry(key).or_insert(env);
            }
        }

        if let Some(env) = self.default_env() {
            default_env = Some(env);
        }

        (merged, default_env)
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
        let root = self.own_root()?;

        if let (Some(root_config), Some(target_path)) = root {
            match root_config.scope {
                RootScope::Exact => Ok(current_dir.as_ref() == target_path),
                RootScope::GitRoot => {
                    if let Some(git_root) = git_root {
                        Ok(current_dir.as_ref().starts_with(git_root.as_ref())
                            && git_root.as_ref() == target_path)
                    } else {
                        Ok(false)
                    }
                }
                RootScope::Global => Ok(true),
            }
        } else {
            Ok(true)
        }
    }

    /// Get the command keys for a given command definition
    /// - This function collects the keys from the command and its parent groups,
    /// - Resolves aliases if they exist
    /// - Returns a vector of vectors, where each represents one level of aliases
    pub fn resolve_aliases<'a>(
        &'a self,
        keys: &[&'a str],
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

    /// Handle default command resolution for groups
    /// The resulting command will be the fully resolved command, which is either:
    /// - The command itself, if it's not a group
    /// - The default command of the group, if it has one
    /// - The group itself, if it has no default command
    pub fn resolve_default<'a>(&'a self, parents: &mut Option<&mut Vec<&'a Group>>) -> &'a Self {
        // Resolve a group with a default, to it's default command
        match &self {
            Command::Group(group) => group.get_default_command(parents).unwrap_or(self),
            _ => self,
        }
    }

    /// Get the command string for the command definition
    pub fn command(&self) -> Option<&str> {
        // Resolve a group with a default, to it's default command
        let command = self.resolve_default(&mut None);

        match command {
            Command::Inline(cmd) => Some(cmd),
            Command::Config(cmd) => Some(&cmd.command),
            Command::Group(_) => None,
        }
    }
}
