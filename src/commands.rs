use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    vec,
};

use crate::{
    config::OnConflict,
    dir::{git_root, resolve_path},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RootScope {
    Global,
    /// The current path must be inside the git root path
    GitRoot,
    /// The current folder must match the root path exactly
    Exact,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RootConfig {
    pub path: String,
    pub scope: Option<RootScope>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupMode {
    Namespaced,
    Flattened,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommandConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub command: String,
    pub envs: Option<Vec<String>>,
    pub root: Option<RootConfig>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Group {
    name: Option<String>,
    description: Option<String>,
    default: Option<String>,
    commands: HashMap<String, CommandDefinition>,
    envs: Option<Vec<String>>,
    dotenv_files: Option<HashMap<String, String>>,
    root: Option<RootConfig>,
    mode: Option<GroupMode>,
}

impl Group {
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Option<Self>> {
        let path = dir.as_ref().join("ds.json");

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;
        let group: Group = serde_json::from_str(&content)?;

        Ok(Some(group))
    }

    pub fn flatten(&self) -> Result<Commands> {
        let current_dir = std::env::current_dir()?;
        let git_root = git_root();
        let mut results = Vec::new();

        for (key, command) in self.commands.iter() {
            let mut sub_results = command.flatten(
                vec![],
                key.clone(),
                self.root.clone(),
                &current_dir,
                &git_root,
            )?;

            results.append(&mut sub_results.0);
        }

        Ok(Commands(results))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum CommandDefinition {
    Command(String),
    CommandConfig(CommandConfig),
    Group(Group),
}

impl CommandDefinition {
    pub fn is_in_scope(
        &self,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> Result<bool> {
        let root = match self {
            CommandDefinition::CommandConfig(cmd) => cmd.root.clone(),
            CommandDefinition::Group(group) => group.root.clone(),
            _ => None,
        };

        if let Some(root_config) = root {
            let target_path = resolve_path(&root_config.path)?;

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

    pub fn get_root_config(&self) -> Option<RootConfig> {
        match self {
            CommandDefinition::CommandConfig(cmd) => cmd.root.clone(),
            CommandDefinition::Group(group) => group.root.clone(),
            _ => None,
        }
    }

    pub fn with_root(&self, root: Option<RootConfig>) -> CommandDefinition {
        match self {
            CommandDefinition::Command(cmd) => CommandDefinition::CommandConfig(CommandConfig {
                name: None,
                description: None,
                envs: None,
                command: cmd.clone(),
                root,
            }),
            CommandDefinition::CommandConfig(cmd) => {
                let mut new_cmd = cmd.clone();
                new_cmd.root = root;
                CommandDefinition::CommandConfig(new_cmd)
            }
            CommandDefinition::Group(group) => {
                let mut new_group = group.clone();
                new_group.root = root;
                CommandDefinition::Group(new_group)
            }
        }
    }

    pub fn get_root_path(&self) -> Result<Option<PathBuf>> {
        if let Some(root) = self.get_root_config() {
            let path = resolve_path(&root.path)?;
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    pub fn flatten(
        &self,
        parent_key: Vec<String>,
        current_key: String,
        parent_root: Option<RootConfig>,
        current_dir: &Path,
        git_root: &Option<PathBuf>,
    ) -> Result<Commands> {
        // If the command is out of scope, return an empty list
        if !self.is_in_scope(current_dir, git_root)? {
            return Ok(Commands(vec![]));
        }

        let item_root = self.get_root_config().or_else(|| parent_root.clone());
        let with_root = self.with_root(item_root.clone());
        let path = with_root.get_root_path()?;

        // Create the new key
        let mut new_key = parent_key.clone();
        new_key.push(current_key.clone());

        let res = match &with_root {
            CommandDefinition::Command(cmd) => {
                let command = Command {
                    name: None,
                    description: None,
                    command: cmd.clone(),
                    env_key: None,
                    env_file: None,
                    root_path: path,
                    source_file: "".to_string(),
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
                    source_file: "".to_string(),
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
                        source_file: "".to_string(),
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
                        item_root.clone(),
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
    pub fn to_clap_command(&self) -> clap::Command {
        let key = self.key.last().unwrap().clone();

        let mut command = clap::Command::new(key).arg(
            clap::Arg::new("args")
                .num_args(0..)
                .trailing_var_arg(true)
                .allow_hyphen_values(true),
        );

        if let Some(desc) = &self.description {
            command = command.about(desc);
        }

        command
    }
}

#[derive(Debug, Clone, Default)]
struct CommandTree {
    children: BTreeMap<String, CommandTree>,
    command: Option<Command>,
}

#[derive(Debug, Clone, Default)]
pub struct Commands(pub Vec<Command>);

impl Commands {
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

    fn build_commands(node: &CommandTree, parent_cmd: clap::Command) -> clap::Command {
        let mut cmd = parent_cmd;

        for (_, child) in &node.children {
            if let Some(c) = &child.command {
                let mut sub_cmd = c.to_clap_command();

                // Recursively add any nested subcommands
                if !child.children.is_empty() {
                    sub_cmd = Commands::build_commands(child, sub_cmd);
                }

                cmd = cmd.subcommand(sub_cmd);
            }
        }

        cmd
    }

    pub fn to_clap(&self, name: &str) -> Result<clap::Command> {
        let app = clap::Command::new(name.to_owned());
        let root = self.to_tree();
        Ok(Commands::build_commands(&root, app))
    }

    pub fn command_from_path(&self, path: &[String]) -> Option<String> {
        self.0.iter().find(|c| c.key == path).map(|command| {
            let cmd = &command.command;
            if let Some(root) = &command.root_path {
                format!("cd {} && {}", root.display(), cmd)
            } else {
                cmd.clone()
            }
        })
    }
}
