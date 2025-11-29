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
    let global_config = config::GlobalConfig::default();
    let tasks = read_tasks().unwrap_or_default();
    let task_map: HashMap<String, String> = tasks.clone();
    // let app = create_commands(tasks);

    run().unwrap();

    // match new_app.get_matches().subcommand() {
    //     Some((subcommand, sub_matches)) => {
    //         println!("Running task: {}", subcommand);

    //         // if let Some(command_str) = task_map.get(subcommand) {
    //         //     let extra_args: Vec<&String> = sub_matches
    //         //         .get_many::<String>("args")
    //         //         .unwrap_or_default()
    //         //         .collect();

    //         //     let full_command = if extra_args.is_empty() {
    //         //         command_str.to_string()
    //         //     } else {
    //         //         let args_str: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
    //         //         format!("{} {}", command_str, args_str.join(" "))
    //         //     };

    //         //     let mut cmd = std::process::Command::new("sh");
    //         //     cmd.arg("-c")
    //         //         .arg(&full_command)
    //         //         .stdin(Stdio::inherit())
    //         //         .stdout(Stdio::inherit())
    //         //         .stderr(Stdio::inherit());

    //         //     if std::io::stdout().is_terminal() {
    //         //         cmd.env("CLICOLOR", "1")
    //         //             .env("CLICOLOR_FORCE", "1")
    //         //             .env("FORCE_COLOR", "1");
    //         //     }

    //         //     let status = cmd
    //         //         .spawn()
    //         //         .expect("Failed to spawn command")
    //         //         .wait()
    //         //         .expect("Failed to wait on command");
    //         //     std::process::exit(status.code().unwrap_or(1));
    //         // }
    //     }
    //     None => {
    //         println!("No valid subcommand provided.");
    //     }
    // }
}
