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

use std::sync::Arc;

use color_eyre::{Result, eyre::Context};

use nucleo::Nucleo;
use ratatui::prelude::*;
use ratatui::style::palette::tailwind::SLATE;
use ratatui::widgets::{HighlightSpacing, ListItem, ListState};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

use crate::ds_file::DsFile;
use crate::help::HelpRow;

struct App {
    search_input: Input,
    groups: Vec<(DsFile, Vec<HelpRow>)>,
    cursor_position: (u16, u16),
    list_state: ListState,
    nucleo: Nucleo<HelpRow>,
    matches: Vec<HelpRow>,
    max_size: usize,
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<Option<HelpRow>> {
        self.select_first();
        self.select_next();

        loop {
            terminal.draw(|frame| {
                frame.render_widget(&mut self, frame.area());
                frame.set_cursor_position(self.cursor_position);
            })?;

            let event = event::read()?;

            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                        return Ok(None);
                    }
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Left => self.select_none(),
                    KeyCode::Down => self.select_next(),
                    KeyCode::Up => self.select_previous(),
                    KeyCode::Home => self.select_first(),
                    KeyCode::End => self.select_last(),
                    KeyCode::Enter => {
                        if let Some(selected) = self.list_state.selected() {
                            if !self.search_input.value().is_empty() {
                                if let Some(row) = self.matches.get(selected) {
                                    return Ok(Some(row.clone()));
                                }
                            } else {
                                let mut index = 0;
                                for (_file, rows) in self.groups.iter().rev() {
                                    if index == selected {
                                        break;
                                    }

                                    // Header
                                    index += 1;

                                    for row in rows {
                                        if index == selected {
                                            return Ok(Some(row.clone()));
                                        }

                                        index += 1;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        self.search_input.handle_event(&event);

                        if !self.search_input.value().is_empty() {
                            self.nucleo.pattern.reparse(
                                0,
                                self.search_input.value(),
                                nucleo::pattern::CaseMatching::Smart,
                                nucleo::pattern::Normalization::Smart,
                                false,
                            );

                            // Update filtered items
                            self.update_filtered_items();
                            // Select the best match
                            self.select_first();
                        }
                    }
                }
            }
        }
    }

    fn update_filtered_items(&mut self) {
        // Tick to process matching (with 10ms timeout)
        self.nucleo.tick(10);

        let snapshot = self.nucleo.snapshot();
        self.matches = snapshot
            .matched_items(..)
            .take(100)
            .map(|item| item.data.clone())
            .collect();
    }

    fn select_none(&mut self) {
        self.list_state.select(None);
    }

    fn get_max_index(&self) -> usize {
        if !self.search_input.value().is_empty() {
            return self.matches.len() - 1;
        }

        let mut max_index = 0;

        for (_file, rows) in self.groups.iter().rev() {
            // Header
            max_index += 1;
            // Rows
            max_index += rows.len();
        }

        max_index - 1
    }

    fn select_next(&mut self) {
        if let Some(index) = self.list_state.selected()
            && index == self.get_max_index()
        {
            self.select_first();
            return;
        }

        self.list_state.select_next();
    }
    fn select_previous(&mut self) {
        if let Some(index) = self.list_state.selected()
            && index == 0
        {
            self.select_last();
            return;
        }

        self.list_state.select_previous();
    }

    fn select_first(&mut self) {
        self.list_state.select_first();
    }

    fn select_last(&mut self) {
        self.list_state.select_last();
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            );
        let inner = block.inner(area);

        block.render(area, buf);

        let [input_area, list_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
        // let greeting = Paragraph::new("Hello World! (press 'q' to quit)");
        // frame.render_widget(greeting, frame.area());
        self.render_input(input_area, buf);
        self.render_list(list_area, buf);
    }
}

const SELECTED_STYLE: Style = Style::new().bg(SLATE.c700).add_modifier(Modifier::BOLD);

impl App {
    fn render_input(&mut self, area: Rect, buf: &mut Buffer) {
        let [prompt_area, input_area] =
            Layout::horizontal([Constraint::Length(2), Constraint::Min(1)]).areas(area);

        Paragraph::new("> ")
            .style(Style::default().fg(Color::Blue).bold())
            .render(prompt_area, buf);

        let width = input_area.width;
        let scroll = self.search_input.visual_scroll(width as usize);
        Paragraph::new(self.search_input.value())
            .block(Block::default())
            .render(input_area, buf);

        let x = self.search_input.visual_cursor().max(scroll) - scroll;
        // Store the cursor position in state, so we can set it after rendering
        self.cursor_position = (input_area.x + x as u16, input_area.y);
    }

    fn render_list(&mut self, area: Rect, buf: &mut Buffer) {
        let mut items = Vec::new();

        if self.search_input.value().is_empty() {
            for (file, rows) in self.groups.iter().rev() {
                if let Some(name) = &file.group.name {
                    let line =
                        Span::styled(name.as_str(), Style::default().fg(Color::LightGreen).bold());

                    items.push(ListItem::new(vec![Line::from(""), Line::from(vec![line])]));
                } else {
                    items.push(ListItem::new(vec![Line::from("")]));
                }

                for row in rows {
                    let item = ListItem::new(row.to_list_line(self.max_size));
                    items.push(item);
                }
            }
        } else {
            items = self
                .matches
                .iter()
                .map(|row| ListItem::new(row.to_list_line(self.max_size)))
                .collect();
        };

        let list = ratatui::widgets::List::new(items)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol("  ")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        StatefulWidget::render(list, area, buf, &mut self.list_state);
    }
}

fn create_nucleo(groups: &[(DsFile, Vec<HelpRow>)], max_size: usize) -> Nucleo<HelpRow> {
    let nucleo: Nucleo<HelpRow> = Nucleo::new(nucleo::Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = nucleo.injector();

    for (_file, rows) in groups.iter().rev() {
        for row in rows.iter() {
            injector.push(row.clone(), |r, cols| {
                cols[0] = r.to_string(max_size).into();
            });
        }
    }

    nucleo
}

pub fn run_tui(groups: Vec<(DsFile, Vec<HelpRow>)>, max_size: usize) -> Result<Option<HelpRow>> {
    color_eyre::install()?; // augment errors / panics with easy to read messages
    let mut terminal = ratatui::init();
    let nucleo = create_nucleo(&groups, max_size);

    let app = App {
        search_input: Input::new("".to_string()),
        groups,
        cursor_position: (0, 0),
        list_state: ListState::default(),
        nucleo,
        matches: Vec::new(),
        max_size,
    };

    let app_result = app.run(&mut terminal).context("app loop failed");

    ratatui::restore();
    app_result
}
