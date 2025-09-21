use std::fmt::Display;

use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use chumsky::text::{digits, ident, newline, whitespace};

// TODO: Think about how to handle inert and error tokens, same for nodes
// I think we should not pass them to the parser since they will be ingored anyway.
// We could just propogate errors through the different layers and for example store a
// ErrorNode(TokenError) but The ast should not be aware of a specific token type.

pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone)]
pub enum Token {
    RealToken(RealToken),
    InertToken(InertToken),
}

#[derive(Debug, Clone)]
pub struct ErrorToken {
    pub char: char,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum InertToken {
    Comment(String),
    Space(usize),
    NewLine(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealToken {
    Ident(String),  // variable names, e.g. x, foo
    Number(f64),    // integer literals
    String(String), // 'atoms

    Fn,        // 'fn'
    Dot,       // '.'
    Colon,     // ':'
    SemiColon, // ';'
    Arrow,     // "->"
    DoubleArrow,
    Pipe,
    Star,   // '*'
    Comma,  // ','
    LParen, // '('
    RParen, // ')'
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    Type,

    Eq, // "="
    //
    Assign,      // ":="
    Tabs(usize), // number of tabs

    // sugar keywords
    Loop,
    While,
    For,
    Break,
    Let,
    In,
}

pub fn extract_good_toks(toks: Vec<(Result<Token, ErrorToken>, SimpleSpan)>) -> Vec<RealToken> {
    toks.into_iter()
        .filter_map(|x| Result::ok(x.0))
        .filter_map(|x| match x {
            Token::RealToken(real_token) => Some(real_token),
            Token::InertToken(_) => None,
        })
        .collect()
}

impl Display for RealToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RealToken::Ident(name) => write!(f, "{}", name),
            RealToken::Number(num) => write!(f, "{}", num),
            RealToken::String(s) => write!(f, "\"{}\"", s),

            RealToken::Fn => write!(f, "fn"),
            RealToken::Dot => write!(f, "."),
            RealToken::Colon => write!(f, ":"),
            RealToken::SemiColon => write!(f, ";"),
            RealToken::Arrow => write!(f, "->"),
            RealToken::DoubleArrow => write!(f, "=>"),
            RealToken::Pipe => write!(f, "|"),
            RealToken::Star => write!(f, "*"),
            RealToken::Comma => write!(f, ","),
            RealToken::LParen => write!(f, "("),
            RealToken::RParen => write!(f, ")"),
            RealToken::LBrace => write!(f, "{{"),
            RealToken::RBrace => write!(f, "}}"),
            RealToken::LBracket => write!(f, "["),
            RealToken::RBracket => write!(f, "]"),

            RealToken::Type => write!(f, "Type"),

            RealToken::Eq => write!(f, "="),
            RealToken::Assign => write!(f, "="),
            RealToken::Tabs(usize) => {
                let tabs = "    ".repeat(*usize);
                write!(f, "{}", tabs)
            }

            RealToken::Loop => write!(f, "loop"),
            RealToken::While => write!(f, "while"),
            RealToken::For => write!(f, "for"),
            RealToken::Break => write!(f, "break"),
            RealToken::Let => write!(f, "let"),
            RealToken::In => write!(f, "in"),
        }
    }
}

// type Span = std::ops::Range<usize>;

fn number_parser<'a>() -> impl Parser<'a, &'a str, f64, extra::Err<Rich<'a, char>>> {
    let digits = text::digits(10).to_slice();

    let frac = just('.').then(digits);

    let exp = just('e')
        .or(just('E'))
        .then(one_of("+-").or_not())
        .then(digits);

    just('-')
        .or_not()
        .then(text::int(10))
        .then(frac.or_not())
        .then(exp.or_not())
        .to_slice()
        .map(|s: &str| s.parse().unwrap())
        .boxed()
}

// fix errors by ()
pub fn lexer<'a>()
-> impl Parser<'a, &'a str, Vec<Spanned<Result<Token, ErrorToken>>>, extra::Err<Rich<'a, char>>> {
    // Keywords
    // Why does lambda need to be between ""?
    let lambda = text::keyword("fn").to(RealToken::Fn);

    let arrow = just('-').then_ignore(just('>')).to(RealToken::Arrow);
    let double_arrow = just('=').then_ignore(just('>')).to(RealToken::DoubleArrow);
    let assign = just(':').then_ignore(just('=')).to(RealToken::Assign);

    let number = number_parser().map(RealToken::Number);

    let string = ident()
        .delimited_by(just('"'), just('"'))
        .map(|x: &str| RealToken::String(x.to_string()));

    let base = select! {
        '.' => RealToken::Dot,
        ':' => RealToken::Colon,
        '*' => RealToken::Star,
        ',' => RealToken::Comma,
        '(' => RealToken::LParen,
        ')' => RealToken::RParen,
        '=' => RealToken::Eq,
        ';' => RealToken::SemiColon,

    };

    let pipe = just('|').then_ignore(just('>')).to(RealToken::Pipe);

    // Keywords
    //
    let ident = text::ident().map(|ident: &str| match ident {
        // "car" => Token::Car,
        // "cdr" => Token::Cdr,
        // "cons" => Token::Cons,
        // "Pair" => Token::Pair,
        "Type" => RealToken::Type,
        // "just" => Token::Just,
        "loop" => RealToken::Loop,
        "while" => RealToken::While,
        "for" => RealToken::For,
        "break" => RealToken::Break,
        "let" => RealToken::Let,
        "in" => RealToken::In,
        // "true" => Token::Bool(true),
        // "false" => Token::Bool(false),
        // "Bool" => Token::BoolTy,
        _ => RealToken::Ident(String::from(ident)),
    });

    // Integers

    let comment_content = any().and_is(newline().or(end()).not()).repeated().collect();

    let tabs = just('\t')
        .repeated()
        .at_least(1)
        .count()
        .map(|n| RealToken::Tabs(n));

    // Token parser: choice of all tokens, with whitespace trimmed around
    let token = choice((
        arrow,
        double_arrow,
        lambda,
        base,
        string,
        number,
        ident,
        pipe,
        tabs,
    ))
    .map_with(|token, e| (Ok(Token::RealToken(token)), e.span()));

    // inert tokens
    let space = whitespace()
        .at_least(1)
        .count()
        .map_with(|amount, e| (Ok(Token::InertToken(InertToken::Space(amount))), e.span()));

    let newline = newline()
        .repeated()
        .at_least(1)
        .count()
        .map_with(|amount, e| (Ok(Token::InertToken(InertToken::NewLine(amount))), e.span()));

    let comment = just("//")
        .ignore_then(comment_content)
        .map_with(|content, e| {
            (
                Ok(Token::InertToken(InertToken::Comment(content))),
                e.span(),
            )
        });

    let invalid = any().map_with(|c, e| {
        (
            Err(ErrorToken {
                char: c,
                message: "found illegal token".to_string(),
            }),
            e.span(),
        )
    });
    let error = choice((newline, space, comment, invalid));

    token.or(error).repeated().collect::<Vec<_>>()
}
