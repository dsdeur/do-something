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
        let mut app = clap::Command::new(name);

        for (key, def) in self.commands.iter() {
            let key = key.clone();

            match def {
                Command::Command(_) => {
                    app = app.subcommand(clap::Command::new(key));
                }
                Command::Subcommand {
                    name: _,
                    description: _,
                    command: _,
                    envs: _,
                } => {
                    app = app.subcommand(clap::Command::new(key));
                }
                Command::Group(group) => {
                    let sub_com = group.to_clap(key.clone());
                    app = app.subcommand(sub_com);
                }
            }
        }

        app
    }
}
