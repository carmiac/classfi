use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::Style,
    symbols,
    text::{Line, Span},
    widgets::{Block, BorderType, LineGauge, Paragraph, Widget},
};
use tca_ratatui::TcaTheme;

use crate::app::App;

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
        let controls = Line::from(" (p)lay (s)tation (+/-) volume (q)uit ")
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
        let station = if let Some(station) = &self.station {
            Line::from(vec![
                Span::styled(format!("{} : ", station.name), self.styles.info),
                Span::styled(station.description, self.styles.primary),
            ])
        } else {
            Line::from("No Station Set").style(self.styles.warn)
        };
        let now_playing = Line::from(vec![
            Span::styled("Now Playing: ", self.styles.info),
            Span::styled(&self.player_state.title, self.styles.primary),
        ]);

        let player_state = match self.player_state.connection_state {
            crate::player::ConnectionState::Disconnected => {
                Line::from("Disconnected").style(self.styles.error)
            }
            crate::player::ConnectionState::Connecting => {
                Line::from("Connecting...").style(self.styles.info)
            }
            crate::player::ConnectionState::Buffering => {
                Line::from(format!("Buffering... {}%", self.player_state.cache))
                    .style(self.styles.warn)
            }
            crate::player::ConnectionState::Playing => {
                let seconds = self.player_state.play_time as i64;
                let hours = seconds / 3600;
                let minutes = (seconds % 3600) / 60;
                let remaining_seconds = seconds % 60;
                Line::from(format!(
                    "Play Time {:02}:{:02}:{:02}",
                    hours, minutes, remaining_seconds
                ))
                .style(self.styles.info)
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
        let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(inner);
        paragraph.render(chunks[0], buf);
        volume.render(chunks[1], buf);
    }
}
