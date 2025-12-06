use crate::commands::{Command, Group, Walk};
use anyhow::Result;
use std::io::{self};

use crossterm::execute;
use crossterm::style::{Attribute, Color, Print, SetAttribute, SetForegroundColor};

fn print_command(group: &String, keys: String) -> Result<()> {
    execute!(
        io::stdout(),
        // Set to bold
        SetForegroundColor(Color::DarkGrey),
        Print(format!("ds ")),
        SetForegroundColor(Color::Blue),
        Print(format!("{} ", group)),
        SetForegroundColor(Color::White),
        Print(format!("{}", keys)),
        SetAttribute(Attribute::Reset),
        // Print a newline
        Print("\n"),
    )?;

    Ok(())
}

impl Group {
    pub fn print_group_help(&self, group_keys: Vec<String>, name: Option<&String>) -> Result<()> {
        let mut res = Ok(());
        let group = group_keys.join(" ");

        if let Some(name) = name.or(Some(&group)) {
            execute!(
                io::stdout(),
                SetAttribute(Attribute::Bold),
                SetAttribute(Attribute::Underlined),
                SetForegroundColor(Color::White),
                Print(format!("\n{}\n", name)),
                SetAttribute(Attribute::Reset)
            )?;
        }

        self.walk_commands(&mut |keys, _cmd, _parents| {
            if let Command::Group(_) = _cmd {
                // If it's a group, we don't print it here, as it will be printed in the next line
                return Walk::Continue;
            }

            match print_command(&group, keys.join(" ")) {
                Ok(_) => Walk::Continue,
                Err(e) => {
                    res = Err(e);
                    Walk::Stop
                }
            }
        });

        res
    }
}
