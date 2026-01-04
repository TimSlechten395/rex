use std::fs::read_to_string;

use anyhow::anyhow;
use rex::compile;

// HACK: this is set up for bootstrapping
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let code = args.nth(1).ok_or(anyhow!("code argument required"))?;

    // let file = read_to_string(path)?;

    let expr = compile(code)?;
    // println!("{expr:?}");
    Ok(())
}
