use anyhow::Result;
use clap::Parser;
#[macro_use]
extern crate tracing;
mod app;
mod cache;
mod cli;
mod event;
mod player;
mod stations;
mod ui;

use app::App;
use cli::{AppConfig, log_init};

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::parse();
    log_init(&config.verbosity)?;

    if config.clear_cache {
        cache::clear()?;
        println!("Station URL cache cleared.");
        return Ok(());
    }

    debug!("Setting panic hook");
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
