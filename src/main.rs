use clap::Parser;
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
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    log_init()?;

    debug!("Loading config");
    let config = AppConfig::parse();
    debug!("Creating terminal");
    let terminal = ratatui::init();
    debug!("Starting app");
    let result = App::new(config).run(terminal).await;
    ratatui::restore();
    result
}
