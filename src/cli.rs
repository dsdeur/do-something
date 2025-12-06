use crate::{
    commands::Group,
    config::{self, GlobalConfig},
    dir::git_root,
    runner::{Runner, get_runner},
};
use anyhow::Result;
use std::env;

/// Load and combine commands from configuration files in standard directories
pub fn load_commands(config: &GlobalConfig, matches: Vec<&str>) -> Result<Option<Runner>> {
    let paths = config.get_command_paths()?;

    // For scoping, get the current directory and git root
    let current_dir = std::env::current_dir()?;
    let git_root = git_root();

    // Have to get the groups first, as otherwise having borrowing trouble
    // We reverse the paths to get the most specific ones first,
    // as in override mode we want the last one to win
    let mut groups = Vec::new();
    for path in paths.iter().rev() {
        if let Some(group) = Group::from_file(&path)? {
            groups.push(group);
        }
    }

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

        get_runner(command, parents, &extra_args)
    })
    .transpose()
}

/// Run the CLI application
pub fn run() -> Result<()> {
    // Load the global configuration
    let config = config::GlobalConfig::load()?;

    // Get the command line arguments, skipping the first one (the program name)
    let parts: Vec<String> = env::args().skip(1).collect();

    // Get the runner based on the provided arguments
    let runner = load_commands(&config, parts.iter().map(|s| s.as_str()).collect())
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
        Runner::Help(group) => {
            return Err(anyhow::anyhow!(
                "Help requested for group: {:?}",
                group.name
            ));
        }
    }
}
