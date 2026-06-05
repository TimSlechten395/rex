use anyhow::bail;
use rex::tools::printer::print_named_expr;
use tower_lsp_server::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
};

use crate::Backend;

pub async fn completion(
    backend: &Backend,
    params: CompletionParams,
) -> anyhow::Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    // let Some(text) = backend.files.get(&uri) else {
    //     bail!("Failed to get file");
    // };
    //
    // let Some(ast) = backend.asts.get(&uri) else {
    //     bail!("Failed to get ast");
    // };

    let Some(expr) = backend.named_exprs.get(&uri) else {
        bail!("Failed to get expr");
    };

    let items = expr
        .0
        .clone()
        .into_iter()
        .map(|x| {
            let name = x.name;
            let ty = x.ty.map(|x| print_named_expr(x.remove_span()));

            CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::VARIABLE),
                detail: ty,
                documentation: None,
                ..Default::default()
            }
        })
        .collect();

    Ok(Some(CompletionResponse::Array(items)))
}
