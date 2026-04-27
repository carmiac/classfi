use crate::cache;
use crate::cli::AppConfig;
use crate::event::{AppEvent, Event, EventHandler};
use crate::player::{ConnectionState, Player, PlayerCommand, PlayerState};
use crate::stations::{ClassicalStations, Station};
use crate::ui::StationSelector;
use anyhow::{Result, anyhow};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::collections::HashMap;
use tca_ratatui::StyleSet;
use tokio::time::Duration;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::FutureExt;
use radiobrowser::RadioBrowserAPI;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use url::Url;

fn update_media_controls(
    controls: &mut Option<MediaControls>,
    state: &PlayerState,
    station: &Station,
) {
    let Some(c) = controls else { return };
    let playback = match state.connection_state {
        ConnectionState::Playing => MediaPlayback::Playing { progress: None },
        _ => MediaPlayback::Stopped,
    };
    if let Err(e) = c.set_playback(playback) {
        debug!("Media controls set_playback: {:?}", e);
    }
    let title = if state.title.is_empty() {
        station.name
    } else {
        state.title.as_str()
    };
    if let Err(e) = c.set_metadata(MediaMetadata {
        title: Some(title),
        artist: Some(station.name),
        ..Default::default()
    }) {
        debug!("Media controls set_metadata: {:?}", e);
    }
}

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
    pub(crate) styles: StyleSet,
    pub(crate) player_state: PlayerState,
    pub(crate) compact: bool,
}

