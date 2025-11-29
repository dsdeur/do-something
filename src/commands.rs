use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Command {
    Command(String),
    Subcommand {
        name: Option<String>,
        description: Option<String>,
        command: String,
        envs: Option<Vec<String>>,
    },
    Group(Group),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Group {
    name: Option<String>,
    description: Option<String>,
    command: Option<String>,
    commands: Box<HashMap<String, Command>>,
    envs: Option<Vec<String>>,
    dotenv_files: Option<Box<HashMap<String, String>>>,
}

pub fn merge_commands(base: &mut HashMap<String, Command>, new: HashMap<String, Command>) {
    for (key, command) in new {
        if base.get(&key).is_some() {
            continue;
        }

        base.insert(key, command);
    }
}

pub fn create_clap_command(key: String) -> clap::Command {
    clap::Command::new(key).arg(
        clap::Arg::new("args")
            .num_args(0..)
            .trailing_var_arg(true)
            .allow_hyphen_values(true),
    )
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

    pub fn merge(&mut self, other: Group) {
        merge_commands(&mut self.commands, *other.commands);
    }

    pub fn to_clap(&self, name: String) -> clap::Command {
        let mut app = create_clap_command(name);

        for (key, def) in self.commands.iter() {
            let key = key.clone();

            match def {
                Command::Command(_) => {
                    app = app.subcommand(create_clap_command(key));
                }
                Command::Subcommand {
                    name: _,
                    description: _,
                    command: _,
                    envs: _,
                } => {
                    app = app.subcommand(create_clap_command(key));
                }
                Command::Group(group) => {
                    let sub_com = group.to_clap(key.clone());
                    app = app.subcommand(sub_com);
                }
            }
        }

        app
    }

    pub fn command_from_path(&self, path: &[String]) -> Option<String> {
        let mut current_group = self;

        for (i, part) in path.iter().enumerate() {
            match current_group.commands.get(part) {
                Some(Command::Group(sub_group)) => {
                    current_group = sub_group;
                }
                Some(command) if i == path.len() - 1 => match command {
                    Command::Command(cmd) => {
                        return Some(cmd.clone());
                    }
                    Command::Subcommand { command: cmd, .. } => {
                        return Some(cmd.clone());
                    }
                    Command::Group(_) => {
                        return None;
                    }
                },
                _ => {
                    return None;
                }
            }
        }

        // If we reach here, it means the path corresponds to a group, not a command
        // Return the command of the group if it exists
        if let Some(command) = &current_group.command {
            Some(command.clone())
        } else {
            None
        }
    }
}
