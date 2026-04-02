use clap::Parser;
use cli::AppConfig;

use crate::app::App;

mod app;
mod cli;
mod event;
mod player;
mod stations;
mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let config = AppConfig::parse();
    let terminal = ratatui::init();
    let result = App::new(config).run(terminal).await;
    ratatui::restore();
    result
}
