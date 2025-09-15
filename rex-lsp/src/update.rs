use anyhow::bail;
use chumsky::Parser;
use rex::{Context, desugar, get_normal_expr, remove_span, sea_nodes::lower_expr, to_indices};
use tower_lsp_server::{
    jsonrpc,
    lsp_types::{MessageType, Uri},
};

use crate::Backend;

pub async fn update(backend: &Backend, uri: Uri, text: String) -> anyhow::Result<()> {
    backend
        .client
        .log_message(MessageType::INFO, "processing file")
        .await;
    let sea = backend.sea_of_nodes.clone();
    let toks = backend.tokens.clone();
    let sugar_asts = backend.sugar_asts.clone();
    let core_asts = backend.core_asts.clone();
    tokio::task::spawn_blocking(move || {
        // weird lifetime problems
        let mut tokens = Vec::new();
        let lexer = rex::lexer();
        let parser = rex::parser();

        let Ok(spanned_tokens) = lexer.parse(&text).into_result() else {
            bail!("Failed to lex file: {uri:?}")
        };

        toks.insert(uri.clone(), spanned_tokens.clone());
        tokens.extend(
            spanned_tokens
                .into_iter()
                .map(|t| t.0)
                .filter_map(Result::ok),
        );

        let Ok(sugar_ast) = parser.parse(&tokens).into_result() else {
            bail!("Failed to parse file: {uri:?}")
        };
        sugar_asts.insert(uri.clone(), sugar_ast.clone());

        let Some(normal_ast) = get_normal_expr(remove_span(sugar_ast.clone())) else {
            bail!("invalid ast: {:?}", sugar_ast)
        };
        let loc = Vec::new();
        let Some(desugared) = desugar(normal_ast, loc) else {
            bail!("Failed to lower ast")
        };

        let name_resolved = to_indices(desugared.remove_span())?;

        let id = lower_expr(&name_resolved, &mut *sea.blocking_lock());
        core_asts.insert(uri.clone(), vec![id]);

        Ok(())
    })
    .await??;

    backend
        .client
        .log_message(MessageType::INFO, "succesfully compiled whole file")
        .await;

    backend
        .client
        .log_message(MessageType::INFO, "done processing file")
        .await;
    Ok(())
}
