use anyhow::{Ok, Result};
use crossterm::style::Stylize;
use do_something::do_something::DoSomething;
use do_something::ds_file::DsFile;
use do_something::env::get_env_by_key;
use do_something::help::HelpRow;
use do_something::runner::Runner;
use do_something::tui::run_tui;
use std::env;
use std::io::IsTerminal;

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
    let runner = command.runner(&parents, &args_str[match_.score..])?;

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
            let row = run_tui(vec![(file.help_group(lines))], max_size);
            run_help_row(row.unwrap())?;

            std::process::exit(0);
        }
    }
}

/// Render the help for all commands
pub fn render_help(ds: &mut DoSomething) -> Result<()> {
    let (groups, max_size) = ds.help_groups()?;

    // If not a terminal, we just print the help
    if !std::io::stdout().is_terminal() {
        for group in groups {
            group.print(max_size);
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

    if args_str.is_empty() {
        // If no arguments are provided, we render the help for all commands
        render_help(&mut ds)?;
        std::process::exit(0);
    }

    run_match(&mut ds, &args_str)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
