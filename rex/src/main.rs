use anyhow::anyhow;
use rex::compile;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let path = args.nth(1).ok_or(anyhow!("file path argument required"))?;

    let expr = compile(&path, true)?;
    println!("{expr:?}");
    Ok(())
}
