use crate::{
    config::{self},
    dir::git_root,
    do_something::DoSomething,
    ds_file::DsFile,
    env::{get_env_by_key, match_env},
    help::{HelpRow, print_lines},
    runner::Runner,
    tui::run_tui,
};
use anyhow::{Ok, Result};
use crossterm::style::Stylize;
use std::io::IsTerminal;
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn run_help_row(row: Option<HelpRow>) -> Result<()> {
    if let Some(row) = row {
        let file = DsFile::from_file(&row.file_path)?;
        let (command, parents) = file.command_from_keys(&row.key)?;
        let (envs, default_env) = command.resolved_envs(&parents);
        let env = get_env_by_key(envs, row.env, default_env);
        let runner = Runner::from_command(command, &parents, &[], env)?;

        if let Runner::Command(cmd_str, mut command) = runner {
            println!("{}", cmd_str.dim());
            let status = command.spawn()?.wait()?;
            std::process::exit(status.code().unwrap_or(1));
        }
    }

    Ok(())
}

pub fn run_match(ds: &mut DoSomething, args_str: &[&str]) -> Result<()> {
    // Get the runner based on the provided arguments
    let match_ = ds.match_command(&args_str)?;
    let (command, parents) = ds.command_from_match(&match_)?;

    let (env, default_env) = command.resolved_envs(&parents);
    let mut extra_args = &args_str[match_.score..];

    let env = if let Some((matched_env, args)) = match_env(env, default_env, extra_args)? {
        extra_args = args;
        Some(matched_env)
    } else {
        None
    };

    let runner = Runner::from_command(command, &parents, extra_args, env)?;

    // Execute the runner
    match runner {
        Runner::Command(cmd_str, mut command) => {
            println!("{}", cmd_str.dim());
            let status = command.spawn()?.wait()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Runner::Help => {
            let lines = ds.help_rows_for_match(&match_)?;
            let file = ds.file_from_match(&match_)?;
            let max_size = lines.iter().map(HelpRow::len).max().unwrap_or(0);
            let row = run_tui(vec![(file.clone(), lines)], max_size);
            run_help_row(row.unwrap())?;

            std::process::exit(0);
        }
    }
}

/// Render the help for all commands
pub fn render_help(
    paths: &[PathBuf],
    current_dir: impl AsRef<Path>,
    git_root: Option<impl AsRef<Path>>,
) -> Result<()> {
    let mut groups = Vec::new();

    for path in paths.iter() {
        let file = DsFile::from_file(path)?;
        let rows = file.help_rows(&current_dir, git_root.as_ref())?;

        // If the group has no commands, we skip it
        if rows.is_empty() {
            continue;
        }

        groups.push((file, rows))
    }

    let max_size = groups
        .iter()
        .flat_map(|(_file, rows)| rows)
        .map(HelpRow::len)
        .max()
        .unwrap_or(0);

    // If not a terminal, we just print the help
    if !std::io::stdout().is_terminal() {
        for (file, rows) in groups {
            print_lines(&file, &rows, max_size);
        }
        return Ok(());
    }

    // Otherwise, we run the TUI
    let row = run_tui(groups, max_size).unwrap();
    run_help_row(row)
}

/// Run the CLI application
pub fn run() -> Result<()> {
    let mut ds = DoSomething::new()?;

    // Get the command line arguments, skipping the first one (the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // Load the global configuration
    let config = config::GlobalConfig::load()?;
    let paths = config.file_paths()?;

    // For scoping, get the current directory and git root
    let current_dir = std::env::current_dir()?;
    let git_root = git_root();

    if args_str.is_empty() {
        // If no arguments are provided, we render the help for all commands
        render_help(&paths, &current_dir, git_root.as_ref())?;
        std::process::exit(0);
    }

    run_match(&mut ds, &args_str)
}
