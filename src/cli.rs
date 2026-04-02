use clap::Parser;

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

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}
"
    )
}
