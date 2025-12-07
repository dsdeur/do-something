use crate::{
    command::{Command, CommandConfig},
    ds_file::Match,
    group::Group,
};
use anyhow::Result;
use shell_escape::escape;
use std::{
    borrow::Cow,
    io::IsTerminal,
    path::PathBuf,
    process::{Command as ProcessCommand, Stdio},
};

/// Create a command to run in the shell
fn create_command(
    command: &str,
    work_dir: Option<&PathBuf>,
    args: &[&str],
) -> Result<ProcessCommand> {
    let mut command_str = command.to_string();

    for arg in args {
        command_str.push(' ');
        command_str.push_str(&escape(Cow::Borrowed(arg)));
    }

    let mut cmd = ProcessCommand::new("sh");

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
    Command(String, Box<ProcessCommand>),
    Help,
}

impl Runner {
    /// Get the command runner for a given command definition
    /// - Returns a `Runner` enum that can either be a command to run or a help group
    /// - If a group has a default command, it will create a command runner for that
    /// - It handles root paths and arguments
    pub fn from_match(
        command_match: &Match,
        parents: &[&Group],
        target: &[&str],
        command: &Command,
    ) -> Result<Self> {
        let path = command.get_command_root_path(parents)?;
        let extra_args = &target[command_match.keys.len().min(target.len())..];

        let runner = match command {
            Command::Group(Group {
                default: Some(cmd), ..
            }) => Runner::Command(
                format!("{} {}", cmd, extra_args.join(" ")),
                Box::new(create_command(cmd, path.as_ref(), extra_args)?),
            ),
            Command::Basic(cmd) => Runner::Command(
                format!("{} {}", cmd, extra_args.join(" ")),
                Box::new(create_command(cmd, path.as_ref(), extra_args)?),
            ),
            Command::CommandConfig(CommandConfig { command: cmd, .. }) => Runner::Command(
                format!("{} {}", cmd, extra_args.join(" ")),
                Box::new(create_command(cmd, path.as_ref(), extra_args)?),
            ),
            Command::Group(_group) => Runner::Help,
        };

        Ok(runner)
    }
}
