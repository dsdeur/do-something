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
use ratatui::widgets::{HighlightSpacing, ListItem, ListState};
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
    cursor_position: (u16, u16),
    list_state: ListState,
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| {
                frame.render_widget(&mut self, frame.area());
                frame.set_cursor_position(self.cursor_position);
            })?;

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
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [input_area, list_area] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
        // let greeting = Paragraph::new("Hello World! (press 'q' to quit)");
        // frame.render_widget(greeting, frame.area());
        self.render_input(input_area, buf);
        self.render_list(list_area, buf);
    }
}

impl App {
    fn render_input(&mut self, area: Rect, buf: &mut Buffer) {
        let [prompt_area, input_area] =
            Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).areas(area);

        Paragraph::new("> ")
            .style(Style::default().fg(Color::Blue).bold())
            .render(prompt_area, buf);

        let width = input_area.width.max(0) - 0;
        let scroll = self.search_input.visual_scroll(width as usize);
        Paragraph::new(self.search_input.value())
            .block(Block::default())
            .render(input_area, buf);

        let x = self.search_input.visual_cursor().max(scroll) - scroll;
        // Store the cursor position in state, so we can set it after rendering
        self.cursor_position = (input_area.x + x as u16, input_area.y);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let block = Block::new();

        let items: Vec<ListItem> = self
            .rows
            .iter()
            .map(|row| {
                let content = row.format_colored();
                ListItem::new(content)
            })
            .collect();

        let list = ratatui::widgets::List::new(items)
            .block(block)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}

pub fn run_tui(help_rows: Vec<HelpRow>) -> Result<()> {
    color_eyre::install()?; // augment errors / panics with easy to read messages
    let mut terminal = ratatui::init();

    let app = App {
        search_input: Input::new("Yo yo!".to_string()),
        rows: help_rows,
        selection_index: 0,
        cursor_position: (0, 0),
        list_state: ListState::default(),
    };
    let app_result = app.run(&mut terminal).context("app loop failed");

    ratatui::restore();
    app_result
}
