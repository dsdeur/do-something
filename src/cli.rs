use std::env;

use crate::commands::Group;
use anyhow::Result;
use clap;

pub fn load_commands() -> Result<Group> {
    let mut dirs = Vec::new();

    if let Some(dir) = env::home_dir().map(|f| f.join(".config").join("ds")) {
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

pub fn run() -> Result<()> {
    let commands = load_commands()?;
    let app = commands.to_clap("DoSomething".to_string());

    let matches = app.get_matches();
    println!("{:#?}", matches);
    Ok(())
}
