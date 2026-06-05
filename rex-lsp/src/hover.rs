use anyhow::bail;
use rex::{
    Traverse,
    data::{ast, expr},
    helper::map_index,
};
use tower_lsp_server::lsp_types::{Hover, HoverContents, HoverParams, MarkedString};

use crate::{Backend, semantic_tokens::semantic_token};

// TODO: use anyhow error type instead of jsonrpc error
pub async fn hover(backend: &Backend, params: HoverParams) -> anyhow::Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let Some(text) = backend.files.get(&uri) else {
        bail!("Failed to get file");
    };

    let Some(ast) = backend.asts.get(&uri) else {
        bail!("Failed to get ast");
    };

    let Some(expr) = backend.named_exprs.get(&uri) else {
        bail!("Failed to get expr");
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

    // map the index from result tokens to all tokens
    let ast_idx = map_index(&*tokens, idx);

    let ast_path = ast_idx.and_then(|x| {
        ast::search_list(ast.clone(), x)
            .into_iter()
            .find(|x| !x.is_empty())
    });

    let ast_node = ast_path.clone().map(|x| ast::traverse_list(ast.clone(), x));

    let expr_path = ast_path
        .clone()
        .map(|p| {
            expr::search_defs(expr.clone(), p)
                .into_iter()
                .max_by_key(|v| v.len())
        })
        .flatten();

    let expr_node = expr_path
        .clone()
        .map(|p| expr::traverse_defs(expr.clone(), p));

    let message = format!(
        "Found token {:?} with idx: {:?} in ast: {:?} (path: {:?}), \n  in expr: {:?} (path: {:?}), \n Semantics: {:?}",
        token,
        ast_idx,
        ast_node,
        ast_path,
        expr_node,
        expr_path,
        semantic_token(ast.clone(), expr.clone(), token.clone(), ast_idx)
    );

    Ok(Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(message)),
        range: None,
    }))
}
