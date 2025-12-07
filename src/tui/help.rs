use crate::commands::{Command, Group, Walk};
use crate::ds_file::DsFile;
use crossterm::style::Stylize;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct HelpRow {
    pub alias_keys: Vec<Vec<String>>,
    pub prefix: String,
    pub command: String,
}

impl HelpRow {
    pub fn new(alias_keys: Vec<Vec<String>>, command: String) -> Self {
        HelpRow {
            prefix: "ds".to_string(),
            alias_keys,
            command,
        }
    }

    pub fn get_group_keys(&self) -> String {
        let res: Vec<_> = self
            .alias_keys
            .iter()
            .take(self.alias_keys.len() - 1)
            .filter_map(|keys| keys.first().cloned())
            .collect();

        res.join(" ")
    }

    pub fn get_key(&self) -> String {
        self.alias_keys
            .last()
            .and_then(|keys| keys.first().cloned())
            .unwrap_or_else(|| "".to_string())
    }

    pub fn len(&self) -> usize {
        let mut len = self.prefix.len() + 1 + self.get_key().len();
        let group_keys = self.get_group_keys();

        if !group_keys.is_empty() {
            len += group_keys.len() + 1; // +1 for the space
        }

        len
    }
}

pub fn print_lines(file: &DsFile, lines: Vec<HelpRow>, max_width: usize) {
    if lines.is_empty() {
        return;
    }

    if let Some(name) = &file.group.name {
        println!("\n{}", name.clone().stylize().green().bold());
    }

    if let Some(description) = &file.group.description {
        if std::io::stdout().is_terminal() {
            println!("{}", description.clone().stylize().dark_yellow().dim());
        } else {
            println!("{}", description);
        }
    }

    for row in lines {
        let group_keys = row.get_group_keys();
        let groups = if group_keys.is_empty() {
            group_keys
        } else {
            format!("{} ", group_keys)
        };

        let key = row.get_key();
        let length = row.len();

        if std::io::stdout().is_terminal() {
            let groups = format!(
                "{} {}{} {}",
                row.prefix.grey(),
                groups.dark_blue().bold(),
                key.white(),
                " ".repeat(max_width - length)
            );

            println!("{} {}", groups.blue(), row.command.dark_yellow());
        } else {
            // If not in a terminal, just print the command and path
            println!("{}{} {}", row.prefix, groups, key);
        }
    }
}

impl Group {
    pub fn print_group_help(
        &self,
        group_keys: Vec<String>,
        current_dir: impl AsRef<Path>,
        git_root: &Option<PathBuf>,
    ) -> (
        String,
        Option<String>,
        Vec<(String, String, String, String, usize)>,
        usize,
    ) {
        let group = group_keys.join(" ");
        let mut commands = Vec::new();

        self.walk_commands(&mut |keys, cmd, parents| {
            let is_in_scope = cmd.is_in_scope(current_dir.as_ref(), git_root);

            // If the command/group is not in scope, we skip it early to avoid unnecessary processing
            match is_in_scope {
                Err(_) => {
                    return Walk::Stop;
                }
                Ok(false) => return Walk::Skip,
                Ok(true) => {}
            }

            if let Command::Group(Group { default: None, .. }) = cmd {
                // If it's a group, we don't print it here, as it will be printed in the next line
                return Walk::Continue;
            }

            let keys = cmd.get_keys(keys, parents);

            let mut command = Vec::new();
            command.extend(group_keys.iter().cloned());

            for key in &keys {
                if let Some(first) = key.first() {
                    command.push(first.to_string());
                }
            }

            commands.push((command, cmd.get_command()));

            Walk::Continue
        });

        let mut max_width = 0;
        let mut lines = Vec::new();

        for (command, path) in commands {
            let (group_keys, cmd) = match command.as_slice() {
                [] => ("".to_string(), "".to_string()),
                [head @ .., last] => (head.join(" "), last.as_str().to_string()),
            };

            let mut len = 3 + cmd.len();

            if !group_keys.is_empty() {
                len += group_keys.len() + 1; // +1 for the space
            }

            max_width = max_width.max(len);
            lines.push((
                "ds".to_string(),
                group_keys,
                cmd,
                path.unwrap_or("".to_string()),
                len,
            ));
        }

        let name = self.name.clone().unwrap_or(group);

        (name, self.description.clone(), lines, max_width)
    }
}
