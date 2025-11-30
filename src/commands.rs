use crate::{
    config::OnConflict,
    dir::{git_root, resolve_path},
};
use anyhow::Result;
use clap::builder::styling;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    vec,
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
}

/// A group of commands, that share common configuration.
///
/// This is the top-level structure of a `ds.json` file, and can be nested.
/// If there are multiple files, they are merged together
/// (configured in `on_conflict` in global config).
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Group {
    /// Optional name for the group, used in help messages.
    name: Option<String>,
    /// Optional longer description for the group, used in help messages.
    description: Option<String>,
    /// Optional default command for the group, if no sub-command is provided.
    /// If not provided, it will show help for the group.
    default: Option<String>,
    /// Commands within the group. Can be commands or sub-groups.
    commands: HashMap<String, CommandDefinition>,
    /// Optional environment keys (not yet implemented).
    envs: Option<Vec<String>>,
    /// Optional dotenv files options (not yet implemented).
    dotenv_files: Option<HashMap<String, String>>,
    /// Optional root configuration, to define where the group is run from.
    root: Option<RootConfig>,
    /// Optional group mode, to define if it is namespaced or flattened.
    mode: Option<GroupMode>,
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

    /// Flatten the group into a list of commands, applying the configuration.
    pub fn flatten(&self, source: &String) -> Result<Commands> {
        let current_dir = std::env::current_dir()?;
        let git_root = git_root();
        let mut results = Vec::new();

        for (key, command) in self.commands.iter() {
            let mut sub_results = command.flatten(
                vec![],
                key.clone(),
                source,
                self.root.clone(),
                &current_dir,
                &git_root,
            )?;

            results.append(&mut sub_results.0);
        }

        Ok(Commands(results))
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
    pub fn get_root(
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
        let root = self.get_root(None)?;

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

    /// Flatten the command or group into a list of commands, applying the configuration.
    pub fn flatten(
        &self,
        parent_key: Vec<String>,
        current_key: String,
        source: &String,
        parent_root: Option<RootConfig>,
        current_dir: &Path,
        git_root: &Option<PathBuf>,
    ) -> Result<Commands> {
        // If the command is out of scope, return an empty list
        if !self.is_in_scope(current_dir, git_root)? {
            return Ok(Commands(vec![]));
        }

        let (root_config, path) = self.get_root(parent_root)?;

        // Create the new key
        let mut new_key = parent_key.clone();
        new_key.push(current_key.clone());

        let res = match &self {
            CommandDefinition::Command(cmd) => {
                let command = Command {
                    name: None,
                    description: None,
                    command: cmd.clone(),
                    env_key: None,
                    env_file: None,
                    root_path: path,
                    source_file: source.clone(),
                    key: new_key.clone(),
                };

                vec![command]
            }
            CommandDefinition::CommandConfig(cmd) => {
                let command = Command {
                    name: cmd.name.clone(),
                    description: cmd.description.clone(),
                    command: cmd.command.clone(),
                    env_key: None,
                    env_file: None,
                    root_path: path,
                    source_file: source.clone(),
                    key: new_key.clone(),
                };

                vec![command]
            }
            CommandDefinition::Group(group) => {
                let mut results = Vec::new();

                // If the group has a default command, add it first
                // We don't flatten the default, as there is no name associated with it
                if let Some(command) = &group.default {
                    let command = Command {
                        name: group.name.clone(),
                        description: group.description.clone(),
                        command: command.clone(),
                        env_key: None,
                        env_file: None,
                        root_path: path.clone(),
                        source_file: source.clone(),
                        key: new_key.clone(),
                    };

                    results.push(command);
                }

                // Process sub-commands
                for (curr_key, command) in group.commands.iter() {
                    // If the group is flattened, use the parent key
                    let command_key = if let Some(GroupMode::Flattened) = group.mode {
                        parent_key.clone()
                    } else {
                        new_key.clone()
                    };

                    let mut sub_results = command.flatten(
                        command_key,
                        curr_key.clone(),
                        source,
                        root_config.clone(),
                        &current_dir,
                        &git_root,
                    )?;

                    results.append(&mut sub_results.0);
                }

                results
            }
        };

        Ok(Commands(res))
    }
}

/// Internal representation of a command.
///
/// This has all the resolved configuration for a command.
/// All fields are final, so that the command can be executed directly.
/// All parent configuration has been applied.
#[derive(Debug, Clone)]
pub struct Command {
    pub command: String,
    pub env_key: Option<String>,
    pub env_file: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub root_path: Option<PathBuf>,
    pub source_file: String,
    pub key: Vec<String>,
}

impl Command {
    /// Get the full command to run, including changing directory if needed.
    pub fn get_command(&self) -> String {
        if let Some(root) = &self.root_path {
            format!("cd {} && {}", root.display(), self.command)
        } else {
            self.command.clone()
        }
    }

    /// Convert the command into a Clap command for CLI integration.
    pub fn to_clap_command(&self) -> clap::Command {
        let key = self.key.last().unwrap().clone();

        let mut command = clap::Command::new(key)
            .arg(
                clap::Arg::new("args")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true),
            )
            .flatten_help(true);

        if let Some(name) = &self.name {
            command = command.about(name);
        } else {
            command = command.about(&self.get_command());
        }

        if let Some(description) = &self.description {
            command = command.long_about(description);
        }

        if let Some(_) = &self.env_key {
            todo!();
        }

        if let Some(_) = &self.env_file {
            todo!();
        }

        command = command.after_help(format!("(Defined in {})", self.source_file));

        command
    }
}

/// A collection of commands in a sorted tree structure.
/// This is used to turn it into the Clap command structure.
#[derive(Debug, Clone, Default)]
struct CommandTree {
    children: BTreeMap<String, CommandTree>,
    command: Option<Command>,
}

/// The internal representation of processed commands.
/// - Should have all configuration applied.
/// - Should not include out-of-scope commands.
#[derive(Debug, Clone, Default)]
pub struct Commands(pub Vec<Command>);

impl Commands {
    /// Merge two Commands collections, handling conflicts based on the configuration.
    pub fn merge(&self, other: Commands, on_conflict: &OnConflict) -> Result<Commands> {
        let mut combined: Vec<Command> = self.0.iter().chain(other.0.iter()).cloned().collect();
        let mut mapped = HashMap::new();

        for command in combined.iter() {
            if mapped.contains_key(&command.key) && matches!(on_conflict, OnConflict::Error) {
                return Err(anyhow::anyhow!(
                    "Conflict detected for command key: {}. If you want to override, change the on_conflict setting to Override.",
                    command.key.join(".")
                ));
            }

            mapped.insert(command.key.clone(), command.clone());
        }

        // Preserve order while removing duplicates
        // We reverse the combined list as we override earlier commands with later ones
        combined.reverse();

        let mut seen = HashSet::new();
        let mut res = Vec::new();

        for c in &combined {
            if seen.insert(c.key.clone()) {
                if let Some(cmd) = mapped.get(&c.key) {
                    res.push(cmd.clone());
                }
            }
        }

        // Reverse back to original order, now with duplicates removed
        res.reverse();

        Ok(Commands(res))
    }

    /// Convert the flat list of commands into a tree structure for Clap.
    fn to_tree(&self) -> CommandTree {
        let mut root = CommandTree::default();

        for command in &self.0 {
            let mut current = &mut root;

            for key_part in &command.key {
                current = current
                    .children
                    .entry(key_part.clone())
                    .or_insert_with(|| CommandTree::default());
            }

            current.command = Some(command.clone());
        }

        root
    }

    /// Recursively build Clap commands from the command tree.
    fn build_commands(node: &CommandTree, parent_cmd: clap::Command) -> clap::Command {
        let mut cmd = parent_cmd;

        for (key, child) in &node.children {
            let mut sub_cmd = if let Some(command) = &child.command {
                command.to_clap_command()
            } else {
                clap::Command::new(key)
                    .arg(
                        clap::Arg::new("args")
                            .num_args(0..)
                            .trailing_var_arg(true)
                            .allow_hyphen_values(true),
                    )
                    .flatten_help(true)
                    .arg_required_else_help(true)
            };

            // Recursively add any nested subcommands
            if !child.children.is_empty() {
                sub_cmd = Commands::build_commands(child, sub_cmd);
            }

            cmd = cmd.subcommand(sub_cmd);
        }

        cmd
    }

    /// Convert the Commands collection into a Clap command structure.
    pub fn to_clap(&self, name: &str) -> Result<clap::Command> {
        const STYLES: styling::Styles = styling::Styles::styled()
            .header(styling::AnsiColor::Green.on_default().bold())
            .usage(styling::AnsiColor::Green.on_default().bold())
            .literal(styling::AnsiColor::Blue.on_default().bold())
            .placeholder(styling::AnsiColor::Cyan.on_default());

        let app = clap::Command::new(name.to_owned())
            .arg_required_else_help(true)
            .flatten_help(true)
            .styles(STYLES);

        let root = self.to_tree();
        Ok(Commands::build_commands(&root, app))
    }

    /// Find a command by its path of keys.
    pub fn command_from_path(&self, path: &[String]) -> Option<String> {
        self.0
            .iter()
            .find(|c| c.key == path)
            .map(|command| command.get_command())
    }
}
