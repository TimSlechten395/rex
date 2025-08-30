use tower_lsp_server::{
    jsonrpc,
    lsp_types::{Hover, HoverContents, HoverParams, MarkedString},
};

use crate::Backend;

// TODO: use anyhow error type instead of jsonrpc error
pub async fn hover(backend: &Backend, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let Some(text) = backend.files.get(&uri) else {
        return Err(jsonrpc::Error::internal_error());
    };

    let offset = text.line_to_char(pos.line as usize) + pos.character as usize;

    let Some(tokens) = backend.tokens.get(&uri) else {
        return Err(jsonrpc::Error::internal_error());
    };

    // TODO: Should we use start or end?
    let idx = match tokens.binary_search_by_key(&offset, |token| token.1.end) {
        Ok(idx) => idx,
        Err(idx) => idx,
    };

    let Some(token) = tokens.get(idx) else {
        return Err(jsonrpc::Error::internal_error());
    };

    // TODO: This is quite slow we might need to store an extra map or even store a list of
    // valid tokens?
    let message = match &token.0 {
        Ok(tok) => {
            let ast_idx = tokens
                .iter()
                .filter(|t| t.0.is_ok())
                .enumerate()
                .map(|(i, _)| i)
                .nth(idx);

            if let Some(ast_idx) = ast_idx {
                let Some(ast) = backend.tokens.get(&uri) else {
                    return Err(jsonrpc::Error::internal_error());
                };

                let node = ast.get(ast_idx);
                format!("Found token {:?} in ast: {:?}", tok, node)
            } else {
                format!("Could not find token {:?} in ast", tok)
            }
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
