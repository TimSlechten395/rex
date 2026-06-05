use anyhow::{Context as _, anyhow, bail};
use functor_derive::Functor;
use std::any::type_name_of_val;
use std::collections::HashMap;
use std::fs::read_to_string;
use std::ops::Range;

use crate::data::ast::{SpannedFixAst, traverse_list};
use crate::data::expr::{Defs, Expr, ExprError, ExprF, GDef, GExpr, NamedExpr, traverse_defs};
use crate::data::tokens::{
    self, AbsoluteIndent, GToken, result_tok_span_to_char_span, tok_span_to_result_tok_span,
};
use crate::eval::normal_form;
use crate::pipeline::desugar::{Binding, Desugar, replace_defs};
use crate::pipeline::lexer::Lexer;

use crate::pipeline::name_resolver::{NameResolver, ResolveError};
use crate::pipeline::parser::Parser;
use crate::tools::printer::print_expr;
use crate::tools::repl;
use crate::r#type::{TypeError, eq, infer_type};

pub mod data;
pub mod pipeline;

pub mod bootstrap;
pub mod eval;
pub mod nbe;

pub mod helper;
pub mod r#type;

pub mod cache;
pub mod experimental;

pub mod tools;
pub mod zipper;

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
    code: String,
) -> anyhow::Result<(
    Vec<tokens::Spanned<GToken<AbsoluteIndent>>>,
    Vec<SpannedFixAst>,
    Defs,
    Vec<(
        String,
        Vec<TypeError<(Expr, anyhow::Result<Range<(usize, usize)>, anyhow::Error>)>>,
    )>,
)> {
    let toks = Lexer::run(code.clone())?;

    let good_toks = toks
        .clone()
        .into_iter()
        .filter_map(|x| {
            if let GToken::ValidToken(token) = x.0 {
                Some(token)
            } else {
                None
            }
        })
        .collect();

    let spanned_asts = Parser::run(good_toks)?;

    let asts = spanned_asts
        .clone()
        .into_iter()
        .map(|x| x.remove_span())
        .collect::<Vec<_>>();

    let spanned_named_defs = Desugar::run(asts).map_err(|err| {
        let err: ExprError<Result<_, anyhow::Error>> = err.fmap(|span| {
            let mut span = span.into_iter();
            let node_id = span.next().ok_or(anyhow!("empty span 0"))?;
            let span: Vec<_> = span.collect();
            let node = anyhow::Context::context(
                spanned_asts
                    .get(node_id)
                    .unwrap()
                    .clone()
                    .traverse(span.clone()),
                format!("span: {span:?}"),
            )?;
            let span = node.0.1;

            let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
            let span = result_tok_span_to_char_span(span.clone(), &toks)?;

            Ok(range_to_line_offset_range(span, &code)?)
        });
        err
    })?;

    let spanned_defs = NameResolver::run(spanned_named_defs.clone())?;
    // TODO: fix

    let ty_errors: Vec<_> = spanned_defs
        .clone()
        .0
        .into_iter()
        .enumerate()
        .filter_map(|(i, GDef { name, ty, val })| {
            let mut ty_errors = Vec::new();
            let ty_errors = {
                match infer_type(
                    val.clone().remove_span(),
                    Vec::new(),
                    &mut ty_errors,
                    vec![i, 1],
                ) {
                    Ok(inf) => {
                        if let Some(ty) = ty {
                            // TODO: fix span
                            let exp = ty.clone().remove_span();

                            if !eq(inf.clone(), exp.clone()) {
                                ty_errors.push(TypeError::TypeMismatch {
                                    expected: (normal_form(exp.clone()), vec![i, 0]),
                                    //
                                    found: (normal_form(inf.clone()), vec![i, 1]),
                                });
                                // println!(
                                //     "Type Mismatch with annotation in '{}':\n\texpected: '{}'\n\tfound   : '{}'",
                                //     name,
                                //     print_expr(normal_form(exp.clone())),
                                //     print_expr(normal_form(inf.clone())),
                                // )
                            }
                        }
                    }
                    Err(err) => {
                        ty_errors.push(err);
                    }
                };

                ty_errors
                    .into_iter()
                    .map(|err| {
                        let err = err.clone();
                        err.clone().fmap(|err| {
                            let span = || {
                                let span = match traverse_defs(spanned_named_defs.clone(), err.1)? {
                                    either::Either::Left(e) => e.0.1,
                                    either::Either::Right(s) => s.1,
                                };

                                let span = traverse_list(spanned_asts.clone(), span)?.0.1;

                                let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
                                let span = result_tok_span_to_char_span(span.clone(), &toks)?;
                                Ok(range_to_line_offset_range(span, &code)?)
                            };
                            (err.0, span())
                        })
                    })
                    .collect()
            };

            Some((name.0.clone(), ty_errors))
        })
        .collect();

    Ok((toks, spanned_asts, spanned_defs, ty_errors))
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

// pub fn get_named_expr(file: String) -> anyhow::Result<NamedExpr, anyhow::Error> {
//     let toks = Lexer::run(file.clone())?;
//
//     let good_toks = toks
//         .clone()
//         .into_iter()
//         .filter_map(|x| {
//             if let ExpectedToken::Token(token) = x.0 {
//                 Some(token)
//             } else {
//                 None
//             }
//         })
//         .collect();
//
//     let spanned_ast = Parser::run(good_toks)?;
//
//     let ast = spanned_ast.clone().remove_span();
//
//     let spanned_named_expr = Desugar::run(ast).map_err(|err| {
//         let err: ExprError<Result<_, anyhow::Error>> = err.fmap(|span| {
//             let node = anyhow::Context::context(
//                 spanned_ast.clone().traverse(span.clone()),
//                 format!("span: {span:?}"),
//             )?;
//             let span = node.0.1;
//
//             let span = tok_span_to_result_tok_span(span.clone(), &toks)?;
//             let span = result_tok_span_to_char_span(span.clone(), &toks)?;
//
//             Ok(range_to_line_offset_range(span, &file)?)
//         });
//         err
//     })?;
//
//     Ok(remove_span_expr(spanned_named_expr.clone()))
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
