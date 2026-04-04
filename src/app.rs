use crate::cli::AppConfig;
use crate::event::{AppEvent, Event, EventHandler};
use crate::player::{Player, PlayerCommand, PlayerState};
use crate::stations;
use crate::ui::UiStyles;

use color_eyre::eyre::{OptionExt, eyre};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use futures::FutureExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

/// Application.
pub struct App {
    running: bool,
    event_handler: EventHandler,
    state_tx: mpsc::UnboundedSender<PlayerState>,
    state_rx: mpsc::UnboundedReceiver<PlayerState>,
    cmd_tx: mpsc::UnboundedSender<PlayerCommand>,
    cmd_rx: Option<mpsc::UnboundedReceiver<PlayerCommand>>,
    pub(crate) station: Option<stations::Station>,
    pub(crate) styles: crate::ui::UiStyles,
    pub(crate) player_state: PlayerState,
}

impl Default for App {
    fn default() -> Self {
        let (state_tx, state_rx) = mpsc::unbounded_channel();
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        Self {
            running: true,
            event_handler: EventHandler::new(),
            state_tx,
            state_rx,
            cmd_tx,
            cmd_rx: Some(cmd_rx),
            station: None,
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

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        // Get the station list, pick the configured station, statrt looking up its url.
        // TODO: Show ui during this with messages.
        // TODO: get_url timeout
        let stations = stations::Station::all();
        let mut station = stations[0].clone();
        self.station = Some(station.clone());
        let (url_tx, url_rx) = tokio::sync::oneshot::channel();
        let mut url_rx = url_rx.fuse();
        tokio::spawn(async move {
            let _ = url_tx.send(station.get_url().await);
        });

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
                url_result = &mut url_rx => {
                    if let Ok(Some(url)) = url_result {
                        self.cmd_tx.send(PlayerCommand::SetStation(url))?;
                    }
                }
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
                            AppEvent::Quit => self.quit(),
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
            KeyCode::Esc | KeyCode::Char('q') => self.event_handler.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.event_handler.send(AppEvent::Quit)
            }
            KeyCode::Char('p') => self.cmd_tx.send(PlayerCommand::Toggle)?,
            KeyCode::Char('-' | '_') => self.cmd_tx.send(PlayerCommand::VolumeDown)?,
            KeyCode::Char('+' | '=') => self.cmd_tx.send(PlayerCommand::VolumeUp)?,
            KeyCode::Char('s') => todo!("Station select popup"),
            // Other handlers you could add here.
            _ => {}
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
