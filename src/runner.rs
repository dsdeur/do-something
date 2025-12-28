use crate::{
    command::{Command, CommandConfig},
    env::{Env, RunnerEnv},
    group::Group,
};
use anyhow::Result;
use shell_escape::escape;
use std::{
    borrow::Cow,
    io::IsTerminal,
    path::Path,
    process::{Command as ProcessCommand, Stdio},
};

/// Create a command to run in the shell
fn create_command(
    command: &str,
    work_dir: Option<impl AsRef<Path>>,
    args: &[&str],
    env: Option<&Env>,
    file_path: impl AsRef<Path>,
) -> Result<(ProcessCommand, String)> {
    let mut cmd = ProcessCommand::new("sh");
    let mut command_str = command.to_string();

    // Handle environment
    if let Some(env) = env {
        let RunnerEnv { command, vars } = env.get_env_vars(file_path)?;

        // Prepend the command if specified
        if let Some(cmd) = command {
            command_str = format!("{} {}", cmd, command_str);
        }

        // Set the custom environment variables
        if let Some(vars) = vars {
            cmd.envs(vars);
        }
    }

    for arg in args {
        command_str.push(' ');
        command_str.push_str(&escape(Cow::Borrowed(arg)));
    }

    cmd.arg("-c");
    cmd.arg(&command_str);
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

    Ok((cmd, command_str))
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
    /// Create a new command runner
    pub fn new_command(
        command: &str,
        path: Option<impl AsRef<Path>>,
        args: &[&str],
        env: Option<&Env>,
        file_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let (cmd, cmd_str) = create_command(command, path, args, env, file_path)?;
        Ok(Runner::Command(cmd_str, Box::new(cmd)))
    }

    /// Get the command runner for a given command definition
    /// - Returns a `Runner` enum that can either be a command to run or a help group
    /// - If a group has a default command, it will create a command runner for that
    /// - It handles root paths and arguments
    pub fn from_command(
        command: &Command,
        parents: &[&Group],
        extra_args: &[&str],
        env: Option<&Env>,
        file_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let path = command.resolve_root_path(parents, &file_path)?;

        let runner = match command {
            Command::Inline(cmd) => {
                Runner::new_command(cmd, path.as_ref(), extra_args, env, file_path)?
            }
            Command::Config(CommandConfig { command: cmd, .. }) => {
                Runner::new_command(cmd, path.as_ref(), extra_args, env, file_path)?
            }
            Command::Group(_group) => Runner::Help,
        };

        Ok(runner)
    }
}
