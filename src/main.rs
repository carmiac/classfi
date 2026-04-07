use clap::Parser;
use anyhow::Result;
#[macro_use]
extern crate tracing;
mod app;
mod cli;
mod event;
mod player;
mod stations;
mod ui;

use app::App;
use cli::{AppConfig, log_init};

#[tokio::main]
async fn main() -> Result<()> {
    log_init()?;

    debug!("Loading config");
    let config = AppConfig::parse();
    debug!("Creating terminal");
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        default_hook(info);
    }));
    debug!("Creating app");
    let app = App::new(config);
    debug!("Starting ratatui");
    let terminal = ratatui::init();
    debug!("Running app");
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
