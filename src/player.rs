//! Async streaming radio player using libmvp2.
use anyhow::{Error, Result, anyhow};
use libmpv2::{
    Format, Mpv,
    events::{Event, PropertyData},
};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};
use url::Url;

// Map mpv errors to anyhow_eyre errors to deal with threading issues.
fn mpv_err(e: libmpv2::Error) -> Error {
    anyhow!("mpv error: {e}")
}

#[derive(Debug, Default, Clone)]
pub struct PlayerState {
    pub volume: i64,
    pub play_time: f64,
    pub title: String,
    pub cache: i64,
    pub connection_state: ConnectionState,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    // Connecting,
    Buffering,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    SetStation(Url),
    // Play,
    // Pause,
    Toggle,
    SetVolume(i64),
    VolumeUp,
    VolumeDown,
}

//#[derive(Debug)]
pub struct Player {
    /// Current state for reporting.
    state: PlayerState,
    /// Transmitter for the player state on update.
    state_tx: mpsc::UnboundedSender<PlayerState>,
    /// Command channel to allow calling commands from different threads.
    cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
    /// The mpv player
    mpv: Mpv,
}

impl Player {
    pub fn new(
        state_tx: mpsc::UnboundedSender<PlayerState>,
        cmd_rx: mpsc::UnboundedReceiver<PlayerCommand>,
    ) -> Self {
        let mpv = Mpv::with_initializer(|init| {
            init.set_property("video", "no")?;
            init.set_property("volume", 80i64)?;
            init.set_property("idle", "once")?;
            init.set_property("terminal", "no")?;
            init.set_property("input-terminal", "no")?;
            init.set_property("input-vo-keyboard", "no")?;
            Ok(())
        })
        .expect("Could not create mpv. Is libmpv2 installed?");

        Player {
            state: PlayerState::default(),
            state_tx,
            cmd_rx,
            mpv,
        }
    }

    fn setup_mpv(&self) -> Result<()> {
        // Set events and properties to watch
        self.mpv.disable_deprecated_events().map_err(mpv_err)?;
        self.mpv
            .observe_property("volume", Format::Int64, 0)
            .map_err(mpv_err)?;
        self.mpv
            .observe_property("time-pos", Format::Double, 0)
            .map_err(mpv_err)?;
        self.mpv
            .observe_property("cache-buffering-state", Format::Int64, 0)
            .map_err(mpv_err)?;
        self.mpv
            .observe_property("media-title", Format::String, 0)
            .map_err(mpv_err)?;
        self.mpv
            .observe_property("paused-for-cache", Format::Flag, 0)
            .map_err(mpv_err)?;
        self.mpv
            .observe_property("pause", Format::Flag, 0)
            .map_err(mpv_err)?;
        self.mpv.enable_all_events().map_err(mpv_err)?;
        Ok(())
    }

    pub async fn run(mut self) -> Result<()> {
        // Setup mpv player.
        self.setup_mpv()?;

        // Process Commands and MPV updates.
        let mut timeout_interval = tokio::time::interval_at(
            Instant::now() + Duration::from_secs(10),
            Duration::from_secs(10),
        );
        let mut property_interval = tokio::time::interval(Duration::from_millis(1000 / 4));
        loop {
            tokio::select! {
                    cmd = self.cmd_rx.recv() => match cmd {
                        Some(cmd) => self.handle_cmd(cmd)?,
                        None => break,
                    },
                    _ = property_interval.tick() => {
                        let num_events = self.process_mpv_events()?;
                        if num_events > 0 {
                            // We got a good message, so reset the timeout and send an update.
                            timeout_interval.reset();
                            self.state_tx.send(self.state.clone())?;
                        }
                    },
                    _ = timeout_interval.tick() => {
                        // Mvp timed out, must have crashed or something.
                        return Err(anyhow!("Mvp timeout."));
                    }
            }
        }
        Ok(())
    }

    fn process_mpv_events(&mut self) -> Result<usize> {
        let mut msg_count = 0;
        while let Some(event) = self.mpv.wait_event(0.0) {
            msg_count += 1;
            match event {
                Err(err) => {
                    error!("Event error {}", err);
                    return Err(mpv_err(err));
                }
                Ok(Event::PropertyChange {
                    name: "time-pos",
                    change: PropertyData::Double(value),
                    ..
                }) => {
                    self.state.play_time = value;
                }
                Ok(Event::PropertyChange {
                    name: "volume",
                    change: PropertyData::Int64(value),
                    ..
                }) => self.state.volume = value,

                Ok(Event::PropertyChange {
                    name: "cache-buffering-state",
                    change: PropertyData::Int64(value),
                    ..
                }) => self.state.cache = value,
                Ok(Event::PropertyChange {
                    name: "media-title",
                    change: PropertyData::Str(value),
                    ..
                }) => self.state.title = value.into(),
                Ok(Event::PropertyChange {
                    name: "pause",
                    change: PropertyData::Flag(value),
                    ..
                }) => {
                    if value {
                        self.state.connection_state = ConnectionState::Paused
                    } else {
                        self.state.connection_state = ConnectionState::Playing
                    }
                }
                Ok(Event::PropertyChange {
                    name: "paused-for-cache",
                    change: PropertyData::Flag(value),
                    ..
                }) => {
                    if value {
                        self.state.connection_state = ConnectionState::Buffering
                    }
                }

                // check for lost stream
                _ => {
                    info!("Unhandled event: {:?}", event)
                }
            }
        }
        Ok(msg_count)
    }

    /// Process commands.
    fn handle_cmd(&mut self, cmd: PlayerCommand) -> Result<()> {
        debug!("Player Command: {:?}", cmd);
        match cmd {
            PlayerCommand::SetStation(url) => {
                self.mpv
                    .command("loadfile", &[url.as_str(), "replace"])
                    .map_err(mpv_err)?;
                Ok(())
            }

            PlayerCommand::Toggle => {
                let connection = self.state.connection_state;
                let pause = match connection {
                    ConnectionState::Disconnected => false,
                    // ConnectionState::Connecting => true,
                    ConnectionState::Buffering => true,
                    ConnectionState::Playing => true,
                    ConnectionState::Paused => false,
                };
                self.mpv.set_property("pause", pause).map_err(mpv_err)
            }

            PlayerCommand::SetVolume(volume) => {
                self.mpv.set_property("volume", volume).map_err(mpv_err)
            }
            PlayerCommand::VolumeUp => {
                let volume = self.state.volume;
                self.mpv
                    .set_property("volume", (volume + 5).min(100))
                    .map_err(mpv_err)
            }

            PlayerCommand::VolumeDown => {
                let volume = self.state.volume;
                self.mpv
                    .set_property("volume", (volume - 5).max(0))
                    .map_err(mpv_err)
            }
        }
    }
}
