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
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        default_hook(info);
    }));
    let terminal = ratatui::init();
    debug!("Starting app");
    let result = App::new(config).run(terminal).await;
    ratatui::restore();
    result
}
