use crate::{
    config::{self},
    dir::git_root,
    ds_file::{DsFile, Match, NestingMode},
    runner::Runner,
    tui::help::print_lines,
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
    paths: &Vec<PathBuf>,
    target: Vec<&str>,
    current_dir: impl AsRef<Path>,
    git_root: &Option<PathBuf>,
) -> Result<Vec<(DsFile, Vec<Match>)>> {
    // Collect the matches
    let mut files = Vec::new();
    let mut match_count = 0;

    for path in paths.iter().rev() {
        let file = DsFile::from_file(path)?;
        let matches = file.get_matches(&target, &NestingMode::Exclude, &current_dir, &git_root)?;

        if matches.len() > 0 {
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

/// Render the help for all commands
pub fn render_help(
    paths: &Vec<PathBuf>,
    current_dir: impl AsRef<Path>,
    git_root: &Option<PathBuf>,
) -> Result<()> {
    let mut groups = Vec::new();

    for path in paths.iter().rev() {
        let file = DsFile::from_file(path)?;
        let rows = file.get_help_rows(&current_dir, &git_root)?;

        // If the group has no commands, we skip it
        if rows.is_empty() {
            continue;
        }

        groups.push((file, rows))
    }

    let max_size = groups
        .iter()
        .flat_map(|(_file, rows)| rows.iter().map(|row| row.len()))
        .max()
        .unwrap_or(0);

    for (file, rows) in groups {
        print_lines(&file, rows, max_size);
    }

    Ok(())
}

/// Run the CLI application
pub fn run() -> Result<()> {
    // Get the command line arguments, skipping the first one (the program name)
    let parts: Vec<String> = env::args().skip(1).collect();

    // Load the global configuration
    let config = config::GlobalConfig::load()?;
    let paths = config.get_command_paths()?;

    // For scoping, get the current directory and git root
    let current_dir = std::env::current_dir()?;
    let git_root = git_root();

    if parts.len() == 0 {
        // If no arguments are provided, we render the help for all commands
        render_help(&paths, &current_dir, &git_root)?;
        std::process::exit(0);
    }

    // Get the runner based on the provided arguments
    let matches = match_command(
        &config,
        &paths,
        parts.iter().map(|s| s.as_str()).collect(),
        &current_dir,
        &git_root,
    )?;

    // Get the first match, as we reverse iterate
    let (file, matches) = matches
        .first()
        .ok_or_else(|| anyhow::anyhow!("No matching command found for: {}", parts.join(" ")))?;

    // Get the runner for the last match
    let last_match = matches
        .last()
        .ok_or_else(|| anyhow::anyhow!("No matching command found in file",))?;

    let (command, parents) = file.command_from_keys(&last_match.keys)?;
    let runner = Runner::from_match(&last_match, &parents, &parts, command)?;

    // Execute the runner
    match runner {
        Runner::Command(cmd_str, mut command) => {
            println!("{}", cmd_str.dim());
            let status = command
                .spawn()
                .expect("Failed to spawn command")
                .wait()
                .expect("Failed to wait on command");

            std::process::exit(status.code().unwrap_or(1));
        }
        Runner::Help() => {
            let lines = file.get_help_rows_for_match(&last_match, &current_dir, &git_root)?;
            let max_size = lines.iter().map(|row| row.len()).max().unwrap_or(0);
            print_lines(file, lines, max_size);
            std::process::exit(0);
        }
    }
}