impl Default for App {
    fn default() -> Self {
        let (state_tx, state_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        Self {
            running: true,
            show_station_selector: false,
            station: ClassicalStations::default().station(),
            event_handler: EventHandler::new(),
            state_tx,
            state_rx,
            cmd_tx,
            cmd_rx: Some(cmd_rx), // Option so it can be moved later.
            station_selector: StationSelector::default(),
            styles: StyleSet::default(),
            player_state: PlayerState::default(),
            station_urls: HashMap::new(),
            compact: false,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(config: AppConfig) -> Self {
        let styles = if let Some(name) = config.theme {
            StyleSet::from_name(&name)
        } else {
            StyleSet::default()
        };
        let station_selector = StationSelector::default().styles(styles.clone());
        let station = config.station.unwrap_or_default().station();
        App {
            station,
            styles,
            station_selector,
            station_urls: cache::load(),
            compact: config.compact,
            ..Default::default()
        }
    }

    /// Get the stream URL from RadioBrowser, retrying every 5 seconds for up to ~1 minute.
    fn get_url(&self, station: Station) {
        info!("Getting URL for {}", station.name);
        let sender = self.event_handler.sender.clone();
        tokio::spawn(async move {
            for attempt in 0..12 {
                if attempt > 0 {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
                if let Ok(api) = RadioBrowserAPI::new().await.map_err(|e| e.to_string())
                    && let Ok(stations) = api
                        .get_stations()
                        .name(station.search)
                        .name_exact(true)
                        .send()
                        .await
                        .map_err(|e| e.to_string())
                    && let Some(s) = stations.into_iter().next()
                {
                    info!("Got URL: {} for station {:?}", s.url_resolved, station);
                    if let Ok(url) = Url::parse(s.url_resolved.as_str()) {
                        _ = sender.send(Event::App(AppEvent::NewStationUrl(station, url)));
                        return;
                    }
                }
                info!(
                    "URL lookup attempt {}/12 failed for {}",
                    attempt + 1,
                    station.name
                );
            }
            _ = sender.send(Event::App(AppEvent::StationUrlFailed(station)));
        });
    }

    /// Get the url for the new station and change to it.
    fn change_station(&mut self, station: Station) {
        self.station = station;
        // Check that we have the url.
        if let Some(url) = self.station_urls.get(&station) {
            _ = self.cmd_tx.send(PlayerCommand::SetStation(url.clone()));
        } else {
            self.get_url(station);
        }
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Get the station list, pick the configured station, start looking up its url.
        self.change_station(self.station);
        // Create the player and send it an initial config.
        let player = Player::new(
            self.state_tx.clone(),
            self.cmd_rx
                .take()
                .ok_or_else(|| anyhow!("Couldn't create cmd_rx"))?,
        )?;
        let mut player_join_handle = tokio::spawn(player.run()).fuse();
        self.cmd_tx.send(PlayerCommand::SetVolume(80))?;

        // Set up OS media controls (MPRIS on Linux, MediaPlayer on macOS).
        // Use a channel to bridge the Send+static souvlaki callback into our async loop.
        // Keep _media_tx_guard alive so media_rx.recv() blocks indefinitely when
        // controls are unavailable (rather than returning None and spinning the loop).
        let (media_tx, mut media_rx) = tokio::sync::mpsc::unbounded_channel::<MediaControlEvent>();
        let _media_tx_guard = media_tx.clone();
        let mut media_controls: Option<MediaControls> = {
            let config = PlatformConfig {
                dbus_name: "classfi",
                display_name: "classfi",
                hwnd: None,
            };
            match MediaControls::new(config) {
                Ok(mut c) => {
                    match c.attach(move |ev| {
                        let _ = media_tx.send(ev);
                    }) {
                        Ok(()) => {
                            info!("Media controls attached");
                            Some(c)
                        }
                        Err(e) => {
                            warn!("Failed to attach media controls: {:?}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!("Media controls unavailable: {:?}", e);
                    None
                }
            }
        };
        update_media_controls(&mut media_controls, &self.player_state, &self.station);

        // Main run loop
        while self.running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            tokio::select! {
                maybe_event = self.event_handler.next() => {
                    match maybe_event? {
                        Event::Crossterm(event) => match event {
                            crossterm::event::Event::Key(key_event)
                                if key_event.kind == crossterm::event::KeyEventKind::Press =>
                            {
                                self.handle_key_events(key_event)?
                            }
                            _ => {}
                        },
                        Event::App(app_event) => self.handle_app_event(app_event),
                    }
                 }
                join = &mut player_join_handle => {
                    match join {
                        Ok(Ok(())) => return Ok(()), // player quit cleanly
                        Ok(Err(e)) => return Err(e), // player quit with an error
                        Err(e) => return Err(anyhow!("Player panicked: {}",e))
                    }
                }
                maybe_player = self.state_rx.recv() => {
                    match maybe_player {
                        Some(new_state) => {
                            let prev_lost = matches!(
                                self.player_state.connection_state,
                                ConnectionState::StreamLost
                            );
                            let now_lost = matches!(
                                new_state.connection_state,
                                ConnectionState::StreamLost
                            );
                            self.player_state = new_state;
                            update_media_controls(&mut media_controls, &self.player_state, &self.station);
                            // On first detection of stream loss, clear the cached URL and
                            // re-fetch so we reconnect with a fresh stream address.
                            if now_lost && !prev_lost {
                                info!("Stream lost detected, invalidating cache and retrying URL lookup");
                                let station = self.station;
                                self.station_urls.remove(&station);
                                #[cfg(not(test))]
                                if let Err(e) = cache::invalidate(station) {
                                    info!("Failed to invalidate cache on stream loss: {}", e);
                                }
                                self.get_url(station);
                            }
                        }
                        None => { break; }
                    }
                }
                Some(media_event) = media_rx.recv() => {
                    match media_event {
                        MediaControlEvent::Play
                        | MediaControlEvent::Pause
                        | MediaControlEvent::Toggle
                        | MediaControlEvent::Stop => {
                            self.cmd_tx.send(PlayerCommand::Toggle)?;
                        }
                        MediaControlEvent::SetVolume(v) => {
                            let vol = (v * 100.0).clamp(0.0, 100.0) as i64;
                            self.cmd_tx.send(PlayerCommand::SetVolume(vol))?;
                            // Echo volume back so MPRIS clients see the updated value.
                            // set_volume only exists on the Linux/BSD MPRIS backend.
                            #[cfg(all(unix, not(any(target_os = "macos", target_os = "ios"))))]
                            if let Some(ref mut c) = media_controls {
                                let _ = c.set_volume(v);
                            }
                        }
                        MediaControlEvent::Quit => {
                            self.event_handler.send(AppEvent::Quit);
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> Result<()> {
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
                if self.show_station_selector
                    && let Some(event) = self.station_selector.handle_key_events(key_event)
                {
                    match event {
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
            }
        }
        Ok(())
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }

    fn handle_app_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Quit => self.quit(),
            AppEvent::NewStationUrl(station, url) => {
                self.station_urls.insert(station, url.clone());
                #[cfg(not(test))]
                if let Err(e) = cache::save(station, &url) {
                    info!("Failed to cache URL for {}: {}", station.name, e);
                }
                if self.station == station {
                    _ = self.cmd_tx.send(PlayerCommand::SetStation(url));
                }
            }
            AppEvent::StationUrlFailed(station) => {
                info!("Failed url lookup for {:?}", station);
                self.station_urls.remove(&station);
                #[cfg(not(test))]
                if let Err(e) = cache::invalidate(station) {
                    info!("Failed to invalidate cache for {}: {}", station.name, e);
                }
                self.player_state.connection_state =
                    crate::player::ConnectionState::UrlLookupFailure;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stations::ClassicalStations;

    fn test_url() -> Url {
        Url::parse("http://example.com/stream").unwrap()
    }

    /// Drain all commands currently queued on cmd_rx without running the event loop.
    fn drain_commands(app: &mut App) -> Vec<PlayerCommand> {
        let mut cmds = Vec::new();
        if let Some(rx) = &mut app.cmd_rx {
            while let Ok(cmd) = rx.try_recv() {
                cmds.push(cmd);
            }
        }
        cmds
    }

    #[tokio::test]
    async fn new_station_url_for_current_station_sends_command_and_caches() {
        let mut app = App::default();
        let station = app.station;
        let url = test_url();

        app.handle_app_event(AppEvent::NewStationUrl(station, url.clone()));

        assert_eq!(app.station_urls.get(&station), Some(&url));
        let cmds = drain_commands(&mut app);
        assert!(
            cmds.iter()
                .any(|c| matches!(c, PlayerCommand::SetStation(u) if *u == url)),
            "expected SetStation to be sent for the current station"
        );
    }

    #[tokio::test]
    async fn new_station_url_for_other_station_caches_but_no_command() {
        let mut app = App::default();
        let other = ClassicalStations::Ultimate.station();
        assert_ne!(app.station, other);
        let url = test_url();

        app.handle_app_event(AppEvent::NewStationUrl(other, url.clone()));

        assert_eq!(app.station_urls.get(&other), Some(&url));
        let cmds = drain_commands(&mut app);
        assert!(
            cmds.is_empty(),
            "expected no command to be sent for a non-current station"
        );
    }

    #[tokio::test]
    async fn change_station_sends_command_on_cache_hit() {
        let mut app = App::default();
        let station = ClassicalStations::Ultimate.station();
        let url = test_url();
        app.station_urls.insert(station, url.clone());

        app.change_station(station);

        assert_eq!(app.station, station);
        let cmds = drain_commands(&mut app);
        assert!(
            cmds.iter()
                .any(|c| matches!(c, PlayerCommand::SetStation(u) if *u == url)),
            "expected SetStation to be sent when URL is cached"
        );
    }

    #[tokio::test]
    async fn change_station_no_command_on_cache_miss() {
        let mut app = App::default();
        let station = ClassicalStations::Ultimate.station();

        app.change_station(station);

        assert_eq!(app.station, station);
        let cmds = drain_commands(&mut app);
        assert!(
            cmds.is_empty(),
            "expected no command when URL is not cached yet"
        );
    }

    #[tokio::test]
    async fn quit_sets_not_running() {
        let mut app = App::default();
        assert!(app.running);
        app.quit();
        assert!(!app.running);
    }
}
