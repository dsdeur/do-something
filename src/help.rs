use crate::ds_file::DsFile;
use anyhow::Result;
use crossterm::style::Stylize;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
// use std::io::{self, Write};
use std::{
    io::{IsTerminal, Write},
    process::{Command, Stdio},
};

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
            let env = match &row.env {
                Some(env) => format!(" {}", env),
                None => "".to_string(),
            };

            let cmd = format!(
                "{} {}{}{} {}",
                prefix.grey(),
                groups.dark_blue().bold(),
                key.white().bold(),
                env.magenta().bold(),
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

pub fn run_fzf(
    groups: Vec<(DsFile, Vec<HelpRow>)>,
    max_width: usize,
) -> Result<Option<(String, Vec<String>)>> {
    let mut child = Command::new("fzf")
        .args([
            "--layout=reverse",
            "--border",
            "--ansi",
            "--cycle",
            "--highlight-line",
            "--with-nth=3..",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().expect("stdin piped");
        for (file, rows) in groups.iter().rev() {
            if let Some(name) = &file.group.name {
                let formatted = name.as_str().stylize().green().bold();
                let file_name = &file.file_name;
                writeln!(stdin, "{file_name}\t.\t {formatted}")?;
            }

            for row in rows {
                let id = row.get_id();
                let row_str = format!(
                    "{}\t{} {}{}",
                    id,
                    row.format_colored(),
                    " ".repeat(max_width - row.len()),
                    row.command
                );

                writeln!(stdin, "{row_str}")?;
            }

            writeln!(stdin, "")?;
        }
    }

    let output = child.wait_with_output()?;

    if output.status.code() == Some(0) {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let sp = s.split('\t').collect::<Vec<&str>>();
        let file_name = sp.get(0);
        let key = sp
            .get(1)
            .unwrap_or(&"")
            .split('.')
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if let Some(file_name) = file_name {
            Ok(Some((file_name.to_string(), key)))
        } else {
            Ok(None)
        }
    } else if output.status.code() == Some(1) {
        Ok(None) // user cancelled
    } else {
        Err(anyhow::anyhow!(
            "fzf failed with exit code: {:?}",
            output.status.code()
        ))
    }
}
