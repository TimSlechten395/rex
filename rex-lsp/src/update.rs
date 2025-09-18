use anyhow::anyhow;
use anyhow::bail;
use chumsky::Parser;
use rex::{
    ErrorToken, SpannedResultSugarExpr, Token, desugar, extract_good_toks, get_normal_expr,
    parser::Spanned, remove_span, sea_nodes::lower_expr, to_indices,
};
use ropey::Rope;
use tokio::sync::mpsc::channel;
use tower_lsp_server::lsp_types::{Diagnostic, DiagnosticSeverity, MessageType, Uri};

use crate::{Backend, helper::span_to_range};

pub async fn update(backend: &Backend, uri: Uri, text: String) -> anyhow::Result<()> {
    backend
        .client
        .log_message(MessageType::INFO, "processing file")
        .await;

    let file = Rope::from_str(&text);

    let files = backend.files.clone();
    files.insert(uri.clone(), file);

    let sea = backend.sea_of_nodes.clone();
    let toks = backend.tokens.clone();
    let sugar_asts = backend.sugar_asts.clone();
    let core_asts = backend.core_asts.clone();

    let (diag_tx, mut diag_rx) = channel::<Vec<Diagnostic>>(10);

    let uri2 = uri.clone();

    let compiler = tokio::task::spawn_blocking(move || {
        let uri = uri2;
        let lexer = rex::lexer();

        let Ok(spanned_tokens) = lexer.parse(&text).into_result() else {
            bail!("Failed to lex file: {uri:?}")
        };

        toks.insert(uri.clone(), spanned_tokens.clone());

        let tokens = extract_good_toks(spanned_tokens.clone());
        let file = files.get(&uri).ok_or(anyhow::anyhow!("failed"))?;

        let err_tokens = spanned_tokens
            .clone()
            .into_iter()
            .filter_map(|(opt_tok, span)| Result::err(opt_tok.map_err(|t| (t, span))))
            .map(|(err_tok, span)| {
                let range = span_to_range(&file, span.into_range());
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("invalid token: {} {}", err_tok.char, err_tok.message),
                    source: Some("rex".to_string()),

                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        diag_tx.blocking_send(err_tokens)?;

        let parser = rex::parser();
        let Ok(sugar_ast) = parser.parse(&tokens).into_result() else {
            bail!("Failed to parse file: {uri:?}")
        };
        sugar_asts.insert(uri.clone(), sugar_ast.clone());

        let parse_diags = collect_parse_diags(&file, &spanned_tokens, sugar_ast.clone())?;

        diag_tx.blocking_send(parse_diags)?;

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
    });

    let diags = &mut backend.diagnostics.lock().await;
    diags.clear();
    backend
        .client
        .publish_diagnostics(uri.clone(), diags.clone(), None)
        .await;

    while let Some(message) = diag_rx.recv().await {
        diags.extend_from_slice(&message);
        backend
            .client
            .publish_diagnostics(uri.clone(), diags.clone(), None)
            .await;
    }

    compiler.await??;

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

pub fn collect_parse_diags(
    rope: &ropey::Rope,
    tokens: &Vec<Spanned<Result<Token, ErrorToken>>>,
    sugar_ast: SpannedResultSugarExpr,
) -> anyhow::Result<Vec<Diagnostic>> {
    let res = match sugar_ast.0.0 {
        Ok(valid) => valid.fold(Vec::new(), |mut acc, node| {
            acc.extend_from_slice(&collect_parse_diags(rope, tokens, *node).unwrap());
            acc
        }),
        Err(invalid) => {
            let span = sugar_ast.0.1.into_range();
            let range = char_span_from_sugar_ast_span(tokens, span)
                .map_err(|x| x.context("Span error: "))?;
            let range = span_to_range(rope, range);
            let diagnostic = Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("invalid ast node: {} ", invalid),
                source: Some("rex".to_string()),

                ..Default::default()
            };
            invalid.fold(vec![diagnostic], |mut acc, node| {
                acc.extend_from_slice(&collect_parse_diags(rope, tokens, *node).unwrap());
                acc
            })
        }
    };
    Ok(res)
}

pub fn char_span_from_sugar_ast_span(
    tokens: &Vec<Spanned<Result<Token, ErrorToken>>>,
    span: std::ops::Range<usize>,
) -> anyhow::Result<std::ops::Range<usize>> {
    let start = span.start;
    let end = span.end - 1;
    let good_toks = tokens
        .iter()
        .filter(|x| matches!(x.0, Ok(Token::RealToken(_))));

    let start_span = good_toks
        .clone()
        .nth(start)
        .ok_or_else(|| {
            anyhow!(
                "start out of bounds: start: {} good_toks: {}",
                start,
                good_toks.clone().count(),
            )
        })?
        .1
        .start;
    let end_span = good_toks
        .clone()
        .nth(end)
        .ok_or_else(|| {
            anyhow!(
                "end out of bounds: end: {} good_toks: {}",
                end,
                good_toks.clone().count(),
            )
        })?
        .1
        .end;
    Ok(start_span..end_span)
}
