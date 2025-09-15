use anyhow::{anyhow, bail};
use chumsky::Parser;
use rex::{
    desugar,
    eval::strong_normalize,
    get_normal_expr,
    lexer::lexer,
    parser::parser,
    remove_span,
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

    let good_toks: Vec<_> = toks.into_iter().filter_map(|x| Result::ok(x.0)).collect();
    println!(
        "got tokens: {:#?}",
        good_toks.iter().enumerate().collect::<Vec<_>>()
    );

    let parser = parser();
    let sugar_ast = parser
        .parse(&good_toks)
        .into_result()
        .map_err(|e| anyhow!("Failed to parse tokenstream: {:?}", e))?;
    // let ast = clean(result);
    println!("got ast: {:#?}", &sugar_ast);

    let Some(normal_ast) = get_normal_expr(remove_span(sugar_ast.clone())) else {
        bail!("invalid ast: {:?}", sugar_ast)
    };
    let Some(desugared) = desugar(normal_ast, Vec::new()) else {
        bail!("Failed to lower ast")
    };

    println!("got desugared ast: {:#?}", &desugared.clone().remove_span());

    let name_resolved = to_indices(desugared.clone().remove_span()).map_err(|e| {
        e.fmap(|e| {
            let n = desugared
                .clone()
                .traverse(e.clone())
                .map_err(|err| err.context(format!("path: {e:?}")))?;
            let path = n.0.1;
            let ast_node = sugar_ast
                .clone()
                .traverse(path.clone())
                .map_err(|err| err.context(format!("path: {path:?}")))?;
            let tokens = ast_node.0.1;
            Ok::<_, anyhow::Error>(tokens)
        })
    })?;

    println!("got final ast: {:#?}", &name_resolved);

    let mut sea = SeaOfNodes::new();

    let id = lower_expr(&name_resolved, &mut sea);

    let mut ty_errors = Vec::new();
    let loc = Vec::new();
    let ty = infer_type(id, &mut sea, &mut Vec::new(), &mut ty_errors, loc).map_err(|e| {
        TypeErrorWithLoc {
            error: err_with_nodes(e.error, &sea).unwrap(),
            loc: e.loc,
        }
    })?;

    let ty_errors = ty_errors
        .into_iter()
        .map(|e| TypeErrorWithLoc {
            error: err_with_nodes(e.error, &sea).unwrap(),
            loc: e.loc,
        })
        .collect::<Vec<_>>();

    let ty_node = sea.get_tree(ty);

    println!("got type: {ty_node:#?}");

    let ty_norm = strong_normalize(ty, &mut sea);

    let ty_norm_node = sea.get_tree(ty_norm);

    println!("got type: {ty_norm_node:#?}");

    println!("got type errors: {ty_errors:#?}");

    Ok(())
}
