use anyhow::anyhow;
use anyhow::bail;
use rex::Compile;
use rex::data::ast::Ast;
use rex::data::ast::SpannedFixAst;
use rex::data::tokens;
use rex::data::tokens::GToken;
use rex::data::tokens::extract_good_toks;
use rex::pipeline::desugar::Desugar;
use rex::pipeline::lexer::Lexer;
use rex::pipeline::name_resolver::NameResolver;
use rex::pipeline::parser::Parser;
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

    // let sea = backend.sea_of_nodes.clone();
    let toks = backend.tokens.clone();
    let asts = backend.asts.clone();
    let named_exprs = backend.named_exprs.clone();
    let exprs = backend.exprs.clone();

    let (diag_tx, mut diag_rx) = channel::<Vec<Diagnostic>>(10);

    let uri2 = uri.clone();

    let compiler = tokio::task::spawn_blocking(move || {
        let uri = uri2;

        let Ok(spanned_tokens) = Lexer::run(text) else {
            bail!("Failed to lex file: {uri:?}")
        };

        toks.insert(uri.clone(), spanned_tokens.clone());

        let tokens =
            extract_good_toks(spanned_tokens.clone().into_iter().map(|(x, _)| x).collect());
        let file = files.get(&uri).ok_or(anyhow::anyhow!("failed"))?;

        let err_tokens = spanned_tokens
            .clone()
            .into_iter()
            .filter_map(|(tok, span)| match tok {
                rex::data::tokens::GToken::ErrorToken(error_token) => Some((error_token, span)),
                _ => None,
            })
            .map(|(err_tok, span)| {
                let range = span_to_range(&file, span.into_range());

                let error_message = match err_tok {
                    rex::data::tokens::ErrorToken::InvalidChar(c) => {
                        format!("invalid token: {}", c)
                    }
                    rex::data::tokens::ErrorToken::MisplacedTabs(tabs) => {
                        format!("tabs not at the beginning of the line count: {} ", tabs)
                    }
                };
                Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: error_message,
                    source: Some("rex".to_string()),

                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();

        diag_tx.blocking_send(err_tokens)?;

        let Ok(ast) = Parser::run(tokens) else {
            bail!("Failed to parse file: {uri:?}")
        };
        asts.insert(uri.clone(), ast.clone());

        // let parse_diags = collect_parse_diags(&file, &spanned_tokens, sugar_ast.clone())?;

        // diag_tx.blocking_send(parse_diags)?;

        let Ok(expr) = Desugar::run(ast.into_iter().map(|x| x.remove_span()).collect()) else {
            bail!("Failed to lower ast")
        };
        named_exprs.insert(uri.clone(), expr.clone());

        let name_resolved = NameResolver::run(expr)?;
        exprs.insert(uri.clone(), name_resolved.clone());

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

pub fn collect_parse_diags<T>(
    rope: &ropey::Rope,
    tokens: &Vec<tokens::Spanned<GToken<T>>>,
    ast: SpannedFixAst,
) -> anyhow::Result<Vec<Diagnostic>> {
    let diags = if let Ast::Error(e, _) = ast.0.0.clone() {
        let span = ast.0.1;
        let range = char_span_from_ast_span(tokens, span).map_err(|x| x.context("Span error: "))?;
        let range = span_to_range(rope, range);
        let diagnostic = Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            message: format!("invalid ast node: {} ", e),
            source: Some("rex".to_string()),
            ..Default::default()
        };
        vec![diagnostic]
    } else {
        vec![]
    };

    let res = ast.0.0.fold(diags, |mut acc, node| {
        acc.extend_from_slice(&collect_parse_diags(rope, tokens, node).unwrap());
        acc
    });
    Ok(res)
}

pub fn char_span_from_ast_span<T>(
    tokens: &Vec<tokens::Spanned<GToken<T>>>,
    span: std::ops::RangeInclusive<usize>,
) -> anyhow::Result<std::ops::Range<usize>> {
    let start = *span.start();
    let end = *span.end() - 1;
    let good_toks = tokens
        .iter()
        .filter(|x| matches!(x.0, GToken::ValidToken(_)));

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
