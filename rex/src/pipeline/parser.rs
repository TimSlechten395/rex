/// We use a special parsing technique optimized for error handling and parallellization
/// The idea is as follows: identify the most important tokens and these can either create a new
/// token or split the whole tokentree
use std::ops::{Range, RangeInclusive};

use crate::{
    Compile,
    data::{
        ast::{Ast, AstError, LitKind, SpannedFixAst},
        tokens::{AbsoluteIndent, Token, ValidToken},
    },
};

pub struct Parser;

impl Compile for Parser {
    type Input = Vec<ValidToken<AbsoluteIndent>>;
    type Output = Vec<SpannedFixAst>;
    type Error = AstError;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(parse(input))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TokenTree {
    Leaf(ValidToken<AbsoluteIndent>),
    Group(Vec<Spanned<TokenTree>>),
    Dot(Vec<Spanned<TokenTree>>),
}

// idea every new indentations starts a new block
// if it is a module block then each line is a definition
// if it is a continuation block each line is just the continuation of the previous one

pub type Spanned<T> = (T, RangeInclusive<usize>);

// NOTE: our span is range_inclusive
pub fn parse(tokens: Vec<ValidToken<AbsoluteIndent>>) -> Vec<SpannedFixAst> {
    let end = tokens.len();

    // indentation should be important
    let mut ignore_new_lines = tokens.into_iter().enumerate().filter(|x| {
        if let ValidToken::Newline(AbsoluteIndent(_)) = x.1 {
            false
        } else {
            true
        }
    });

    // if there is one that starts with def we name resolve
    //
    match ignore_new_lines.next() {
        Some((_, ValidToken::Def)) => {}
        Some(n) => {
            return vec![SpannedFixAst((
                Ast::Error(AstError::InvalidToken(n.1), vec![]),
                0..=0,
            ))];
        }
        None => return Vec::new(),
    };

    let items = ignore_new_lines.collect::<Vec<_>>();

    let items = items.split(|x| {
        if let ValidToken::Def = x.1 {
            true
        } else {
            false
        }
    });

    items
        .map(|x| parse_assign(&parse_group_start(&mut x.to_vec().into_iter())))
        .collect()
}

fn get_span_from_tree(tokens: &[Spanned<TokenTree>]) -> RangeInclusive<usize> {
    let start = tokens.first().map(|x| *x.1.start()).unwrap_or_default();
    let end = tokens.last().map(|x| *x.1.end()).unwrap_or_default();

    start..=end
}

fn parse_assign(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::Assign));
    let expr: Vec<_> = splits.map(|x| parse_ann(x)).collect();

    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Binding(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_ann(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::Colon));
    let expr: Vec<_> = splits.map(|x| parse_lambda(x)).collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Ann(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_lambda(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::DoubleArrow));
    let expr: Vec<_> = splits.map(|x| parse_pi(x)).collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Lambda(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_pi(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::Arrow));
    let expr: Vec<_> = splits.map(|x| parse_pipe(x)).collect();

    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Pi(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_pipe(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::Pipe));
    let expr: Vec<_> = splits.map(|x| parse_sigma(x)).collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Pipe(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_sigma(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::SemiColon));
    let expr: Vec<_> = splits.map(|x| parse_tuple(x)).collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Sigma(expr), get_span_from_tree(&tokens))),
    }
}

fn parse_tuple(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(ValidToken::Comma));
    let expr: Vec<_> = splits.map(|x| parse_dot(x)).collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::Tuple(expr), get_span_from_tree(&tokens))),
    }
}

// this is just like parens a special case
fn parse_dot(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let new_tokens: Vec<Spanned<TokenTree>> = Vec::new();
    let expr: Vec<_> = tokens.into_iter().fold(new_tokens, |mut acc, item| {
        match acc.last() {
            Some((TokenTree::Leaf(ValidToken::Dot), _span)) => {
                let _dot = acc.pop();
                if let Some(prev) = acc.pop() {
                    let new_token = match prev {
                        (TokenTree::Dot(mut items), span) => {
                            items.push(item.clone());
                            (TokenTree::Dot(items), *span.start()..=*item.1.end())
                        }
                        prev_item => (
                            TokenTree::Dot(vec![prev_item.clone(), item.clone()]),
                            *prev_item.1.start()..=*item.1.end(),
                        ),
                    };
                    acc.push(new_token);
                } else {
                    acc.push(item.clone());
                }
            }

            _ => {
                acc.push(item.clone());
            }
        }
        acc
    });
    parse_app(&expr)
}

// this is the final case meaning it handles a list of expressions without delimiters
fn parse_app(tokens: &[Spanned<TokenTree>]) -> SpannedFixAst {
    let expr: Vec<_> = tokens
        .iter()
        .map(|x| {
            let token = match x.0.clone() {
                TokenTree::Leaf(token) => match token {
                    ValidToken::Type => Ast::Type,
                    ValidToken::Ident(name) => Ast::Var(name),
                    ValidToken::String(name) => Ast::Lit(LitKind::String(name)),
                    ValidToken::Number(x) => Ast::Lit(LitKind::Number(x)),
                    token => Ast::Error(AstError::InvalidToken(token), vec![]),
                },
                TokenTree::Group(group) => Ast::Group(Box::new(parse_assign(&group))),
                TokenTree::Dot(items) => {
                    Ast::Dot(items.into_iter().map(|x| parse_assign(&[x])).collect())
                }
            };
            SpannedFixAst((token, x.1.clone()))
        })
        .collect();
    match expr.len() {
        0 => SpannedFixAst((Ast::Unit, get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedFixAst((Ast::App(expr), get_span_from_tree(&tokens))),
    }
}

// TODO: return errors instead
fn parse_group_start<I>(tokens: &mut I) -> Vec<Spanned<TokenTree>>
where
    I: Iterator<Item = (usize, ValidToken<AbsoluteIndent>)>,
{
    let mut trees = Vec::new();

    while let Some((i, tok)) = tokens.next() {
        match tok {
            ValidToken::LParen => {
                let group = parse_group_end(tokens, i);
                trees.push(group);
            }
            ValidToken::RParen => panic!("unexpected closing parenthesis"),
            _ => trees.push((TokenTree::Leaf(tok.clone()), i..=i)),
        }
    }

    trees
}

fn parse_group_end<I>(iter: &mut I, start: usize) -> Spanned<TokenTree>
where
    I: Iterator<Item = (usize, ValidToken<AbsoluteIndent>)>,
{
    let mut inner = Vec::new();

    while let Some((i, tok)) = iter.next() {
        match tok {
            ValidToken::LParen => {
                let group = parse_group_end(iter, i);
                inner.push(group);
            }
            ValidToken::RParen => return (TokenTree::Group(inner), start..=i),
            _ => inner.push((TokenTree::Leaf(tok), i..=i)),
        }
    }
    panic!("unterminated paren group")
}
