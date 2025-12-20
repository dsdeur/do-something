use crate::{
    config::{self},
    dir::git_root,
    ds_file::{DsFile, Match},
    help::{HelpRow, print_lines, run_fzf},
    runner::Runner,
    tui::run_tui,
};
use anyhow::Result;
use crossterm::style::Stylize;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Find and match a command in the provided paths
pub fn match_command(
    config: &config::GlobalConfig,
    paths: &[PathBuf],
    target: &[&str],
    current_dir: impl AsRef<Path>,
    git_root: Option<impl AsRef<Path>>,
) -> Result<Vec<(DsFile, Vec<Match>)>> {
    // Collect the matches
    let mut files = Vec::new();
    let mut match_count = 0;

    for path in paths.iter().rev() {
        let file = DsFile::from_file(path)?;
        let matches = file.get_matches(target, &current_dir, git_root.as_ref())?;

        if !matches.is_empty() {
            match_count += matches.len();
            files.push((file, matches));
        }

        match config.on_conflict {
            // Since we are reverse iterating, we can break on the first match
            config::OnConflict::Override if match_count > 0 => break,
            // If we have multiple matches, or previous files with matches, and the config is set to error,
            // we return an error
            config::OnConflict::Error if match_count > 1 => {
                return Err(anyhow::anyhow!("Conflict detected in group"));
            }
            // Otherwise we just continue to collect matches
            _ => {}
        }
    }

    Ok(files)
}

pub fn run_matches(
    matches: Vec<(DsFile, Vec<Match>)>,
    args_str: &[&str],
    current_dir: impl AsRef<Path>,
    git_root: Option<impl AsRef<Path>>,
) -> Result<()> {
    // Get the first match, as we reverse iterate
    let (file, matches) = matches
        .first()
        .ok_or_else(|| anyhow::anyhow!("No matching command found for: {}", args_str.join(" ")))?;

    // Get the runner for the last match
    let last_match = matches
        .last()
        .ok_or_else(|| anyhow::anyhow!("No matching command found in file"))?;

    let (command, parents) = file.command_from_keys(&last_match.keys)?;
    let runner = Runner::from_match(last_match, &parents, &args_str, command)?;

    // Execute the runner
    match runner {
        Runner::Command(cmd_str, mut command) => {
            println!("{}", cmd_str.dim());
            let status = command.spawn()?.wait()?;
            std::process::exit(status.code().unwrap_or(1));
        }
        Runner::Help => {
            let lines =
                file.get_help_rows_for_match(last_match, &current_dir, git_root.as_ref())?;
            let max_size = lines.iter().map(|row| row.len()).max().unwrap_or(0);
            print_lines(file, lines, max_size);
            std::process::exit(0);
        }
    }
}

/// Render the help for all commands
pub fn render_help(
    paths: &[PathBuf],
    current_dir: impl AsRef<Path>,
    git_root: Option<impl AsRef<Path>>,
    config: &config::GlobalConfig,
) -> Result<()> {
    let mut groups = Vec::new();

    for path in paths.iter() {
        let file = DsFile::from_file(path)?;
        let rows = file.get_help_rows(&current_dir, git_root.as_ref())?;

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

    if let Some((file, rows)) = groups.first() {
        run_tui(rows.clone()).unwrap();
        return Ok(());
    };

    match config.help_mode {
        config::HelpMode::Fzf => {
            let res = run_fzf(groups, max_size)?;
            if let Some((file_name, key)) = res {
                let args_str = &key.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
                // Get the runner based on the provided arguments
                let paths = vec![PathBuf::from(file_name)];
                let matches =
                    match_command(&config, &paths, args_str, &current_dir, git_root.as_ref())?;

                run_matches(matches, &args_str, &current_dir, git_root.as_ref())?;
            }

            Ok(())
        }
        config::HelpMode::List => {
            for (file, rows) in groups {
                print_lines(&file, rows, max_size);
            }
            Ok(())
        }
    }
}

/// Run the CLI application
pub fn run() -> Result<()> {
    // Get the command line arguments, skipping the first one (the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // Load the global configuration
    let config = config::GlobalConfig::load()?;
    let paths = config.get_command_paths()?;

    // For scoping, get the current directory and git root
    let current_dir = std::env::current_dir()?;
    let git_root = git_root();

    if args_str.is_empty() {
        // If no arguments are provided, we render the help for all commands
        render_help(&paths, &current_dir, git_root.as_ref(), &config)?;
        std::process::exit(0);
    }

    // Get the runner based on the provided arguments
    let matches = match_command(&config, &paths, &args_str, &current_dir, git_root.as_ref())?;
    run_matches(matches, &args_str, &current_dir, git_root.as_ref())
}
