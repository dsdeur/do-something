use anyhow::Result;
use clap::Command;
use std::{collections::HashMap, env, fs, path::Path};

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
    let mut app = Command::new("DoSomething CLI")
        .version("1.0")
        .about("Run tasks from dosomething.json");

    for (key, value) in tasks {
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
    let tasks = read_tasks().unwrap_or_default();
    let task_map: HashMap<String, String> = tasks.clone();
    let app = create_commands(tasks);

    match app.get_matches().subcommand() {
        Some((subcommand, sub_matches)) => {
            println!("Running task: {}", subcommand);

            if let Some(command_str) = task_map.get(subcommand) {
                let extra_args: Vec<&String> = sub_matches
                    .get_many::<String>("args")
                    .unwrap_or_default()
                    .collect();

                let full_command = if extra_args.is_empty() {
                    command_str.to_string()
                } else {
                    let args_str: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
                    format!("{} {}", command_str, args_str.join(" "))
                };

                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&full_command)
                    .status()
                    .expect("Failed to execute command");

                if !status.success() {
                    eprintln!("Command failed with exit code: {:?}", status.code());
                    std::process::exit(1);
                }
            }
        }
        None => {
            println!("No valid subcommand provided.");
        }
    }
}
