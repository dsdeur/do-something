use clap::Command;
use std::{collections::HashMap, env, fs, path::PathBuf};

fn read_tasks() -> HashMap<String, String> {
    let current_dir = env::current_dir()
        .expect("Failed to get current directory. Are you running in a valid folder?");

    let mut file_path = PathBuf::from(&current_dir);
    file_path.push("dosomething.json");

    let json_content = fs::read_to_string(&file_path)
        .expect("Failed to read dosomething.json. Make sure the file exists and is readable.");

    let tasks: HashMap<String, String> = serde_json::from_str(&json_content).expect(
        "Failed to parse dosomething.json. Ensure it is valid JSON with string key-value pairs.",
    );

    tasks
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
                .about(value_static), // Use the static string
        );
    }

    app
}

fn main() {
    // Read tasks from dosomething.json
    let tasks = read_tasks();

    // Create a HashMap for quick lookup of commands
    let task_map: HashMap<String, String> = tasks.clone();

    // Create the command line interface
    let app = create_commands(tasks);

    match app.get_matches().subcommand() {
        Some((subcommand, _)) => {
            println!("Running task: {}", subcommand);

            if let Some(command_str) = task_map.get(subcommand) {
                // Execute the command
                let status = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(command_str)
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
