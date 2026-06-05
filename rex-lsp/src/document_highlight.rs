use rex::{
    data::{
        ast::{self, traverse_list},
        expr::{self, ExprF, traverse_defs},
        tokens::{result_tok_span_to_char_span, tok_span_to_result_tok_span},
    },
    helper::map_index,
};
use tower_lsp_server::lsp_types::{
    DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams, Position, Range,
};

use crate::{
    Backend,
    helper::{get_ast, get_named_defs, get_text, get_tokens, span_to_range},
};

pub async fn document_highlight(
    backend: &Backend,
    params: DocumentHighlightParams,
) -> anyhow::Result<Option<Vec<DocumentHighlight>>> {
    println!("try highlight");
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;

    let text = get_text(backend, &uri)?;
    let tokens = get_tokens(backend, &uri)?;
    let ast = get_ast(backend, &uri)?;
    let expr = get_named_defs(backend, &uri)?;

    let offset = text.line_to_char(pos.line as usize) + (pos.character + 1) as usize;

    // TODO: Should we use start or end? end is current winner
    let idx = match tokens.binary_search_by_key(&offset, |token| token.1.end) {
        Ok(idx) => idx,
        Err(idx) => idx,
    };

    // map the index from result tokens to all tokens
    let ast_idx = map_index(&*tokens, idx);

    let ast_path = ast_idx.and_then(|x| {
        ast::search_list(ast.clone(), x)
            .into_iter()
            .find(|x| !x.is_empty())
    });

    let ast_node = ast_path.clone().map(|x| traverse_list(ast.clone(), x));

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
        .map(|p| traverse_defs(expr.clone(), p))
        .transpose()?;

    let first = expr_node
        .clone()
        .map(|n| -> anyhow::Result<_> {
            let span = n.0.1;
            let span = ast::traverse_list(ast, span).unwrap().0.1;
            let span = tok_span_to_result_tok_span(span, &tokens)?;
            let span = result_tok_span_to_char_span(span, &tokens)?;
            let span = span_to_range(&text, span);
            Ok(span)
        })
        .transpose()?;

    let name = expr_node
        .map(|n| {
            if let ExprF::Var { idx } = n.remove_span().0 {
                match idx {
                    expr::VarKind::Named(n) => Some(n),
                    expr::VarKind::Idx(_) => None,
                }
            } else {
                None
            }
        })
        .flatten();

    let mut highlights = vec![];

    if let Some(first) = first {
        highlights.push(DocumentHighlight {
            range: first,
            kind: Some(DocumentHighlightKind::TEXT),
        })
    }

    Ok(Some(highlights))
}
