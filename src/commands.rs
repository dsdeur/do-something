use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    config::OnConflict,
    dir::{git_root, resolve_path},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RootScope {
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
pub struct CommandConfig {
    pub name: Option<String>,
    pub description: Option<String>,
    pub command: String,
    pub envs: Option<Vec<String>>,
    pub root: Option<RootConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Command {
    Command(String),
    CommandConfig(CommandConfig),
    Group(Group),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupMode {
    Namespaced,
    Flattened,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Group {
    name: Option<String>,
    description: Option<String>,
    default: Option<String>,
    commands: Box<HashMap<String, Command>>,
    envs: Option<Vec<String>>,
    dotenv_files: Option<Box<HashMap<String, String>>>,
    root: Option<RootConfig>,
    mode: Option<GroupMode>,
}

fn create_clap_command(key: String) -> clap::Command {
    clap::Command::new(key).arg(
        clap::Arg::new("args")
            .num_args(0..)
            .trailing_var_arg(true)
            .allow_hyphen_values(true),
    )
}

fn is_in_scope(
    root: &Option<RootConfig>,
    current_dir: impl AsRef<Path>,
    git_root: &Option<PathBuf>,
) -> Result<bool> {
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
            None => Ok(true),
        }
    } else {
        Ok(true)
    }
}

fn add_root(root_path: impl AsRef<Path>, cmd: &str) -> String {
    format!("cd {} && {}", root_path.as_ref().display(), cmd)
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

    pub fn merge(&mut self, other: Group, on_conflict: &OnConflict) -> Result<()> {
        let base = &mut self.commands;

        for (key, command) in *other.commands {
            if base.contains_key(&key) {
                match on_conflict {
                    OnConflict::Error => {
                        return Err(anyhow::anyhow!(
                            "Conflict detected for command key: {}. If you want to override, change the on_conflict setting to Override.",
                            key
                        ));
                    }
                    _ => (),
                }
            }

            base.insert(key, command);
        }

        Ok(())
    }

    fn resolve(&mut self, current_dir: &Path, git_root: &Option<PathBuf>) -> Result<()> {
        if !is_in_scope(&self.root, &current_dir, git_root)? {
            // Clear commands if out of scope
            self.commands.clear();
            self.default = None;
        };

        let mut new_commands: HashMap<String, Command> = HashMap::new();

        for (key, command) in self.commands.iter_mut() {
            match command {
                Command::CommandConfig(cmd) => {
                    if is_in_scope(&cmd.root, &current_dir, &git_root)? {
                        new_commands.insert(key.clone(), command.clone());
                    }
                }
                Command::Command(_) => {
                    new_commands.insert(key.clone(), command.clone());
                }
                Command::Group(sub_group) => {
                    sub_group.resolve(current_dir, &git_root)?;

                    match sub_group.mode {
                        Some(GroupMode::Flattened) => {
                            new_commands.extend(sub_group.commands.drain().map(|(k, v)| match v {
                                Command::CommandConfig(command) => {
                                    if command.root.is_none() {
                                        let new_command = Command::CommandConfig(CommandConfig {
                                            root: sub_group.root.clone(),
                                            ..command
                                        });
                                        (k, new_command)
                                    } else {
                                        (k, Command::CommandConfig(command.clone()))
                                    }
                                }
                                Command::Command(cmd) => (
                                    k,
                                    Command::CommandConfig(CommandConfig {
                                        command: cmd,
                                        root: sub_group.root.clone(),
                                        name: None,
                                        description: None,
                                        envs: None,
                                    }),
                                ),
                                _ => (k, v),
                            }));
                            sub_group.commands.clear();
                        }
                        _ => {
                            new_commands.insert(key.clone(), command.clone());
                        }
                    }
                }
            }
        }

        self.commands = Box::new(new_commands);

        Ok(())
    }

    pub fn to_clap(&mut self, name: String) -> Result<clap::Command> {
        let mut app = create_clap_command(name);
        let current_dir = std::env::current_dir()?;
        let git_root = git_root();

        self.resolve(&current_dir, &git_root)?;

        for (key, def) in self.commands.iter_mut() {
            let key = key.clone();

            match def {
                Command::Command(_) => {
                    app = app.subcommand(create_clap_command(key));
                }
                Command::CommandConfig(_) => {
                    app = app.subcommand(create_clap_command(key));
                }
                Command::Group(group) => {
                    let sub_com = group.to_clap(key.clone())?;
                    app = app.subcommand(sub_com);
                }
            }
        }

        Ok(app)
    }

    pub fn command_from_path(&self, path: &[String]) -> Result<Option<String>> {
        let mut current_group = self;
        let mut root_path = if let Some(root) = current_group.root.clone() {
            Some(resolve_path(&root.path)?)
        } else {
            None
        };

        for (i, part) in path.iter().enumerate() {
            match current_group.commands.get(part) {
                Some(Command::Group(sub_group)) => {
                    current_group = sub_group;

                    if let Some(root) = &sub_group.root {
                        root_path = Some(resolve_path(&root.path)?);
                    }
                }
                Some(command) if i == path.len() - 1 => match command {
                    Command::Command(cmd) => {
                        if let Some(path) = root_path {
                            return Ok(Some(add_root(path, cmd)));
                        } else {
                            return Ok(Some(cmd.clone()));
                        };
                    }
                    Command::CommandConfig(command) => {
                        if let Some(root) = &command.root {
                            let path = resolve_path(&root.path)?;
                            return Ok(Some(add_root(path, &command.command)));
                        } else if let Some(path) = root_path {
                            return Ok(Some(add_root(path, &command.command)));
                        } else {
                            return Ok(Some(command.command.clone()));
                        };
                    }
                    Command::Group(_) => {
                        return Ok(None);
                    }
                },
                _ => {
                    return Ok(None);
                }
            }
        }

        // If we reach here, it means the path corresponds to a group, not a command
        // Return the command of the group if it exists
        if let Some(command) = &current_group.default {
            if let Some(path) = root_path {
                return Ok(Some(add_root(path, command)));
            } else {
                return Ok(Some(command.clone()));
            };
        } else {
            Ok(None)
        }
    }
}
