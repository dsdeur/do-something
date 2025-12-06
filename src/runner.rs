use crate::{
    commands::{CommandConfig, CommandDefinition, Group},
    dir::resolve_path,
};
use anyhow::Result;
use shell_escape::escape;
use std::{
    io::IsTerminal,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Get the root path and configuration for a command
/// - Returns a tuple of the root configuration and the resolved path
/// - Looks at the command first, then at the parent groups
///
/// This one is different from CommandDefinition::get_command_root, as it looks
/// at all parents, not just the immediate parent group.
pub fn get_command_root_path<'a>(
    command: &'a CommandDefinition,
    parents: &[&'a Group],
) -> Result<Option<PathBuf>> {
    let command_root = match command {
        CommandDefinition::CommandConfig(cmd) => cmd.root.as_ref(),
        CommandDefinition::Group(group) => group.root.as_ref(),
        _ => None,
    };

    if let Some(root) = command_root.or(parents.iter().rev().find_map(|g| g.root.as_ref())) {
        let path = resolve_path(&root.path)?;
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn create_command(command: &str, work_dir: Option<&PathBuf>, args: &[&str]) -> Result<Command> {
    let mut command_str = command.to_string();

    for arg in args {
        command_str.push(' ');
        command_str.push_str(&escape((*arg).into()));
    }

    let mut cmd = std::process::Command::new("sh");

    cmd.arg("-c");
    cmd.arg(command_str);
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }

    if std::io::stdout().is_terminal() {
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("FORCE_COLOR", "1");
    }

    Ok(cmd)
}

/// Enum representing the type of command runner
/// - `Command` is a command to run
/// - `Help` is a help group that provides information about commands
#[derive(Debug)]
pub enum Runner {
    Command(Command),
    Help(Group),
}

/// Get the command runner for a given command definition
/// - Returns a `Runner` enum that can either be a command to run or a help group
/// - If a group has a default command, it will create a command runner for that
/// - It handles root paths and arguments
pub fn get_runner(
    command: &CommandDefinition,
    parents: &[&Group],
    args: &[&str],
) -> Result<Runner> {
    let path = get_command_root_path(command, parents)?;

    let runner = match command {
        CommandDefinition::Group(Group {
            default: Some(cmd), ..
        }) => Runner::Command(create_command(cmd, path.as_ref(), args)?),
        CommandDefinition::Command(cmd) => {
            Runner::Command(create_command(cmd, path.as_ref(), args)?)
        }
        CommandDefinition::CommandConfig(CommandConfig { command: cmd, .. }) => {
            Runner::Command(create_command(cmd, path.as_ref(), args)?)
        }
        CommandDefinition::Group(group) => Runner::Help(group.clone()),
    };

    Ok(runner)
}
