use anyhow::anyhow;
use std::str::FromStr;

use chumsky::{prelude::*, text::*};
use num_bigint::BigUint;

use crate::{
    Compile,
    data::tokens::{AbsoluteIndent, ErrorToken, GToken, InertToken, Spanned, Token, ValidToken},
};

pub struct Lexer;

impl Compile for Lexer {
    type Input = String;

    type Output = Vec<Spanned<Token>>;

    type Error = anyhow::Error;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        lexer()
            .parse(&input)
            .into_result()
            .map_err(|x| anyhow!("failed to parse: {:?}", x))
    }
}

pub fn collect_results<A, E>(v: Vec<Result<A, E>>) -> Result<Vec<A>, E> {
    let mut res = Vec::with_capacity(v.len());

    for r in v {
        match r {
            Ok(a) => res.push(a),
            Err(e) => return Err(e), // return immediately on first error
        }
    }

    Ok(res)
}

fn number_parser<'a>() -> impl Parser<'a, &'a str, BigUint, extra::Err<Rich<'a, char>>> {
    text::digits(10)
        .to_slice()
        .map(|x| BigUint::from_str(x).unwrap())
    // let digits = text::digits(10).to_slice();
    //
    // let frac = just('.').then(digits);
    //
    // let exp = just('e')
    //     .or(just('E'))
    //     .then(one_of("+-").or_not())
    //     .then(digits);
    //
    // just('-')
    //     .or_not()
    //     .then(text::int(10))
    //     .then(frac.or_not())
    //     .then(exp.or_not())
    //     .to_slice()
    //     .map(|s: &str| s.parse().unwrap())
    //     .boxed()
}

// fn count_indent(line: &str) -> usize {
//     line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
// }
//

fn lexer<'a>()
-> impl Parser<'a, &'a str, Vec<Spanned<GToken<AbsoluteIndent>>>, extra::Err<Rich<'a, char>>> {
    // Keywords
    // Why does lambda need to be between ""?
    let lambda = text::keyword("fn").to(ValidToken::Fn);

    let arrow = just('-').then_ignore(just('>')).to(ValidToken::Arrow);
    let double_arrow = just('=').then_ignore(just('>')).to(ValidToken::DoubleArrow);
    let assign = just(':').then_ignore(just('=')).to(ValidToken::Assign);

    let number = number_parser().map(ValidToken::Number);

    let string = ident()
        .delimited_by(just('"'), just('"'))
        .map(|x: &str| ValidToken::String(x.to_string()));

    let base = select! {
        '.' => ValidToken::Dot,
        ':' => ValidToken::Colon,
        '*' => ValidToken::Star,
        ',' => ValidToken::Comma,
        '(' => ValidToken::LParen,
        ')' => ValidToken::RParen,
        '=' => ValidToken::Eq,
        ';' => ValidToken::SemiColon,

    };

    let pipe = just('|').then_ignore(just('>')).to(ValidToken::Pipe);

    // Keywords
    //
    let ident = text::ident().map(|ident: &str| match ident {
        // "car" => Token::Car,
        // "cdr" => Token::Cdr,
        // "cons" => Token::Cons,
        // "Pair" => Token::Pair,
        "def" => ValidToken::Def,
        "Type" => ValidToken::Type,
        "Mod" => ValidToken::Mod,
        // "just" => Token::Just,
        "loop" => ValidToken::Loop,
        "while" => ValidToken::While,
        "for" => ValidToken::For,
        "break" => ValidToken::Break,
        "let" => ValidToken::Let,
        "in" => ValidToken::In,
        // "true" => Token::Bool(true),
        // "false" => Token::Bool(false),
        // "Bool" => Token::BoolTy,
        _ => ValidToken::Ident(String::from(ident)),
    });

    // Integers

    let comment_content = any().and_is(newline().or(end()).not()).repeated().collect();

    let newline = newline()
        .ignore_then(just('\t').repeated().count())
        .map(|n| ValidToken::Newline(AbsoluteIndent(n)));

    // Token parser: choice of all tokens, with whitespace trimmed around
    let token = choice((
        arrow,
        double_arrow,
        lambda,
        assign,
        base,
        string,
        number,
        ident,
        pipe,
        newline,
    ))
    .map_with(|token, e| (GToken::ValidToken(token), e.span()));

    // inert tokens
    let space = whitespace()
        .at_least(1)
        .count()
        .map_with(|amount, e| (GToken::InertToken(InertToken::Space(amount)), e.span()));

    let comment = just("//")
        .ignore_then(comment_content)
        .map_with(|content, e| (GToken::InertToken(InertToken::Comment(content)), e.span()));

    let expected_token = choice((token, space, comment));

    let invalid = any().map_with(|c, e| (GToken::ErrorToken(ErrorToken::InvalidChar(c)), e.span()));

    let normal_token = expected_token.or(invalid).repeated().collect::<Vec<_>>();
    normal_token
}
