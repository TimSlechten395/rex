use anyhow::{anyhow, bail};
use chumsky::Parser;
use rex::{
    desugar,
    eval::strong_normalize,
    lexer::ExpectedToken,
    rex_parser::{lexer::lexer, new_parser::parse},
    sea_nodes::{SeaOfNodes, lower_expr},
    to_indices,
    r#type::{TypeErrorWithLoc, err_with_nodes, infer_type},
};
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

    let good_toks = toks
        .into_iter()
        .filter_map(|x| {
            if let Ok(ExpectedToken::Token(token)) = x.0 {
                Some(token)
            } else {
                None
            }
        })
        .collect();

    println!("{good_toks:?}");

    let tree = parse(good_toks);

    println!("{tree:?}");
    Ok(())
}
