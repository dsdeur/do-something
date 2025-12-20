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

#[derive(Clone)]
struct SearchRow {
    row: HelpRow,
    group_index: usize,
    row_index: usize,
}

struct App {
    search_input: Input,
    groups: Vec<(DsFile, Vec<HelpRow>)>,
    cursor_position: (u16, u16),
    list_state: ListState,
    nucleo: Nucleo<SearchRow>,
    matches: Vec<SearchRow>,
}

impl App {
    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.select_first();
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
                    KeyCode::Esc => return Ok(()),
                    KeyCode::Left => self.select_none(),
                    KeyCode::Down => self.select_next(),
                    KeyCode::Up => self.select_previous(),
                    KeyCode::Home => self.select_first(),
                    KeyCode::End => self.select_last(),
                    _ => {
                        self.search_input.handle_event(&event);

                        self.nucleo.pattern.reparse(
                            0,
                            &self.search_input.value(),
                            nucleo::pattern::CaseMatching::Smart,
                            nucleo::pattern::Normalization::Smart,
                            false,
                        );

                        self.update_filtered_items();
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

    fn is_selected_header(&self) -> bool {
        if let Some(selected) = self.list_state.selected() {
            let mut index = 0;

            for (_file, rows) in self.groups.iter().rev() {
                // Check header
                if index == selected {
                    return true;
                }
                index += 1;

                // Skip rows
                index += rows.len();
            }
        }

        false
    }

    fn get_max_index(&self) -> usize {
        let mut max_index = 0;

        for (_file, rows) in self.groups.iter() {
            // Header
            max_index += 1;
            // Rows
            max_index += rows.len();
        }

        max_index - 1
    }

    fn select_next(&mut self) {
        self.list_state.select_next();

        if let Some(index) = self.list_state.selected() {
            if index > self.get_max_index() {
                self.select_first();
                return;
            }

            if self.is_selected_header() {
                self.select_next();
            }
        }
    }
    fn select_previous(&mut self) {
        self.list_state.select_previous();

        if let Some(index) = self.list_state.selected() {
            if index == 0 {
                self.select_last();
                return;
            }

            if self.is_selected_header() {
                if index == 1 {
                    self.select_last();
                } else {
                    self.select_previous();
                }
            }
        }
    }

    fn select_first(&mut self) {
        self.list_state.select_first();

        if self.is_selected_header() {
            self.select_next();
        }
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
                    let item = ListItem::from(row);
                    items.push(item);
                }
            }
        } else {
            items = self
                .matches
                .iter()
                .map(|search_row| ListItem::from(&search_row.row))
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

impl From<&HelpRow> for ListItem<'_> {
    fn from(row: &HelpRow) -> Self {
        let line = row.to_list_line();
        ListItem::new(line)
    }
}

fn create_nucleo(groups: &Vec<(DsFile, Vec<HelpRow>)>) -> Nucleo<SearchRow> {
    let nucleo: Nucleo<SearchRow> = Nucleo::new(nucleo::Config::DEFAULT, Arc::new(|| {}), None, 1);
    let injector = nucleo.injector();

    for (group_index, (_file, rows)) in groups.iter().enumerate() {
        for (row_index, row) in rows.iter().enumerate() {
            let search_row = SearchRow {
                row: row.clone(),
                group_index,
                row_index,
            };

            injector.push(search_row, |r, cols| {
                cols[0] = r.row.print().into();
            });
        }
    }

    nucleo
}

pub fn run_tui(groups: Vec<(DsFile, Vec<HelpRow>)>) -> Result<()> {
    color_eyre::install()?; // augment errors / panics with easy to read messages
    let mut terminal = ratatui::init();
    let nucleo = create_nucleo(&groups);

    let app = App {
        search_input: Input::new("".to_string()),
        groups: groups,
        cursor_position: (0, 0),
        list_state: ListState::default(),
        nucleo,
        matches: Vec::new(),
    };

    let app_result = app.run(&mut terminal).context("app loop failed");

    ratatui::restore();
    app_result
}
