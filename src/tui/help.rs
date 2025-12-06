use crate::commands::{Command, Group, Walk};
use crossterm::style::Stylize;

pub fn print_lines(
    title: String,
    lines: Vec<(String, String, String, String, usize)>,
    max_width: usize,
) {
    println!("\n{}:\n", title.stylize().green().bold());

    for (prefix, group_keys, cmd, path, len) in lines {
        let groups = if group_keys.is_empty() {
            group_keys
        } else {
            format!(" {}", group_keys)
        };

        let groups = format!(
            "{}{} {} {}",
            prefix.dark_grey(),
            groups.blue().bold(),
            cmd.white(),
            " ".repeat(max_width - len)
        );

        println!("{} {}", groups.blue(), path.black());
    }

    println!("");
}

impl Group {
    pub fn print_group_help(
        &self,
        group_keys: Vec<String>,
    ) -> (String, Vec<(String, String, String, String, usize)>, usize) {
        let group = group_keys.join(" ");
        let mut commands = Vec::new();

        self.walk_commands(&mut |keys, cmd, parents| {
            if let Command::Group(_) = cmd {
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

        (name, lines, max_width)
    }
}
