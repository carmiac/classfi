use std::collections::HashMap;

use crate::cli::AppConfig;
use crate::event::{AppEvent, Event, EventHandler};
use crate::player::{Player, PlayerCommand, PlayerState};
use crate::stations::{CLASSICAL_STATIONS, Station};
use crate::ui::{StationSelector, UiStyles};

use color_eyre::eyre::{OptionExt, eyre};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::FutureExt;
use radiobrowser::RadioBrowserAPI;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use url::Url;

/// Application.
pub struct App {
    running: bool,
    event_handler: EventHandler,
    state_tx: mpsc::UnboundedSender<PlayerState>,
    state_rx: mpsc::UnboundedReceiver<PlayerState>,
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<PlayerCommand>>,
    pub(crate) station: Station,
    station_urls: HashMap<Station, Url>,
    pub(crate) station_selector: StationSelector,
    pub(crate) show_station_selector: bool,
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
            station: CLASSICAL_STATIONS[0],
            event_handler: EventHandler::new(),
            state_tx,
            state_rx,
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            station_selector: StationSelector::default(),
            styles: UiStyles::default(),
            player_state: PlayerState::default(),
            station_urls: HashMap::new(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(config: AppConfig) -> Self {
        let mut station_selector = StationSelector::default();
        station_selector.set_station_idx(config.station);
        App {
            styles: UiStyles::from(config.theme),
            station_selector,
            ..Default::default()
        }
    }

    /// Get the stream URL if available and cache it.
    pub fn get_url(&self, station: Station) {
        // Spawn a thread to et it from Radio Browser
        info!("Getting URL for {}", station.name);
        let sender = self.event_handler.sender.clone();
        tokio::spawn(async move {
            if let Ok(api) = RadioBrowserAPI::new().await.map_err(|e| e.to_string())
            && let Ok(stations) = api
                .get_stations()
                .name(station.name)
                .name_exact(true)
                .send()
                .await
                .map_err(|e| e.to_string())
            && let Some(s) = stations.into_iter().next()
            {
                info!("Got URL: {} for station {:?}", s.url_resolved, station);
                if let Ok(url) = Url::parse(s.url_resolved.as_str()) {
                        _ = sender.send(Event::App(AppEvent::NewStationUrl(station, url)));
                } else {
                    _ = sender.send(Event::App(AppEvent::StationUrlFailed(station)));
                }
            }
        });
    }

    /// Get the url for the new station and change to it.
    fn change_station(&mut self, station: Station) {
        self.station = station;
        // Check that we have the url.
        if let Some(url) = self.station_urls.get(&station){
               _ = self.cmd_tx.send(PlayerCommand::SetStation(url.clone()));
        } else {
            self.get_url(station);
        }
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
                            AppEvent::NewStationUrl(station, url)=> {
                                // Add to the url table
                                self.station_urls.insert(station, url.clone());  
                                // If for the current station, send the new url to the player
                                if self.station == station{
                                _ = self.cmd_tx.send(PlayerCommand::SetStation(url));
                                }
                            },
                            AppEvent::StationUrlFailed(station) => {info!("Failed url lookup for {:?}", station);}
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
