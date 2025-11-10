use std::fs::read_to_string;

use anyhow::anyhow;
use rex::compile;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let path = args.nth(1).ok_or(anyhow!("file path argument required"))?;

    let file = read_to_string(path)?;

    let expr = compile(file)?;
    // println!("{expr:?}");
    Ok(())
}
