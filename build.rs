use anyhow::Result;
use vergen_gix::{BuildBuilder, Emitter, GixBuilder};

fn main() -> Result<()> {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        println!("cargo:rustc-link-search=native=/opt/homebrew/include");
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib/pkgconfig");
    }
    let build = BuildBuilder::default().build_date(true).build()?;
    let gix = GixBuilder::default().describe(true, true, None).build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()
}
