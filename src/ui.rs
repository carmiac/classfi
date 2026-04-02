use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Paragraph, Widget},
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
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from("ClassFi")
            .centered()
            .style(self.styles.border.bold());
        let controls = Line::from(" (p)lay (s)tream (+/-) volume (q)uit ")
            .centered()
            .style(self.styles.border);
        let block = Block::bordered()
            .title(title)
            .title_alignment(Alignment::Center)
            .border_type(BorderType::Rounded)
            .title_bottom(controls)
            .border_style(self.styles.border)
            .style(self.styles.primary);

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
        // TODO: Player state Play duration
        // TODO: volume

        let text = vec![station, now_playing];
        let paragraph = Paragraph::new(text).block(block).left_aligned();

        paragraph.render(area, buf);
    }
}
