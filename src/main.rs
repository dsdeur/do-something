use anyhow::Result;
use clap::Command;
use std::{collections::HashMap, env, fs, io::IsTerminal, path::Path, process::Stdio};

use crate::cli::run;

mod cli;
mod commands;
mod config;

fn process_config(dir: &Path) -> Result<Option<HashMap<String, String>>> {
    let file_path = dir.join("dosomething.json");

    if !file_path.exists() {
        return Ok(None);
    }

    let json_content = fs::read_to_string(&file_path)?;
    let tasks: HashMap<String, String> = serde_json::from_str(&json_content)?;
    Ok(Some(tasks))
}

fn read_tasks() -> Result<HashMap<String, String>> {
    let mut dirs = Vec::new();

    if let Some(dir) = env::home_dir().map(|f| f.join(".config").join("dosomething")) {
        dirs.push(dir);
    }

    if let Ok(dir) = env::current_dir() {
        dirs.push(dir);
    }

    let mut tasks = HashMap::new();

    for dir in &dirs {
        if let Some(config_tasks) = process_config(&dir)? {
            tasks.extend(config_tasks);
        }
    }

    Ok(tasks)
}

fn create_commands(tasks: HashMap<String, String>) -> Command {
    let mut app = Command::new("DoSomething CLI").version("1.0");

    let mut entries: Vec<(String, String)> = tasks.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    for (key, value) in entries {
        let key_static: &'static str = Box::leak(key.into_boxed_str());
        let value_static: &'static str = Box::leak(value.into_boxed_str());

        app = app.subcommand(
            Command::new(key_static).about(value_static).arg(
                clap::Arg::new("args")
                    .num_args(0..)
                    .trailing_var_arg(true)
                    .allow_hyphen_values(true),
            ),
        );
    }

    app
}

fn main() {
    run().unwrap();
}
