use crate::ds_file::DsFile;
use crossterm::style::Stylize;
use std::io::IsTerminal;

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
            .unwrap_or_default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        let mut len = self.prefix.len() + 1 + self.get_key().len();
        let group_keys = self.get_group_keys();

        if !group_keys.is_empty() {
            len += group_keys.len() + 1; // +1 for the space
        }

        len
    }

    pub fn aliases(&self) -> Option<String> {
        if !self.alias_keys.iter().any(|keys| keys.len() > 1) {
            return None;
        }

        let aliases = self.alias_keys.iter().map(|keys| {
            if keys.len() == 1 {
                return keys[0].clone();
            }

            format!("({})", keys.join("|"))
        });

        Some(aliases.collect::<Vec<_>>().join(" "))
    }
}

pub fn print_lines(file: &DsFile, lines: Vec<HelpRow>, max_width: usize) {
    if lines.is_empty() {
        return;
    }

    if let Some(name) = &file.group.name {
        println!("\n{}", name.as_str().stylize().green().bold());
    }

    if let Some(description) = &file.group.description {
        if std::io::stdout().is_terminal() {
            println!("{}", description.as_str().stylize().dark_yellow().dim());
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
        let aliases = row.aliases();
        let prefix = row.prefix;

        if std::io::stdout().is_terminal() {
            let cmd = format!(
                "{} {}{} {}",
                prefix.grey(),
                groups.dark_blue().bold(),
                key.white().bold(),
                " ".repeat(max_width - length)
            );

            println!("{} {}", cmd.blue(), row.command.dark_yellow());

            if let Some(aliases) = aliases {
                println!("{}{}", " - ".dim(), aliases.dim());
            }
        } else {
            // If not in a terminal, just print the command and path
            println!("{}{} {}", prefix, groups, key);
        }
    }
}
