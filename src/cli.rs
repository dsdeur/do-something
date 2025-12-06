use crate::{
    commands::Group,
    config::{self, GlobalConfig},
    dir::git_root,
    runner::{Runner, get_runner},
    tui::help::print_lines,
};
use anyhow::Result;
use std::env;

/// Load the config, then the command files, and match the command
pub fn match_command(
    config: &GlobalConfig,
    matches: Vec<&str>,
    groups: &Vec<Group>,
) -> Result<Option<Runner>> {
    // For scoping, get the current directory and git root
    let current_dir = std::env::current_dir()?;

    let git_root = git_root();
    // Collect the matches
    let mut results = Vec::new();
    for group in groups.iter() {
        let group_matches = group.get_matches(matches.clone(), false, &current_dir, &git_root)?;
        if group_matches.is_empty() {
            continue;
        }

        // Push the group with the matches, so it stays alive
        for m in group_matches {
            results.push(m)
        }

        match config.on_conflict {
            // Since we are reverse iterating, we can break on the first match
            config::OnConflict::Override if results.len() > 0 => break,
            // If we have multiple matches, and the config is set to error, we return an error
            config::OnConflict::Error if results.len() > 1 => {
                return Err(anyhow::anyhow!("Conflict detected in group"));
            }
            // Otherwise we just continue to collect matches
            _ => {}
        }
    }

    // We use the last item in case we are in override mode, with multiple matches in one group
    // In case of error on_conflict mode, there will be only one match
    let last = results.last();

    // Get the runner if we have a match
    last.map(|(_, keys, command, parents)| {
        // Get the extra args and flags provided
        let extra_args = matches
            .iter()
            .skip(keys.len())
            .map(|s| *s)
            .collect::<Vec<_>>();

        get_runner(keys.clone(), command, parents, &extra_args)
    })
    .transpose()
}

/// Run the CLI application
pub fn run() -> Result<()> {
    // Get the command line arguments, skipping the first one (the program name)
    let parts: Vec<String> = env::args().skip(1).collect();

    // Load the global configuration
    let config = config::GlobalConfig::load()?;
    let paths = config.get_command_paths()?;

    // Have to get the groups first, as otherwise having borrowing trouble
    // We reverse the paths to get the most specific ones first,
    // as in override mode we want the last one to win
    let mut groups = Vec::new();
    for path in paths.iter().rev() {
        if let Some(group) = Group::from_file(&path)? {
            groups.push(group);
        }
    }

    if parts.len() == 0 {
        let mut group_lines = Vec::new();

        for group in groups.iter().rev() {
            group_lines.push(group.print_group_help(vec![]));
        }

        let max_size = group_lines
            .iter()
            .map(|(_title, _description, _lines, len)| *len)
            .max()
            .unwrap_or(0);

        for (title, description, lines, _max_size) in group_lines {
            print_lines(title, description.unwrap_or_default(), lines, max_size);
        }

        std::process::exit(0);
    }

    // Get the runner based on the provided arguments
    let runner = match_command(&config, parts.iter().map(|s| s.as_str()).collect(), &groups)
        .unwrap_or(None)
        .ok_or(anyhow::anyhow!(
            "No command found matching the provided arguments"
        ))?;

    // Execute the runner
    match runner {
        Runner::Command(mut cmd) => {
            println!("Running command: {:?}", cmd);
            let status = cmd
                .spawn()
                .expect("Failed to spawn command")
                .wait()
                .expect("Failed to wait on command");

            std::process::exit(status.code().unwrap_or(1));
        }
        Runner::Help(keys, group) => {
            let lines = group.print_group_help(keys);
            print_lines(lines.0, lines.1.unwrap_or_default(), lines.2, lines.3);
            std::process::exit(0);
        }
    }
}
