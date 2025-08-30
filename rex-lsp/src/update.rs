use anyhow::bail;
use chumsky::Parser;
use rex::{Context, sea_nodes::lower_expr};
use tower_lsp_server::{
    jsonrpc,
    lsp_types::{MessageType, Uri},
};

use crate::Backend;

pub async fn update(backend: &Backend, uri: Uri, text: String) -> anyhow::Result<()> {
    let sea = backend.sea_of_nodes.clone();
    let toks = backend.tokens.clone();
    let sugar_asts = backend.sugar_asts.clone();
    let core_asts = backend.core_asts.clone();
    tokio::task::spawn_blocking(move || {
        // weird lifetime problems
        let mut tokens = Vec::new();
        let lexer = rex::lexer();
        let parser = rex::parser();

        if let Ok(spanned_tokens) = lexer.parse(&text).into_result() {
            toks.insert(uri.clone(), spanned_tokens.clone());
            tokens.extend(
                spanned_tokens
                    .into_iter()
                    .map(|t| t.0)
                    .filter_map(Result::ok),
            );

            if let Ok(sugar_ast) = parser.parse(&tokens).into_result() {
                sugar_asts.insert(uri.clone(), sugar_ast.clone());
                // let clean_ast = clean(sugar_ast);

                // let valid_subtrees = extract_valid_subtrees(clean_ast);
                //
                // for subtree in valid_subtrees {
                //     let mut ctx = Context::new();
                //     let ast_tree = rex::desugar(subtree, &mut ctx);
                //     let ast = lower_expr(&ast_tree, &mut sea.blocking_lock());
                //     core_asts
                //         .entry(uri.clone())
                //         .or_insert_with(Vec::new)
                //         .push(ast);
                // }
            }
            Ok(())
        } else {
            bail!("Failed to lex file: {uri:?}")
        }
    })
    .await?
}
