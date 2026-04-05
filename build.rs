use anyhow::Result;
use vergen_gix::{BuildBuilder, Emitter, GixBuilder};

fn main() -> Result<()> {
    let build = BuildBuilder::default().build_date(true).build()?;
    let gix = GixBuilder::default().describe(true, true, None).build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&gix)?
        .emit()
}
