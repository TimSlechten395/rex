use chumsky::prelude::*;
use chumsky::text::{digits, ident, newline, whitespace};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Ident(String), // variable names, e.g. x, foo
    Int(i64),      // integer literals
    Bool(bool),    // true or false literals
    BoolTy,
    Atom(String), // 'atoms

    Lambda, // '\' or 'λ'
    Dot,    // '.'
    Colon,  // ':'
    Arrow,  // "->"
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

pub fn lexer<'a>() -> impl Parser<'a, &'a str, Vec<Token>, extra::Err<Rich<'a, char>>> {
    // Keywords
    // Why does lambda need to be between ""?
    let lambda = just("λ")
        .or(text::keyword("lambda"))
        .or(text::keyword("fn"))
        .to(Token::Lambda);

    let dot = just('.').to(Token::Dot);
    let colon = just(':').to(Token::Colon);
    let arrow = just('-').then_ignore(just('>')).to(Token::Arrow);

    let atom_lit = just('\'')
        .ignore_then(ident())
        .map(|x: &str| Token::Atom(x.to_string()));

    let star = just('*').to(Token::Star);
    let comma = just(',').to(Token::Comma);
    let lparen = just('(').to(Token::LParen);
    let rparen = just(')').to(Token::RParen);
    let eq = just('=').to(Token::Eq);
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
        // "true" => Token::Bool(true),
        // "false" => Token::Bool(false),
        // "Bool" => Token::BoolTy,
        _ => Token::Ident(String::from(ident)),
    });

    // Integers
    let integer = digits(10)
        .collect::<String>()
        .from_str()
        .unwrapped()
        .map(Token::Int);

    let comment_content = any().and_is(newline().or(end()).not()).repeated().collect();

    let comment = just("//")
        .ignore_then(comment_content)
        .map(|content| Token::Comment(content));

    // Token parser: choice of all tokens, with whitespace trimmed around
    let token = choice((
        comment, lambda, dot, colon, arrow, atom_lit, star, comma, lparen, rparen, integer, ident,
        eq, pipe,
    ))
    .padded();

    token.repeated().collect::<Vec<_>>()
}
