use anyhow::{Context as _, anyhow, bail};
use rex_parser::lexer::{ExpectedToken, result_tok_span_to_char_span, tok_span_to_result_tok_span};
use rex_parser::new_parser::parse;
use rex_parser::parser::{get_normal_expr, remove_span};
use std::fs::read_to_string;
use std::ops::Range;

use chumsky::Parser;
pub use rex_parser;
pub use rex_parser::*;

pub use rex_core;
pub use rex_core::*;

pub mod desugar;
pub use desugar::*;

use crate::r#type::infer_type;

pub mod eval;

pub mod experimental;

pub mod r#type;

pub mod repl;

pub mod autoconvert;
pub mod context;

pub mod sea_nodes;

pub mod lower;
pub fn compile(path: &str, self_rec: bool) -> anyhow::Result<Expr> {
    let path = path;
    let file = read_to_string(path)?;

    let lexer = lexer::lexer();

    let toks = lexer
        .parse(&file)
        .into_result()
        .map_err(|e| anyhow!("failed to parse file: {:?}", e))?
        .into_iter()
        .map(|(tok, s)| (tok, s.into_range()))
        .collect::<Vec<_>>();

    let good_toks = toks
        .clone()
        .into_iter()
        .filter_map(|x| {
            if let Ok(ExpectedToken::Token(token)) = x.0 {
                Some(token)
            } else {
                None
            }
        })
        .collect();

    let spanned_result_sugar_expr = parse(good_toks, self_rec);

    let sugar_expr = get_normal_expr(remove_span(spanned_result_sugar_expr.clone()))?;

    let spanned_named_expr = desugar(sugar_expr, Vec::new()).map_err(|err| {
        let err: ExprError<Result<_, anyhow::Error>> = err.fmap(|x| {
            let span = x.1;

            let node = anyhow::Context::context(
                spanned_result_sugar_expr
                    .clone()
                    .traverse(span.clone().into_iter()),
                format!("span: {span:?}"),
            )?;
            let span = node.0.1;

            let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
            let span = result_tok_span_to_char_span(span.clone(), &toks)?;

            Ok(range_to_line_offset_range(span, &file)?)
        });
        err
    })?;

    let named_expr = remove_span_expr(spanned_named_expr.clone());

    let tree = match to_indices(named_expr.clone()) {
        Ok(tree) => tree,
        Err(err) => {
            let err: ResolveError<Result<_, anyhow::Error>> = err.fmap(|x| {
                let span = x.0;

                let span = spanned_named_expr
                    .clone()
                    .traverse(span.clone().into_iter())
                    .context(anyhow!(
                        "failed to resolve path in named_expr. path: {span:?}"
                    ))?
                    .0
                    .1;

                let span = spanned_result_sugar_expr
                    .clone()
                    .traverse(span.clone().into_iter())
                    .context(anyhow!(
                        "failed to resolve path in sugar_expr. path: {span:?}"
                    ))?
                    .0
                    .1;

                let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
                let span = result_tok_span_to_char_span(span.clone(), &toks)?;
                Ok((range_to_line_offset_range(span, &file)?, x.1))
            });
            panic!("failed to resolve at {err:?}")
        }
    };

    // now we get a module and need to do resolution
    let expr: Expr = if self_rec {
        let ycomb = compile("Y.rx", false).context("Y.rx: ")?;
        GExpr(ExprF::App {
            func: Box::new(ycomb),
            arg: Box::new(tree),
        })
    } else {
        tree
    };

    infer_type(expr.clone(), &mut Vec::new(), &mut Vec::new(), Vec::new())?.v_err();

    Ok(expr)
}

pub fn range_to_line_offset_range(
    range: Range<usize>,
    content: &str,
) -> anyhow::Result<Range<(usize, usize)>> {
    Ok(Range {
        start: char_index_to_line_offset(content, range.start)?,
        end: char_index_to_line_offset(content, range.end)?,
    })
}

pub fn char_index_to_line_offset(s: &str, index: usize) -> anyhow::Result<(usize, usize)> {
    let mut current_index = 0;
    let lines: Vec<&str> = s.split_inclusive('\n').collect(); // keep newline if present

    for (line_number, line) in lines.iter().enumerate() {
        let line_len = line.len(); // includes newline if present
        if index < current_index + line_len {
            let offset = index - current_index;
            return Ok((line_number + 1, offset + 1));
        }
        current_index += line_len;
    }

    bail!(
        "Index {} out of bounds for string of length {}",
        index,
        s.len()
    )
}

fn format_parentheses(input: &str) -> String {
    let mut result = String::new();
    let mut indent_level = 0;

    for c in input.chars() {
        match c {
            '(' => {
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str(&"  ".repeat(indent_level)); // indent
                result.push('(');
                indent_level += 1;
            }
            ')' => {
                indent_level -= 1;
                if !result.ends_with('\n') {
                    result.push('\n');
                }
                result.push_str(&"  ".repeat(indent_level));
                result.push(')');
            }
            _ => {
                result.push(c);
            }
        }
    }

    result
}
