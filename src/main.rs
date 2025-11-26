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

    let config_dir = env::home_dir().map(|f| f.join(".config").join("dosomething"));
    if let Some(dir) = &config_dir {
        dirs.push(dir);
    }

    let current_dir = env::current_dir();
    if let Ok(dir) = &current_dir {
        dirs.push(dir);
    }

    let mut tasks = HashMap::new();

    for dir in dirs {
        if let Some(config_tasks) = process_config(dir)? {
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
        // Convert to &'static str using Box::leak
        let key_static: &'static str = Box::leak(key.into_boxed_str());
        let value_static: &'static str = Box::leak(value.into_boxed_str());

        app = app.subcommand(
            Command::new(key_static) // Use the static string
                .about(value_static) // Use the static string
                // Allow additional arguments to be passed to the command
                .arg(
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
    // Read tasks from dosomething.json
    let tasks = read_tasks().unwrap_or_default();

    // Create a HashMap for quick lookup of commands
    let task_map: HashMap<String, String> = tasks.clone();

    // Create the command line interface
    let app = create_commands(tasks);

    match app.get_matches().subcommand() {
        Some((subcommand, sub_matches)) => {
            println!("Running task: {}", subcommand);

            if let Some(command_str) = task_map.get(subcommand) {
                // Get additional arguments passed after the subcommand
                let extra_args: Vec<&String> = sub_matches
                    .get_many::<String>("args")
                    .unwrap_or_default()
                    .collect();

                // Build the full command string with extra arguments
                let full_command = if extra_args.is_empty() {
                    command_str.to_string()
                } else {
                    let args_str: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
                    format!("{} {}", command_str, args_str.join(" "))
                };

                // Execute the command
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
