use crate::dir::resolve_path;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
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
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
    pub commands: BTreeMap<String, CommandDefinition>,
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
    /// Load a group configuration from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Option<Self>> {
        if !path.as_ref().exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let group: Group = serde_json::from_str(&content)?;

        Ok(Some(group))
    }
}

/// A command definition in a group commands field.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CommandDefinition {
    /// A simple command string.
    Command(String),
    /// A command with additional configuration.
    CommandConfig(CommandConfig),
    /// A nested group of commands.
    Group(Group),
}

impl CommandDefinition {
    /// Get the root configuration for the command or group,
    /// falling back to the parent root if not defined.
    ///
    /// Resolves the root path to an absolute path (including tilde expansion).
    fn get_root_for_scope(
        &self,
        parent_root: Option<RootConfig>,
    ) -> Result<(Option<RootConfig>, Option<PathBuf>)> {
        let item_root = match self {
            CommandDefinition::CommandConfig(cmd) => cmd.root.clone(),
            CommandDefinition::Group(group) => group.root.clone(),
            _ => None,
        };

        if let Some(root) = item_root.or(parent_root) {
            let path = resolve_path(&root.path)?;
            Ok((Some(root), Some(path)))
        } else {
            Ok((None, None))
        }
    }

    /// Check if the command or group is in scope for the current directory/git root.
    pub fn is_in_scope(
        &self,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<bool> {
        let root = self.get_root_for_scope(None)?;

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
}
