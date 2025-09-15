use anyhow::bail;
use rex::{desugar, find_node, get_normal_expr, remove_span, sea_nodes::lower_expr};
use tower_lsp_server::{
    jsonrpc,
    lsp_types::{Hover, HoverContents, HoverParams, MarkedString},
};

use crate::Backend;

// TODO: use anyhow error type instead of jsonrpc error
pub async fn hover(backend: &Backend, params: HoverParams) -> anyhow::Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let Some(text) = backend.files.get(&uri) else {
        bail!("Failed to get file");
    };

    // pos.chararacter is ZERO based and editor is ONE based
    let offset = text.line_to_char(pos.line as usize) + (pos.character + 1) as usize;

    let Some(tokens) = backend.tokens.get(&uri) else {
        bail!("Failed to get tokens");
    };

    // TODO: Should we use start or end? end is current winner
    let idx = match tokens.binary_search_by_key(&offset, |token| token.1.end) {
        Ok(idx) => idx,
        Err(idx) => idx,
    };

    let Some(token) = tokens.get(idx) else {
        bail!("Token was not in tokenstream");
    };

    // TODO: This is quite slow we might need to store an extra map or even store a list of
    // valid tokens?
    let message =
        match &token.0 {
            Ok(tok) => {
                let ast_idx = tokens.iter().take(idx).fold(0, |count, value| {
                    if value.0.is_ok() { count + 1 } else { count }
                });
                let Some(ast) = backend.sugar_asts.get(&uri) else {
                    bail!("Failed to get ast. Got token: {:?}", tok)
                };

                let Some(normal_ast) = get_normal_expr(remove_span(ast.clone())) else {
                    bail!("invalid ast: {:?}", ast)
                };

                let node = find_node(ast.clone(), ast_idx);

                format!(
                    "Found token {:?} with idx: {:?} in ast: {:?}",
                    tok, ast_idx, node
                )
            }
            Err(tok) => {
                format!("Found illegal token: {:?}", tok)
            }
        };

    Ok(Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(message)),
        range: None,
    }))
}
