use crate::cli::AppConfig;
use crate::event::{AppEvent, Event, EventHandler};
use crate::player::{Player, PlayerCommand, PlayerState};
use crate::stations::Station;
use crate::ui::{StationSelector, UiStyles};

use color_eyre::eyre::{OptionExt, eyre};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::FutureExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

/// Application.
pub struct App {
    running: bool,
    pub(crate) show_station_selector: bool,
    pub(crate) station: Option<Station>,
    event_handler: EventHandler,
    state_tx: mpsc::UnboundedSender<PlayerState>,
    state_rx: mpsc::UnboundedReceiver<PlayerState>,
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<PlayerCommand>>,
    pub(crate) station_selector: StationSelector,
    pub(crate) styles: crate::ui::UiStyles,
    pub(crate) player_state: PlayerState,
}

impl Default for App {
    fn default() -> Self {
        let (state_tx, state_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        Self {
            running: true,
            show_station_selector: false,
            station: None,
            event_handler: EventHandler::new(),
            state_tx,
            state_rx,
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            station_selector: StationSelector::default(),
            styles: UiStyles::default(),
            player_state: PlayerState::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(config: AppConfig) -> Self {
        App {
            styles: UiStyles::from(config.theme),
            ..Default::default()
        }
    }

    /// Get the url for the new station and change to it.
    fn change_station(&mut self, station: Station) {
        let mut s = station.clone();
        self.station = Some(station);
        let sender = self.event_handler.sender.clone();
        tokio::spawn(async move {
            let rslt = s.get_url().await;
            if let Some(url) = rslt {
                _ = sender.send(Event::App(AppEvent::NewStationUrl(url)));
            } else {
                _ = sender.send(Event::App(AppEvent::StationUrlFailed));
            }
        });
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        // Get the station list, pick the configured station, statrt looking up its url.
        self.change_station(self.station_selector.station());
        // Create the player and send it an initial config.
        let player = Player::new(
            self.state_tx.clone(),
            self.cmd_rx.take().ok_or_eyre("Couldn't get cmd_rx")?,
        );
        let mut player_join_handle = tokio::spawn(player.run()).fuse();
        self.cmd_tx.send(PlayerCommand::SetVolume(80))?;

        // Main run loop
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            tokio::select! {
                maybe_event = self.event_handler.next() => {
                    match maybe_event? {
                        Event::Tick => self.tick(),
                        Event::Crossterm(event) => match event {
                            crossterm::event::Event::Key(key_event)
                                if key_event.kind == crossterm::event::KeyEventKind::Press =>
                            {
                                self.handle_key_events(key_event)?
                            }
                            _ => {}
                        },
                        Event::App(app_event) => match app_event {
                            AppEvent::Quit=>self.quit(),
                            AppEvent::NewStationUrl(url)=>{ _ = self.cmd_tx.send(PlayerCommand::SetStation(url));},
                            AppEvent::StationUrlFailed => todo!(),
                        },
                    }
                 }
                join = &mut player_join_handle => {
                    match join {
                        Ok(Ok(())) => return Ok(()), // player quit cleanly
                        Ok(Err(e)) => return Err(e), // player quit with an error
                        Err(e) => return Err(eyre!("Player panicked: {}",e))
                    }
                }
                maybe_player = self.state_rx.recv() => {
                    match maybe_player {
                        Some(state) => {self.player_state = state;}
                        None => {break;}
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            // Keys that always work regardless of mode.
            KeyCode::Char('q') => self.event_handler.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.event_handler.send(AppEvent::Quit)
            }
            KeyCode::Char('p') => self.cmd_tx.send(PlayerCommand::Toggle)?,
            KeyCode::Char('-' | '_') => self.cmd_tx.send(PlayerCommand::VolumeDown)?,
            KeyCode::Char('+' | '=') => self.cmd_tx.send(PlayerCommand::VolumeUp)?,
            KeyCode::Char('s') => self.show_station_selector = true,
            _ => {
                if self.show_station_selector {
                    match self.station_selector.handle_key_events(key_event) {
                        crate::ui::StationSelectorResult::None => {}
                        crate::ui::StationSelectorResult::Scrolling => {}
                        crate::ui::StationSelectorResult::CloseSelector => {
                            self.show_station_selector = false
                        }
                        crate::ui::StationSelectorResult::NewStation(station) => {
                            self.show_station_selector = false;
                            self.change_station(station)
                        }
                    }
                }
                // Check mode
                // Send to mode handlers
            }
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
