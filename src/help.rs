use crate::ds_file::DsFile;
use crossterm::style::Stylize;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::io::IsTerminal;

/// Represents a row in the help output
#[derive(Debug, Clone)]
pub struct HelpRow {
    pub file_name: String,
    pub key: Vec<String>,
    pub alias_keys: Vec<Vec<String>>,
    pub prefix: &'static str,
    pub command: String,
    pub env: Option<String>,
}

impl HelpRow {
    /// Create a new help row with the given alias keys and command
    pub fn new(
        file_name: String,
        key: Vec<String>,
        alias_keys: Vec<Vec<String>>,
        command: String,
        env: Option<String>,
    ) -> Self {
        HelpRow {
            file_name,
            key,
            prefix: "ds",
            alias_keys,
            command,
            env,
        }
    }

    /// Get the group keys as a space-separated string
    pub fn get_group_keys(&self) -> String {
        let res: Vec<_> = self
            .alias_keys
            .iter()
            .take(self.alias_keys.len() - 1)
            .filter_map(|keys| keys.first().cloned())
            .collect();

        res.join(" ")
    }

    /// Get the main key for the command, so the first key in the last alias
    pub fn get_key(&self) -> String {
        self.alias_keys
            .last()
            .and_then(|keys| keys.first().cloned())
            .unwrap_or_default()
    }

    pub fn get_key_and_env(&self) -> Vec<&str> {
        let mut res = self.key.iter().map(|s| s.as_str()).collect::<Vec<&str>>();
        if let Some(env) = &self.env {
            res.push(env);
        }
        res
    }

    /// Get the length of the row, to calculate how much space it will take in the output
    #[must_use]
    pub fn len(&self) -> usize {
        let env_size = match &self.env {
            Some(env) => env.len() + 1, // +1 for the space
            None => 0,
        };

        let mut len = self.prefix.len() + 1 + self.get_key().len() + env_size;
        let group_keys = self.get_group_keys();

        if !group_keys.is_empty() {
            len += group_keys.len() + 1; // +1 for the space
        }

        len
    }

    /// Get the formatted aliases for the command
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

    pub fn format_colored(&self) -> String {
        let group_keys = self.get_group_keys();
        let groups = if group_keys.is_empty() {
            group_keys
        } else {
            format!("{} ", group_keys)
        };

        let key = self.get_key();
        let prefix = self.prefix;

        let env = match &self.env {
            Some(env) => format!(" {}", env),
            None => "".to_string(),
        };

        format!(
            "{} {}{}{}",
            prefix.grey(),
            groups.dark_blue().bold(),
            key.white().bold(),
            env.magenta().bold(),
        )
    }

    pub fn get_id(&self) -> String {
        let env = match &self.env {
            Some(env) => format!(".{}", env),
            None => "".to_string(),
        };

        format!("{}\t{}{}", self.file_name, self.key.join("."), env)
    }

    pub fn to_string(&self, max_size: usize) -> String {
        let group_keys = self.get_group_keys();
        let groups = if group_keys.is_empty() {
            group_keys
        } else {
            format!("{} ", group_keys)
        };

        let key = self.get_key();
        let prefix = self.prefix;

        let env = match &self.env {
            Some(env) => format!(" {}", env),
            None => "".to_string(),
        };

        format!(
            "{} {}{}{} {}{}",
            prefix,
            groups,
            key,
            env,
            " ".repeat(max_size - self.len()),
            self.command
        )
    }

    pub fn to_list_line(&self, max_size: usize) -> Vec<Line<'static>> {
        let group_keys = self.get_group_keys();

        let key = self.get_key();

        let mut spans = vec![
            Span::styled(self.prefix, Style::default().fg(Color::Gray)),
            Span::raw(" "),
        ];

        if !group_keys.is_empty() {
            spans.push(Span::styled(
                format!("{} ", group_keys),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        spans.push(Span::styled(
            key,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ));

        if let Some(env) = &self.env {
            spans.push(Span::styled(
                format!(" {}", env),
                Style::default()
                    .fg(Color::LightMagenta)
                    .add_modifier(Modifier::BOLD),
            ));
        };

        spans.push(Span::styled(
            format!(" {}{}", " ".repeat(max_size - self.len()), self.command),
            Style::default().fg(Color::LightYellow),
        ));

        let mut lines = vec![Line::from(spans)];
        let aliases = self.aliases();

        if let Some(aliases) = aliases {
            lines.push(Line::from(vec![Span::styled(
                format!(" - {}", aliases),
                Style::default().fg(Color::Gray).add_modifier(Modifier::DIM),
            )]));
        }

        lines
    }
}

/// Print the help lines for a given file
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
        println!("{}", row.to_string(max_width));
    }
}
