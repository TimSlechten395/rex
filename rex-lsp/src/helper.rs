use anyhow::bail;
use rex::data::{
    ast::SpannedFixAst,
    expr::NamedDefs,
    tokens::{self, GToken, Spanned, Token},
};
use ropey::Rope;
use tower_lsp_server::lsp_types::{Position, Range, Uri};

use crate::Backend;

pub fn char_to_pos(text: &ropey::Rope, char: usize) -> (usize, usize) {
    let line = text.char_to_line(char);
    let line_start = text.line_to_char(line);
    let column = char - line_start;
    (line, column)
}

pub fn span_to_range(text: &ropey::Rope, span: std::ops::Range<usize>) -> Range {
    let (start_line, start_char) = char_to_pos(text, span.start);
    let (end_line, end_char) = char_to_pos(text, span.end);
    Range {
        start: Position {
            line: start_line as u32,
            character: start_char as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_char as u32,
        },
    }
}

// map validToken index to token index

pub fn get_text(backend: &Backend, uri: &Uri) -> anyhow::Result<Rope> {
    let Some(text) = backend.files.get(&uri) else {
        bail!("Failed to get file");
    };
    Ok(text.clone())
}

pub fn get_tokens(backend: &Backend, uri: &Uri) -> anyhow::Result<Vec<tokens::Spanned<Token>>> {
    let Some(tokens) = backend.tokens.get(&uri) else {
        bail!("Failed to get file");
    };
    Ok(tokens.clone())
}

pub fn get_ast(backend: &Backend, uri: &Uri) -> anyhow::Result<Vec<SpannedFixAst>> {
    let Some(ast) = backend.asts.get(&uri) else {
        bail!("Failed to get ast");
    };
    Ok(ast.clone())
}

pub fn get_named_defs(backend: &Backend, uri: &Uri) -> anyhow::Result<NamedDefs> {
    let Some(expr) = backend.named_exprs.get(&uri) else {
        bail!("Failed to get expr");
    };
    Ok(expr.clone())
}
