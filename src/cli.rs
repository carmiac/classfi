//! CLI Options and general application utilities.
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::Verbosity;
use etcetera::{AppStrategy, AppStrategyArgs, choose_app_strategy};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*};

// Useful directories.
pub fn dir_strategy() -> Result<etcetera::app_strategy::Xdg> {
    choose_app_strategy(AppStrategyArgs {
        top_level_domain: "org".to_string(),
        author: "carmiac".to_string(),
        app_name: env!("CARGO_PKG_NAME").to_string(),
    })
    .map_err(|e| anyhow::anyhow!("Couldn't determine XDG dirs: {e}"))
}

// Setup logging
pub fn log_init(verbosity: &Verbosity) -> Result<()> {
    let directory = dir_strategy()?.data_dir();
    std::fs::create_dir_all(&directory)?;
    let log_path = directory.join(env!("CARGO_PKG_NAME")).with_extension("log");
    let log_file = std::fs::File::create(log_path)?;
    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(verbosity.tracing_level_filter());
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
    /// Initial Station
    #[arg(short, long)]
    pub station: Option<crate::stations::ClassicalStations>,
    /// Color Theme Name
    #[arg(short, long)]
    pub theme: Option<String>,
    /// Compact single-line display
    #[arg(long)]
    pub compact: bool,
    /// Clear the station URL cache and exit
    #[arg(long)]
    pub clear_cache: bool,
    // Debug level
    #[command(flatten)]
    pub verbosity: Verbosity,
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
    let (config_dir_path, data_dir_path) = match dir_strategy() {
        Ok(dirs) => (
            dirs.config_dir().display().to_string(),
            dirs.data_dir().display().to_string(),
        ),
        Err(_) => ("<unknown>".to_string(), "<unknown>".to_string()),
    };
    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}

Config directory: {config_dir_path}
Data directory: {data_dir_path}
"
    )
}
