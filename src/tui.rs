//! # [Ratatui] Hello World example
//!
//! The latest version of this example is available in the [examples] folder in the repository.
//!
//! Please note that the examples are designed to be run against the `main` branch of the Github
//! repository. This means that you may not be able to compile with the latest release version on
//! crates.io, or the one that you have installed locally.
//!
//! See the [examples readme] for more information on finding examples that match the version of the
//! library you are using.
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use std::time::Duration;

use color_eyre::{Result, eyre::Context};

use ratatui::prelude::*;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::help::HelpRow;

struct App {
    search_input: Input,
    rows: Vec<HelpRow>,
    selection_index: usize,
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            let event = event::read()?;

            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Esc => return Ok(()),
                    _ => {
                        self.search_input.handle_event(&event);
                    }
                }
            }
        }
    }

    fn draw(&self, frame: &mut Frame) {
        let [input_area, list_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(frame.area());
        // let greeting = Paragraph::new("Hello World! (press 'q' to quit)");
        // frame.render_widget(greeting, frame.area());
        self.render_input(frame, input_area);
    }

    fn render_input(&self, frame: &mut Frame, area: Rect) {
        let [prompt_area, input_area] =
            Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).areas(area);

        let prompt = Paragraph::new("> ").style(Style::default().fg(Color::Blue).bold());
        frame.render_widget(prompt, prompt_area);

        let width = input_area.width.max(0) - 0;
        let scroll = self.search_input.visual_scroll(width as usize);
        let input = Paragraph::new(self.search_input.value()).block(Block::default());
        frame.render_widget(input, input_area);

        let x = self.search_input.visual_cursor().max(scroll) - scroll;
        frame.set_cursor_position((input_area.x + x as u16, input_area.y));
    }
}

pub fn run_tui(help_rows: Vec<HelpRow>) -> Result<()> {
    color_eyre::install()?; // augment errors / panics with easy to read messages
    let mut terminal = ratatui::init();

    let app = App {
        search_input: Input::new("Yo yo!".to_string()),
        rows: help_rows,
        selection_index: 0,
    };
    let app_result = app.run(&mut terminal).context("app loop failed");

    ratatui::restore();
    app_result
}
