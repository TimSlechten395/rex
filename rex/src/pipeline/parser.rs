use std::ops::Range;

use crate::data::{
    ast::{LitKind, SpannedResultSugarExpr, SugarExpr, SugarExprError},
    tokens::{AbsoluteIndent, Token},
};

#[derive(Debug, Clone, PartialEq)]
enum TokenTree {
    Leaf(Token<AbsoluteIndent>),
    Group(Vec<Spanned<TokenTree>>),
    Dot(Vec<Spanned<TokenTree>>),
}

pub type Spanned<T> = (T, Range<usize>);

// NOTE: our span is range_inclusive
pub fn parse(tokens: Vec<Token<AbsoluteIndent>>, self_dep: bool) -> SpannedResultSugarExpr {
    let end = tokens.len();

    // indentation should be important
    let ignore_new_lines = tokens
        .into_iter()
        .enumerate()
        .filter(|x| {
            if let Token::Newline(AbsoluteIndent(_)) = x.1 {
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>();

    let items = ignore_new_lines
        .split(|x| if let Token::Def = x.1 { true } else { false })
        .filter(|x| !x.is_empty());

    SpannedResultSugarExpr((
        Ok(SugarExpr::Module(
            Vec::new(),
            items
                .map(|x| parse_assign(&parse_group_start(&mut x.to_vec().into_iter())))
                .collect(),
            self_dep,
        )),
        Range { start: 0, end },
    ))
}

pub fn get_span_from_tree(tokens: &[Spanned<TokenTree>]) -> Range<usize> {
    let start = tokens.first().map(|x| x.1.start).unwrap_or_default();
    let end = tokens.last().map(|x| x.1.end).unwrap_or_default();

    Range { start, end }
}

pub fn parse_assign(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Assign));
    let expr: Vec<_> = splits.map(|x| parse_ann(x)).collect();

    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Binding(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_ann(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Colon));
    let expr: Vec<_> = splits.map(|x| parse_lambda(x)).collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Ann(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_lambda(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::DoubleArrow));
    let expr: Vec<_> = splits.map(|x| parse_pi(x)).collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Lambda(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_pi(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Arrow));
    let expr: Vec<_> = splits.map(|x| parse_pipe(x)).collect();

    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Pi(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_pipe(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Pipe));
    let expr: Vec<_> = splits.map(|x| parse_tuple(x)).collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Pipe(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_sigma(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::SemiColon));
    let expr: Vec<_> = splits.map(|x| parse_dot(x)).collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Sigma(expr)), get_span_from_tree(&tokens))),
    }
}

pub fn parse_tuple(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Comma));
    let expr: Vec<_> = splits.map(|x| parse_dot(x)).collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::Tuple(expr)), get_span_from_tree(&tokens))),
    }
}

// this is just like parens a special case
pub fn parse_dot(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let new_tokens: Vec<Spanned<TokenTree>> = Vec::new();
    let expr: Vec<_> = tokens.into_iter().fold(new_tokens, |mut acc, item| {
        match acc.last() {
            Some((TokenTree::Leaf(Token::Dot), _span)) => {
                let _dot = acc.pop();
                if let Some(prev) = acc.pop() {
                    let new_token = match prev {
                        (TokenTree::Dot(mut items), span) => {
                            items.push(item.clone());
                            (
                                TokenTree::Dot(items),
                                Range {
                                    start: span.start,
                                    end: item.1.end,
                                },
                            )
                        }
                        prev_item => (
                            TokenTree::Dot(vec![prev_item.clone(), item.clone()]),
                            Range {
                                start: prev_item.1.start,
                                end: item.1.end,
                            },
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

// this is the final case meaning it handles a list of expressions without delimiter
pub fn parse_app(tokens: &[Spanned<TokenTree>]) -> SpannedResultSugarExpr {
    let expr: Vec<_> = tokens
        .iter()
        .map(|x| {
            let token = match x.0.clone() {
                TokenTree::Leaf(token) => match token {
                    Token::Type => Ok(SugarExpr::Type),
                    Token::Ident(name) => Ok(SugarExpr::Var(name)),
                    Token::String(name) => Ok(SugarExpr::Lit(LitKind::String(name))),
                    Token::Number(x) => Ok(SugarExpr::Lit(LitKind::Number(x))),
                    token => Err(SugarExprError::InvalidToken(token)),
                },
                TokenTree::Group(group) => Ok(SugarExpr::Group(Box::new(parse_assign(&group)))),
                TokenTree::Dot(items) => Ok(SugarExpr::Dot(
                    items.into_iter().map(|x| parse_assign(&[x])).collect(),
                )),
            };
            SpannedResultSugarExpr((token, x.1.clone()))
        })
        .collect();
    match expr.len() {
        0 => SpannedResultSugarExpr((Ok(SugarExpr::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultSugarExpr((Ok(SugarExpr::App(expr)), get_span_from_tree(&tokens))),
    }
}

// TODO: return errors instead
fn parse_group_start<I>(tokens: &mut I) -> Vec<Spanned<TokenTree>>
where
    I: Iterator<Item = (usize, Token<AbsoluteIndent>)>,
{
    let mut trees = Vec::new();

    while let Some((i, tok)) = tokens.next() {
        match tok {
            Token::LParen => {
                let group = parse_group_end(tokens, i);
                trees.push(group);
            }
            Token::RParen => panic!("unexpected closing parenthesis"),
            _ => trees.push((TokenTree::Leaf(tok.clone()), Range { start: i, end: i })),
        }
    }

    trees
}

fn parse_group_end<I>(iter: &mut I, start: usize) -> Spanned<TokenTree>
where
    I: Iterator<Item = (usize, Token<AbsoluteIndent>)>,
{
    let mut inner = Vec::new();

    while let Some((i, tok)) = iter.next() {
        match tok {
            Token::LParen => {
                let group = parse_group_end(iter, i);
                inner.push(group);
            }
            Token::RParen => return (TokenTree::Group(inner), Range { start, end: i }),
            _ => inner.push((TokenTree::Leaf(tok), Range { start: i, end: i })),
        }
    }
    panic!("unterminated paren group")
}
