//! CLI Options and general application utilities.
use clap::Parser;
use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};
use tracing_error::ErrorLayer;
use tracing_subscriber::{filter, fmt, prelude::*};

// Useful directories.
pub fn dir_strategy() -> etcetera::app_strategy::Xdg {
    choose_app_strategy(AppStrategyArgs {
        top_level_domain: "org".to_string(),
        author: "carmiac".to_string(),
        app_name: env!("CARGO_PKG_NAME").to_string(),
    })
    .unwrap()
}

// Setup logging
pub fn log_init() -> color_eyre::Result<()> {
    let directory = dir_strategy().data_dir();
    std::fs::create_dir_all(&directory)?;
    let log_path = directory.join(env!("CARGO_PKG_NAME"));
    let log_file = std::fs::File::create(log_path)?;
    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(filter::LevelFilter::DEBUG);
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;
    Ok(())
}

// CLI parsing
#[derive(Parser, Debug, Default)]
#[command(author, version = version(), about)]
pub struct AppConfig {
    /// Station Index
    #[arg(short, long, default_value_t = 0)]
    pub station: usize,
    /// Color Theme Name
    #[arg(short, long)]
    pub theme: Option<String>,
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("VERGEN_GIT_DESCRIBE"),
    " (",
    env!("VERGEN_BUILD_DATE"),
    ")"
);

pub fn version() -> String {
    let author = clap::crate_authors!();
    let config_dir_path = dir_strategy().config_dir().display().to_string();
    let data_dir_path = dir_strategy().data_dir().display().to_string();
    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}
"
    )
}
