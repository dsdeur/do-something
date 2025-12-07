use crate::{
    commands::Group,
    config::{self, GlobalConfig},
    dir::git_root,
    ds_file::{DsFile, Match, NestingMode},
    runner::{Runner, get_runner},
    tui::help::print_lines,
};
use anyhow::Result;
use crossterm::style::Stylize;
use std::{
    env,
    path::{Path, PathBuf},
};

/// Load the config, then the command files, and match the command
pub fn match_command(
    target: Vec<&str>,
    current_dir: impl AsRef<Path>,
    git_root: &Option<PathBuf>,
) -> Result<Vec<(DsFile, Vec<Match>)>> {
    // Load the global configuration
    let config = config::GlobalConfig::load()?;
    let paths = config.get_command_paths()?;

    // Collect the matches
    let mut files = Vec::new();
    let mut match_count = 0;

    for path in paths.iter().rev() {
        let file = DsFile::from_file(path)?;
        let matches = file.get_matches(&target, &NestingMode::Exclude, &current_dir, &git_root)?;

        println!(
            "Loaded {} commands from {}, {:?}, {:?}",
            matches.len(),
            path.to_string_lossy().dim(),
            matches,
            config.on_conflict
        );

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

    // Have to get the groups first, as otherwise having borrowing trouble
    // We reverse the paths to get the most specific ones first,
    // as in override mode we want the last one to win
    // let mut groups = Vec::new();
    // for path in paths.iter().rev() {
    //     let group = loader.load_file(path);

    //     if let Some(group) = Group::from_file(&path)? {
    //         groups.push(group);
    //     }
    // }

    if parts.len() == 0 {
        // let mut group_lines = Vec::new();

        // for group in groups.iter().rev() {
        //     group_lines.push(group.print_group_help(vec![], &current_dir, &git_root));
        // }

        // let max_size = group_lines
        //     .iter()
        //     .map(|(_title, _description, _lines, len)| *len)
        //     .max()
        //     .unwrap_or(0);

        // for (title, description, lines, _max_size) in group_lines {
        //     print_lines(title, description.unwrap_or_default(), lines, max_size);
        // }

        std::process::exit(0);
    }

    // Get the runner based on the provided arguments
    let matches = match_command(
        parts.iter().map(|s| s.as_str()).collect(),
        &current_dir,
        &git_root,
    )?;

    // Get the first match, as we reverse iterate
    let (file, matches) = matches
        .first()
        .ok_or_else(|| anyhow::anyhow!("No matching command found for: {}", parts.join(" ")))?;

    // Get the runner for the first match
    let first_match = matches
        .first()
        .ok_or_else(|| anyhow::anyhow!("No matching command found in file",))?;

    let parents = file.groups_from_keys(&first_match.keys);
    let runner = get_runner(&first_match, &parents, &parts)?;

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
            // let lines = group.print_group_help(keys, current_dir, &git_root);
            // print_lines(lines.0, lines.1.unwrap_or_default(), lines.2, lines.3);
            std::process::exit(0);
        }
    }
}
