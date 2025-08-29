use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use chumsky::text::{digits, ident, newline, whitespace};

pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),  // variable names, e.g. x, foo
    Number(f64),    // integer literals
    Bool(bool),     // true or false literals
    String(String), // 'atoms
    BoolTy,

    Lambda, // '\' or 'λ'
    Dot,    // '.'
    Colon,  // ':'
    SemiColon,
    Arrow, // "->"
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

    Car, // "proj1"
    Cdr, // "proj2"

    Eq, // "="
    // keywords
    Cons,
    Pair,
    Claim,
    Define,
    Just,
    Comment(String),

    // sugar keywords
    Loop,
    While,
    For,
    Break,
    Let,
    In,
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

pub fn lexer<'a>() -> impl Parser<'a, &'a str, Vec<Spanned<Token>>, extra::Err<Rich<'a, char>>> {
    // Keywords
    // Why does lambda need to be between ""?
    let lambda = just("λ")
        .or(text::keyword("lambda"))
        .or(text::keyword("fn"))
        .to(Token::Lambda);

    let arrow = just('-').then_ignore(just('>')).to(Token::Arrow);

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

    };

    let pipe = just('|').then_ignore(just('>')).to(Token::Pipe);

    // Keywords
    //
    let ident = text::ident().map(|ident: &str| match ident {
        // "car" => Token::Car,
        // "cdr" => Token::Cdr,
        // "cons" => Token::Cons,
        // "Pair" => Token::Pair,
        "claim" => Token::Claim,
        "define" => Token::Define,
        "Type" => Token::Type,
        // "just" => Token::Just,
        "loop" => Token::Loop,
        "while" => Token::While,
        "for" => Token::For,
        "break" => Token::Break,
        "let" => Token::Let,
        "in" => Token::In,
        "true" => Token::Bool(true),
        "false" => Token::Bool(false),
        // "Bool" => Token::BoolTy,
        _ => Token::Ident(String::from(ident)),
    });

    // Integers

    let comment_content = any().and_is(newline().or(end()).not()).repeated().collect();

    let comment = just("//")
        .ignore_then(comment_content)
        .map(|content| Token::Comment(content));

    // Token parser: choice of all tokens, with whitespace trimmed around
    let token = choice((comment, lambda, base, arrow, string, number, ident, pipe))
        .map_with(|token, e| (token, e.span()))
        .padded();

    token.repeated().collect::<Vec<_>>()
}
