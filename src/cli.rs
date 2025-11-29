use std::{env, io::IsTerminal, process::Stdio};

use crate::{
    commands::Group,
    config::{self, get_config_dir},
};
use anyhow::Result;
use clap;

pub fn load_commands() -> Result<Group> {
    let mut dirs = Vec::new();

    if let Some(dir) = get_config_dir() {
        dirs.push(dir);
    }

    if let Ok(dir) = env::current_dir() {
        dirs.push(dir);
    }

    let mut commands = Group::default();

    for dir in &dirs {
        if let Some(config_tasks) = Group::from_dir(dir)? {
            commands.merge(config_tasks);
        }
    }

    Ok(commands)
}

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

pub fn run() -> Result<()> {
    let config = config::GlobalConfig::load()?;
    let commands = load_commands()?;
    let app = commands.to_clap("DoSomething".to_string());

    let matches = app.get_matches();
    let (path, extra_args) = get_command_path(&matches);
    let command = commands
        .command_from_path(&path)
        .ok_or(anyhow::anyhow!("Command not found"))?;

    let full_command = if extra_args.is_empty() {
        command.to_string()
    } else {
        let args_str: Vec<&str> = extra_args.iter().map(|s| s.as_str()).collect();
        format!("{} {}", command, args_str.join(" "))
    };

    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c")
        .arg(&full_command)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if std::io::stdout().is_terminal() {
        cmd.env("CLICOLOR", "1")
            .env("CLICOLOR_FORCE", "1")
            .env("FORCE_COLOR", "1");
    }

    let status = cmd
        .spawn()
        .expect("Failed to spawn command")
        .wait()
        .expect("Failed to wait on command");

    std::process::exit(status.code().unwrap_or(1));
}
