use std::cell::RefCell;

use crate::{
    app::App,
    stations::{ClassicalStations, Station},
};
use clap::ValueEnum;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, LineGauge, Paragraph, Row, Table, TableState, Widget},
};
use tca_ratatui::StyleSet;

impl Widget for &App {
    /// Renders the user interface widgets.
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create the main frame (block) for the screen border./
        let title = Line::from("ClassFi")
            .centered()
            .style(self.styles.border.bold());
        let controls = Line::from(" (p)lay/pause (s)tation (+/-) volume (q)uit ")
            .centered()
            .style(self.styles.border);
        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded)
            .title_bottom(controls)
            .border_style(self.styles.border)
            .style(self.styles.primary);

        // Create the lines that make up the main display info.
        let station = Line::from(vec![
            Span::styled(format!("{} : ", self.station.name), self.styles.info),
            Span::styled(self.station.description, self.styles.primary),
        ]);
        let now_playing = Line::from(vec![
            Span::styled("Now Playing: ", self.styles.info),
            Span::styled(&self.player_state.title, self.styles.primary),
        ]);

        let player_state = match self.player_state.connection_state {
            crate::player::ConnectionState::Disconnected => {
                Line::from("Disconnected").style(self.styles.error)
            }
            // crate::player::ConnectionState::Connecting => {
            //     Line::from("Connecting...").style(self.styles.info)
            // }
            crate::player::ConnectionState::Buffering => {
                Line::from(format!("Buffering... {}%", self.player_state.cache))
                    .style(self.styles.warning)
            }
            crate::player::ConnectionState::Playing => {
                let seconds = self.player_state.play_time as i64;
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;
                let remaining_seconds = seconds % 60;

                Line::from(vec![
                    Span::styled("Play Time: ", self.styles.info),
                    Span::styled(
                        format!("{:02}:{:02}:{:02}", hours, minutes, remaining_seconds),
                        self.styles.primary,
                    ),
                ])
            }
            crate::player::ConnectionState::Paused => Line::from("Paused").style(self.styles.info),
        };

        let text = vec![station, now_playing, player_state];
        let paragraph = Paragraph::new(text).left_aligned();

        // Create the volume gauge.
        let volume = LineGauge::default()
            .filled_style(self.styles.info)
            .unfilled_style(self.styles.border)
            .filled_symbol(symbols::line::THICK_HORIZONTAL)
            .ratio(self.player_state.volume as f64 / 100.0)
            .label(format!("Volume: {}%", self.player_state.volume));

        // Render all the things.
        let inner = block.inner(area);
        block.render(area, buf);
        let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(3)]).split(inner);
        paragraph.render(chunks[0], buf);
        volume.render(chunks[1], buf);
        // Create the popup.
        if self.show_station_selector {
            let centered_area =
                area.centered(Constraint::Percentage(65), Constraint::Percentage(65));
            Clear.render(centered_area, buf);
            self.station_selector.render(centered_area, buf);
        }
    }
}

pub struct StationSelector {
    /// Widget state
    table_state: RefCell<TableState>,
    /// Style Set
    styles: Option<StyleSet>,
}

pub enum StationSelectorResult {
    /// User is scrolling
    Scrolling,
    /// User wants to close the selector
    CloseSelector,
    /// User selected a new station
    NewStation(Station),
}

impl StationSelector {
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> Option<StationSelectorResult> {
        match key_event.code {
            // Keys that always work regardless of mode.
            KeyCode::Esc => Some(StationSelectorResult::CloseSelector),
            KeyCode::Enter => {
                let idx = self.table_state.borrow().selected().unwrap_or(0);
                Some(StationSelectorResult::NewStation(
                    ClassicalStations::value_variants()[idx].station(),
                ))
            }
            KeyCode::Up => {
                self.table_state.borrow_mut().select_previous();
                Some(StationSelectorResult::Scrolling)
            }
            KeyCode::Down => {
                self.table_state.borrow_mut().select_next();
                Some(StationSelectorResult::Scrolling)
            }
            _ => None,
        }
    }

    pub fn station(&self) -> Station {
        let idx = self.table_state.borrow().selected().unwrap_or(0);
        ClassicalStations::value_variants()[idx].station()
    }

    pub fn styles(mut self, styles: StyleSet) -> Self {
        self.styles = Some(styles);
        self
    }
}

impl Default for StationSelector {
    fn default() -> Self {
        StationSelector {
            table_state: TableState::default().into(),
            styles: None,
        }
    }
}

use std::sync::LazyLock;

static STATION_COL_WIDTHS: LazyLock<(u16, u16)> = LazyLock::new(|| {
    ClassicalStations::value_variants()
        .iter()
        .map(|v| v.station())
        .fold((0, 0), |(name_w, desc_w), s| {
            (
                name_w.max(s.name.len() as u16),
                desc_w.max(s.description.len() as u16),
            )
        })
});

impl Widget for &StationSelector {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (name_width, desc_width) = *STATION_COL_WIDTHS;
        let rows: Vec<Row> = ClassicalStations::value_variants()
            .iter()
            .map(|s| {
                Row::new(vec![
                    s.station().name.to_string(),
                    s.station().description.to_string(),
                ])
            })
            .collect();

        let widths = [
            Constraint::Min(name_width + 2),
            Constraint::Min(desc_width + 2),
        ];

        let block = Block::bordered().title("Select a new station, Esc to cancel");
        let block = if let Some(style) = &self.styles {
            block
                .border_style(style.border)
                .title_style(style.primary)
                .style(style.primary)
        } else {
            block
        };

        let table = Table::new(rows, widths).block(block);
        let table = if let Some(style) = &self.styles {
            table
                .header(Row::new(vec!["Name", "Description"]).style(style.info))
                .style(style.primary)
                .row_highlight_style(style.selection)
        } else {
            table.header(Row::new(vec!["Name", "Description"]))
        };

        ratatui::widgets::StatefulWidget::render(
            table,
            area,
            buf,
            &mut self.table_state.borrow_mut(),
        );
    }
}
