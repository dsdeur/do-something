use std::{env, io::IsTerminal, process::Stdio};

use crate::{
    commands::{Commands, Group},
    config::{self, GlobalConfig},
    dir::git_root,
};
use anyhow::{Ok, Result};
use clap;

/// Load and combine commands from configuration files in standard directories
pub fn load_commands(config: &GlobalConfig, matches: Vec<&str>) -> Result<Commands> {
    let mut commands = Commands::default();
    let paths = config.get_command_paths()?;

    let current_dir = std::env::current_dir()?;
    let git_root = git_root();

    println!("\n\nMatches:");
    for path in &paths {
        if let Some(group) = Group::from_file(&path)? {
            let matches = group.get_matches(matches.clone(), true, &current_dir, &git_root)?;

            println!(
                "Found {} matches for {}:\n\n",
                matches.len(),
                path.display()
            );
            for m in matches {
                println!("{:#?}", m.1);
            }
            let group_commands = group.flatten(&path.display().to_string())?;
            commands = commands.merge(group_commands, &config.on_conflict)?;
        }
    }

    Ok(commands)
}

/// Extract the command path and any extra arguments from the clap matches
pub fn get_command_path(matches: &clap::ArgMatches) -> (Vec<String>, Vec<&String>) {
    let mut path = Vec::new();
    let mut current = matches;
    let mut extra_args = Vec::new();

    while let Some((name, sub_m)) = current.subcommand() {
        path.push(name.to_string());
        current = sub_m;
        extra_args = sub_m
            .get_many::<String>("args")
            .unwrap_or_default()
            .collect();
    }

    (path, extra_args)
}

/// Run the CLI application
pub fn run() -> Result<()> {
    let config = config::GlobalConfig::load()?;
    let parts: Vec<String> = env::args().skip(1).collect();
    println!("Args {:#?}", parts);

    let commands = load_commands(&config, parts.iter().map(|s| s.as_str()).collect())?;
    let app = commands.to_clap("DoSomething")?;

    for command in &commands.0 {
        // println!("{}", &command.key.join(" "));
    }
    Ok(())

    // let matches = app.get_matches();
    // let (path, extra_args) = get_command_path(&matches);
    // let command = commands
    //     .command_from_path(&path)
    //     .ok_or(anyhow::anyhow!("Command not found"))?;

    // let full_command = if extra_args.is_empty() {
    //     command.to_string()
    // } else {
    //     let args_str: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
    //     format!("{} {}", command, args_str.join(" "))
    // };

    // println!("Running: {}", full_command);

    // let mut cmd = std::process::Command::new("sh");
    // cmd.arg("-c")
    //     .arg(&full_command)
    //     .stdin(Stdio::inherit())
    //     .stdout(Stdio::inherit())
    //     .stderr(Stdio::inherit());

    // if std::io::stdout().is_terminal() {
    //     cmd.env("CLICOLOR", "1")
    //         .env("CLICOLOR_FORCE", "1")
    //         .env("FORCE_COLOR", "1");
    // }

    // let status = cmd
    //     .spawn()
    //     .expect("Failed to spawn command")
    //     .wait()
    //     .expect("Failed to wait on command");

    // std::process::exit(status.code().unwrap_or(1));
}
