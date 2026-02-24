use std::ops::Range;

use crate::{
    Compile,
    data::{
        ast::{Ast, AstError, LitKind, SpannedFixAst, SpannedResultAst, get_fix_ast},
        tokens::{AbsoluteIndent, Token},
    },
};

pub struct Parser;

impl Compile for Parser {
    type Input = Vec<Token<AbsoluteIndent>>;
    type Output = SpannedFixAst;
    type Error = AstError<SpannedResultAst>;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        get_fix_ast(parse(input))
    }
}

#[derive(Debug, Clone, PartialEq)]
enum TokenTree {
    Leaf(Token<AbsoluteIndent>),
    Group(Vec<Spanned<TokenTree>>),
    Dot(Vec<Spanned<TokenTree>>),
}

// idea every new indentations starts a new block
// if it is a module block then each line is a definition
// if it is a continuation block each line is just the continuation of the previous one

pub type Spanned<T> = (T, Range<usize>);

// NOTE: our span is range_inclusive
pub fn parse(tokens: Vec<Token<AbsoluteIndent>>) -> SpannedResultAst {
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

    // if there is one that starts with def we name resolve
    let items = ignore_new_lines.split(|x| if let Token::Def = x.1 { true } else { false });

    let is_mod = &items.clone().next().is_some_and(|x| x.is_empty());

    let mut items = items.filter(|x| !x.is_empty());

    // if its a list of definitions is a module else it depends
    let ast = if *is_mod {
        Ast::Module(
            Vec::new(),
            items
                .map(|x| parse_assign(&parse_group_start(&mut x.to_vec().into_iter())))
                .collect(),
        )
    } else {
        match items.clone().count() {
            0 => Ast::Unit,
            1 => {
                return parse_assign(&parse_group_start(
                    &mut items.next().unwrap().to_vec().into_iter(),
                ));
            }
            _ => Ast::Sigma(
                items
                    .map(|x| parse_assign(&parse_group_start(&mut x.to_vec().into_iter())))
                    .collect(),
            ),
        }
    };

    SpannedResultAst((
        Ok(ast),
        Range {
            start: 0,
            end: end - 1,
        },
    ))
}

fn get_span_from_tree(tokens: &[Spanned<TokenTree>]) -> Range<usize> {
    let start = tokens.first().map(|x| x.1.start).unwrap_or_default();
    let end = tokens.last().map(|x| x.1.end).unwrap_or_default();

    Range { start, end }
}

fn parse_assign(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Assign));
    let expr: Vec<_> = splits.map(|x| parse_ann(x)).collect();

    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Binding(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_ann(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Colon));
    let expr: Vec<_> = splits.map(|x| parse_lambda(x)).collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Ann(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_lambda(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::DoubleArrow));
    let expr: Vec<_> = splits.map(|x| parse_pi(x)).collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Lambda(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_pi(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Arrow));
    let expr: Vec<_> = splits.map(|x| parse_pipe(x)).collect();

    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Pi(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_pipe(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Pipe));
    let expr: Vec<_> = splits.map(|x| parse_sigma(x)).collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Pipe(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_sigma(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::SemiColon));
    let expr: Vec<_> = splits.map(|x| parse_tuple(x)).collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Sigma(expr)), get_span_from_tree(&tokens))),
    }
}

fn parse_tuple(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let splits = tokens.split(|x| x.0 == TokenTree::Leaf(Token::Comma));
    let expr: Vec<_> = splits.map(|x| parse_dot(x)).collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::Tuple(expr)), get_span_from_tree(&tokens))),
    }
}

// this is just like parens a special case
fn parse_dot(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
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

// this is the final case meaning it handles a list of expressions without delimiters
fn parse_app(tokens: &[Spanned<TokenTree>]) -> SpannedResultAst {
    let expr: Vec<_> = tokens
        .iter()
        .map(|x| {
            let token = match x.0.clone() {
                TokenTree::Leaf(token) => match token {
                    Token::Type => Ok(Ast::Type),
                    Token::Ident(name) => Ok(Ast::Var(name)),
                    Token::String(name) => Ok(Ast::Lit(LitKind::String(name))),
                    Token::Number(x) => Ok(Ast::Lit(LitKind::Number(x))),
                    token => Err(AstError::InvalidToken(token)),
                },
                TokenTree::Group(group) => Ok(Ast::Group(Box::new(parse_assign(&group)))),
                TokenTree::Dot(items) => Ok(Ast::Dot(
                    items.into_iter().map(|x| parse_assign(&[x])).collect(),
                )),
            };
            SpannedResultAst((token, x.1.clone()))
        })
        .collect();
    match expr.len() {
        0 => SpannedResultAst((Ok(Ast::Unit), get_span_from_tree(&tokens))),
        1 => expr.first().unwrap().clone(),
        _ => SpannedResultAst((Ok(Ast::App(expr)), get_span_from_tree(&tokens))),
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
