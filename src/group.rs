use crate::{
    command::{Command, RootConfig},
    env::Env,
    help::HelpRow,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

/// Util for tree walking to control the flow of the walk.
#[derive(PartialEq, Eq)]
pub enum Walk {
    Continue,
    Skip,
    Stop,
}

/// Allows to flatten groups into their parent namespace
///
/// Useful to organize commands without adding extra nesting in the CLI.
///
/// For example, if you can't introduce a `ds.json` file in a project, you can define
/// the commands in a group in your global config:
/// - Set the root path to the project git root folder, so the commands are run from there.
/// - Set root scope to `GitRoot` so the commands are only available inside that project.
/// - Set group mode to Flattened, so the commands are available without the extra step
///   (e.g. `ds command` instead of `ds group command`).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GroupMode {
    Namespaced,
    Flattened,
}

fn default_command() -> String {
    "default".to_string()
}

/// A group of commands, that share common configuration.
///
/// This is the top-level structure of a `ds.json` file, and can be nested.
/// If there are multiple files, they are merged together
/// (configured in `on_conflict` in global config).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    /// Optional name for the group, used in help messages.
    pub name: Option<String>,
    /// Optional longer description for the group, used in help messages.
    pub description: Option<String>,
    /// Optional default command for the group, if no sub-command is provided.
    /// If not provided, it will show help for the group.
    #[serde(default = "default_command")]
    pub default: String,
    /// Commands within the group. Can be commands or sub-groups.
    pub commands: BTreeMap<String, Command>,
    /// Optional environment keys (not yet implemented).
    pub envs: Option<BTreeMap<String, Env>>,
    /// Optional default environment key to use if no specific environment is set.
    pub default_env: Option<String>,
    /// Optional root configuration, to define where the group is run from.
    pub root: Option<RootConfig>,
    /// Optional group mode, to define if it is namespaced or flattened.
    pub mode: Option<GroupMode>,
    /// Optional aliases for the group, used to run it with different names.
    pub aliases: Option<Vec<String>>,
}

impl Group {
    /// Walk the group commands recursively, calling `on_command` for each command.
    #[allow(clippy::type_complexity)]
    pub fn walk_tree<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        on_command: &mut dyn FnMut(&[&str], &Command, &[&'a Group]) -> Walk,
    ) -> Walk {
        parents.push(self);

        for (key, command) in self.commands.iter() {
            keys.push(key);

            match on_command(keys, command, parents) {
                Walk::Continue => (),
                // Skip the current command, meaning don't process the group
                Walk::Skip => {
                    keys.pop();
                    continue;
                }
                // Stop the walk, meaning don't process any more commands
                Walk::Stop => {
                    keys.pop();
                    parents.pop();
                    return Walk::Stop;
                }
            };

            if let Command::Group(group) = command {
                // If the command is a group, walk through its tree
                // If the walk returns Stop, we stop processing
                if group.walk_tree(keys, parents, on_command) == Walk::Stop {
                    keys.pop();
                    parents.pop();
                    return Walk::Stop;
                }
            }

            keys.pop();
        }

        parents.pop();
        Walk::Continue
    }

    /// Walk through all commands in the group and its subgroups
    /// - Calls `on_command` for each command with the current path, command definition, and parent groups
    /// - The path is a vector of strings representing the command keys
    /// - The command definition is the current command being processed
    /// - The parent groups are the groups that lead to the current command
    #[allow(clippy::type_complexity)]
    pub fn walk_commands<'a>(
        &'a self,
        on_command: &mut dyn FnMut(&[&str], &Command, &[&'a Group]) -> Walk,
    ) {
        let mut keys = Vec::new();
        let mut parents = Vec::new();
        self.walk_tree(&mut keys, &mut parents, on_command);
    }

    /// Get the help rows for the group and its subgroups
    pub fn get_help_rows<'a>(
        &'a self,
        keys: &mut Vec<&'a str>,
        parents: &mut Vec<&'a Group>,
        current_dir: impl AsRef<Path>,
        git_root: Option<impl AsRef<Path>>,
    ) -> Result<Vec<HelpRow>> {
        let mut rows = Vec::new();
        let mut err = None;

        self.walk_tree(keys, parents, &mut |keys, cmd, parents| {
            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match cmd.is_in_scope(current_dir.as_ref(), git_root.as_ref()) {
                Err(_) => {
                    // Store the error and stop processing
                    err = Some(anyhow::anyhow!(
                        "Error determining scope for command: {}",
                        keys.join(" ")
                    ));
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            if let Some(command) = cmd.get_command() {
                let alias_keys = cmd
                    .get_keys(keys, parents)
                    .into_iter()
                    .map(|inner| inner.into_iter().map(|s| s.to_string()).collect())
                    .collect::<Vec<Vec<String>>>();

                let (envs, default_env) = cmd.get_envs(parents);
                let mut envs = envs
                    .keys()
                    .map(|f| {
                        if let Some(default_env) = default_env {
                            if *f == default_env {
                                return Some(format!("({})", f));
                            }
                        }

                        Some(f.to_string())
                    })
                    .collect::<Vec<Option<String>>>();

                if envs.is_empty() {
                    envs.push(None);
                }

                for env in envs {
                    rows.push(HelpRow::new(
                        alias_keys.clone(),
                        command.to_string(),
                        env.clone(),
                    ));
                }
            }

            Walk::Continue
        });

        err.map_or(Ok(rows), Err)
    }

    pub fn get_default_command<'a>(
        &'a self,
        parents: &mut Option<&mut Vec<&'a Group>>,
    ) -> Option<&'a Command> {
        let mut curr = self;

        loop {
            if let Some(cmd) = curr.commands.get(&curr.default) {
                match cmd {
                    Command::Config(_) | Command::Inline(_) => return Some(cmd),
                    Command::Group(group) => {
                        if let Some(parents) = parents.as_deref_mut() {
                            parents.push(curr);
                        }

                        // Continue down the group
                        curr = group;
                    }
                }
            } else {
                return None;
            };
        }
    }
}
