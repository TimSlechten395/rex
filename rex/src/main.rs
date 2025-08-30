use anyhow::anyhow;
use chumsky::Parser;
use rex::{lexer, parser};
use std::fs::read_to_string;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args();
    let path = args.nth(1).ok_or(anyhow!("file path argument required"))?;
    let file = read_to_string(path)?;

    let lexer = lexer();

    let toks = lexer
        .parse(&file)
        .into_result()
        .map_err(|e| anyhow!("failed to parse file: {:?}", e))?;

    let good_toks: Vec<_> = toks.into_iter().filter_map(|x| Result::ok(x.0)).collect();
    println!(
        "got tokens: {:?}",
        good_toks.iter().enumerate().collect::<Vec<_>>()
    );

    let parser = parser();
    let ast = parser
        .parse(&good_toks)
        .into_result()
        .map_err(|e| anyhow!("Failed to parse tokenstream: {:?}", e))?;
    // let ast = clean(result);
    println!("got ast: {:?}", &ast);

    Ok(())
}
