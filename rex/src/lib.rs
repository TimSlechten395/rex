use anyhow::{Context as _, anyhow, bail};
use std::fs::read_to_string;
use std::ops::Range;

use crate::data::expr::{Expr, ExprError, ExprF, GExpr, NamedExpr, remove_span_expr};
use crate::data::tokens::{
    ExpectedToken, result_tok_span_to_char_span, tok_span_to_result_tok_span,
};
use crate::pipeline::desugar::Desugar;
use crate::pipeline::lexer::Lexer;

use crate::pipeline::name_resolver::{NameResolver, ResolveError};
use crate::pipeline::parser::Parser;
use crate::r#type::{TypeError, infer_type};

pub mod data;
pub mod pipeline;

pub mod bootstrap;
pub mod eval;

pub mod helper;
pub mod r#type;

pub mod cache;
pub mod experimental;

pub mod tools;

pub trait Traverse {
    type Span;
    fn traverse(self, span: Self::Span) -> anyhow::Result<Box<Self>>;
}

pub trait CompileError<S> {
    fn span(&self) -> S;
}

pub type Spanned<T, S> = (T, S);

// this represents a compiler pass
pub trait Compile {
    type Input;
    type Output;
    type Error;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub fn compile(
    file: String,
) -> anyhow::Result<(
    Expr,
    Vec<TypeError<anyhow::Result<Range<(usize, usize)>, anyhow::Error>>>,
)> {
    let toks = Lexer::run(file.clone())?;

    let good_toks = toks
        .clone()
        .into_iter()
        .filter_map(|x| {
            if let ExpectedToken::Token(token) = x.0 {
                Some(token)
            } else {
                None
            }
        })
        .collect();

    let spanned_ast = Parser::run(good_toks)?;

    let ast = spanned_ast.clone().remove_span();

    let spanned_named_expr = Desugar::run(ast).map_err(|err| {
        let err: ExprError<Result<_, anyhow::Error>> = err.fmap(|span| {
            let node = anyhow::Context::context(
                spanned_ast.clone().traverse(span.clone()),
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

    let expr = match NameResolver::run(named_expr.clone()) {
        Ok(tree) => Ok(tree),
        Err(err) => {
            let err: ResolveError<Result<_, anyhow::Error>> = err.fmap(|x| {
                let span = x.0;

                let span = spanned_named_expr
                    .clone()
                    .traverse(span.clone())
                    .context(anyhow!(
                        "failed to resolve path in named_expr. path: {span:?}"
                    ))?
                    .0
                    .1;

                let span = spanned_ast
                    .clone()
                    .traverse(span.clone())
                    .context(anyhow!(
                        "failed to resolve path in sugar_expr. path: {span:?}"
                    ))?
                    .0
                    .1;

                let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
                let span = result_tok_span_to_char_span(span.clone(), &toks)?;
                Ok((range_to_line_offset_range(span, &file)?, x.1))
            });
            Err(anyhow!("failed to name resolve at {err:?}"))
        }
    }?;

    // here we should resolve modules but single module for now

    // TODO: fix
    let mut ty_errors = Vec::new();

    match infer_type(expr.clone(), Vec::new(), &mut ty_errors, Vec::new()) {
        Ok(ok) => {
            // dbg!(ok);
        }
        Err(err) => {
            ty_errors.push(err);
        }
    };

    let ty_errors = ty_errors
        .into_iter()
        .map(|err| {
            let err = err.clone();
            err.clone().fmap(|span| {
                let span = spanned_named_expr.clone().traverse(span)?.0.1;

                let span = spanned_ast.clone().traverse(span.clone())?.0.1;

                let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
                let span = result_tok_span_to_char_span(span.clone(), &toks)?;
                Ok(range_to_line_offset_range(span, &file)?)
            })
        })
        .collect();

    Ok((expr, ty_errors))
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

pub fn get_named_expr(file: String) -> anyhow::Result<NamedExpr, anyhow::Error> {
    let toks = Lexer::run(file.clone())?;

    let good_toks = toks
        .clone()
        .into_iter()
        .filter_map(|x| {
            if let ExpectedToken::Token(token) = x.0 {
                Some(token)
            } else {
                None
            }
        })
        .collect();

    let spanned_ast = Parser::run(good_toks)?;

    let ast = spanned_ast.clone().remove_span();

    let spanned_named_expr = Desugar::run(ast).map_err(|err| {
        let err: ExprError<Result<_, anyhow::Error>> = err.fmap(|span| {
            let node = anyhow::Context::context(
                spanned_ast.clone().traverse(span.clone()),
                format!("span: {span:?}"),
            )?;
            let span = node.0.1;

            let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
            let span = result_tok_span_to_char_span(span.clone(), &toks)?;

            Ok(range_to_line_offset_range(span, &file)?)
        });
        err
    })?;

    Ok(remove_span_expr(spanned_named_expr.clone()))
}

// fn format_parentheses(input: &str) -> String {
//     let mut result = String::new();
//     let mut indent_level = 0;
//
//     for c in input.chars() {
//         match c {
//             '(' => {
//                 if !result.ends_with('\n') {
//                     result.push('\n');
//                 }
//                 result.push_str(&"  ".repeat(indent_level)); // indent
//                 result.push('(');
//                 indent_level += 1;
//             }
//             ')' => {
//                 indent_level -= 1;
//                 if !result.ends_with('\n') {
//                     result.push('\n');
//                 }
//                 result.push_str(&"  ".repeat(indent_level));
//                 result.push(')');
//             }
//             _ => {
//                 result.push(c);
//             }
//         }
//     }
//
//     result
// }

// fn format_parentheses(input: &str) -> String {
//     let mut result = String::new();
//     let mut indent_level = 0;
//
//     for c in input.chars() {
//         match c {
//             '(' => {
//                 if !result.ends_with('\n') {
//                     result.push('\n');
//                 }
//                 result.push_str(&"  ".repeat(indent_level)); // indent
//                 result.push('(');
//                 indent_level += 1;
//             }
//             ')' => {
//                 indent_level -= 1;
//                 if !result.ends_with('\n') {
//                     result.push('\n');
//                 }
//                 result.push_str(&"  ".repeat(indent_level));
//                 result.push(')');
//             }
//             _ => {
//                 result.push(c);
//             }
//         }
//     }
//
//     result
// }
