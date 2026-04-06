use std::cell::RefCell;

use crate::{
    app::App,
    stations::{Station, CLASSICAL_STATIONS},
};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, Clear, LineGauge, Paragraph, Row, Table, TableState, Widget},
};
use tca_ratatui::TcaTheme;

#[derive(Debug, Clone)]
pub(crate) struct UiStyles {
    pub primary: Style,
    pub border: Style,
    pub info: Style,
    pub error: Style,
    pub warn: Style,
}

impl Default for UiStyles {
    fn default() -> Self {
        Self::from(&TcaTheme::default())
    }
}
impl From<&TcaTheme> for UiStyles {
    fn from(value: &TcaTheme) -> Self {
        UiStyles {
            primary: Style::default()
                .fg(value.ui.fg_primary)
                .bg(value.ui.bg_primary),
            border: Style::default()
                .fg(value.ui.border_primary)
                .bg(value.ui.bg_primary)
                .bold(),
            info: Style::default()
                .fg(value.semantic.info)
                .bg(value.ui.bg_primary),
            error: Style::default()
                .fg(value.semantic.error)
                .bg(value.ui.bg_primary),
            warn: Style::default()
                .fg(value.semantic.warning)
                .bg(value.ui.bg_primary),
        }
    }
}

impl From<Option<String>> for UiStyles {
    fn from(value: Option<String>) -> Self {
        Self::from(&TcaTheme::new(value.as_deref()))
    }
}

impl Widget for &App {
    /// Renders the user interface widgets.
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Create the main frame (block) for the screen border.
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
                    .style(self.styles.warn)
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
            let clear = Clear::default();
            clear.render(centered_area, buf);
            self.station_selector.render(centered_area, buf);
        }
    }
}

pub struct StationSelector {
    /// Widget state
    table_state: RefCell<TableState>,
}

pub enum StationSelectorResult {
    /// Not our problem
    None,
    /// User is scrolling
    Scrolling,
    /// User wants to close the selector
    CloseSelector,
    /// User selected a new station
    NewStation(Station),
}

impl StationSelector {
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> StationSelectorResult {
        match key_event.code {
            // Keys that always work regardless of mode.
            KeyCode::Esc => StationSelectorResult::CloseSelector,
            KeyCode::Enter => {
                let idx = self.table_state.borrow().selected().unwrap_or(0);
                StationSelectorResult::NewStation(CLASSICAL_STATIONS[idx])
            }
            KeyCode::Up => {
                self.table_state.borrow_mut().select_previous();
                StationSelectorResult::Scrolling
            }
            KeyCode::Down => {
                self.table_state.borrow_mut().select_next();
                StationSelectorResult::Scrolling
            }
            _ => StationSelectorResult::None,
        }
    }

    pub fn station(&self) -> Station {
        let idx = self.table_state.borrow().selected().unwrap_or(0);
        debug!("station idx: {:?}", idx);
        CLASSICAL_STATIONS[idx]
    }

    pub fn set_station_idx(&mut self, idx: usize) {
        self.table_state = TableState::default().with_selected(idx).into();
    }
}

impl Default for StationSelector {
    fn default() -> Self {
        StationSelector {
            table_state: TableState::default().into(),
        }
    }
}

impl Widget for &StationSelector {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // TODO: Get style
        // Create the table
        let mut rows = Vec::<Row>::new();
        let mut name_width = 0;
        let mut desc_width = 0;

        for station in CLASSICAL_STATIONS {
            name_width = name_width.max(station.name.len());
            desc_width = desc_width.max(station.description.len());
            let row = Row::new(vec![
                station.name.to_string(),
                station.description.to_string(),
            ]);
            rows.push(row);
        }

        let block = Block::bordered().title("Select a new station, Esc to cancel");
        let widths = [
            Constraint::Min(name_width as u16 + 2),
            Constraint::Percentage(100),
        ];
        let table = Table::new(rows, widths)
            .header(Row::new(vec!["Name", "Description"]))
            .highlight_symbol(">>")
            .block(block);

        ratatui::widgets::StatefulWidget::render(
            table,
            area,
            buf,
            &mut self.table_state.borrow_mut(),
        );
    }
}
