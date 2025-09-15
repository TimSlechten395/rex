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
pub enum ErrorToken {
    ErrorToken(char),
    // HACK: Comments are not really errors
    Comment(String),
    Space(usize),
    NewLine(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
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

    // sugar keywords
    Loop,
    While,
    For,
    Break,
    Let,
    In,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(name) => write!(f, "{}", name),
            Token::Number(num) => write!(f, "{}", num),
            Token::String(s) => write!(f, "\"{}\"", s),

            Token::Fn => write!(f, "fn"),
            Token::Dot => write!(f, "."),
            Token::Colon => write!(f, ":"),
            Token::SemiColon => write!(f, ";"),
            Token::Arrow => write!(f, "->"),
            Token::DoubleArrow => write!(f, "=>"),
            Token::Pipe => write!(f, "|"),
            Token::Star => write!(f, "*"),
            Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),

            Token::Type => write!(f, "Type"),

            Token::Eq => write!(f, "="),

            Token::Loop => write!(f, "loop"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Break => write!(f, "break"),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
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
    let lambda = text::keyword("fn").to(Token::Fn);

    let arrow = just('-').then_ignore(just('>')).to(Token::Arrow);
    let double_arrow = just('=').then_ignore(just('>')).to(Token::DoubleArrow);

    let number = number_parser().map(Token::Number);

    let string = ident()
        .delimited_by(just('"'), just('"'))
        .map(|x: &str| Token::String(x.to_string()));

    let base = select! {
        '.' => Token::Dot,
        ':' => Token::Colon,
        '*' => Token::Star,
        ',' => Token::Comma,
        '(' => Token::LParen,
        ')' => Token::RParen,
        '=' => Token::Eq,
        ';' => Token::SemiColon,

    };

    let pipe = just('|').then_ignore(just('>')).to(Token::Pipe);

    // Keywords
    //
    let ident = text::ident().map(|ident: &str| match ident {
        // "car" => Token::Car,
        // "cdr" => Token::Cdr,
        // "cons" => Token::Cons,
        // "Pair" => Token::Pair,
        "Type" => Token::Type,
        // "just" => Token::Just,
        "loop" => Token::Loop,
        "while" => Token::While,
        "for" => Token::For,
        "break" => Token::Break,
        "let" => Token::Let,
        "in" => Token::In,
        // "true" => Token::Bool(true),
        // "false" => Token::Bool(false),
        // "Bool" => Token::BoolTy,
        _ => Token::Ident(String::from(ident)),
    });

    // Integers

    let comment_content = any().and_is(newline().or(end()).not()).repeated().collect();

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
    ))
    .map_with(|token, e| (Ok(token), e.span()));

    // inert tokens
    let space = whitespace()
        .at_least(1)
        .count()
        .map_with(|amount, e| (Err(ErrorToken::Space(amount)), e.span()));

    let newline = newline()
        .repeated()
        .at_least(1)
        .count()
        .map_with(|amount, e| (Err(ErrorToken::NewLine(amount)), e.span()));

    let comment = just("//")
        .ignore_then(comment_content)
        .map_with(|content, e| (Err(ErrorToken::Comment(content)), e.span()));

    let invalid = any().map_with(|c, e| (Err(ErrorToken::ErrorToken(c)), e.span()));
    let error = choice((space, newline, comment, invalid));

    token.or(error).repeated().collect::<Vec<_>>()
}
