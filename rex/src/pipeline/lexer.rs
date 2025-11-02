use std::str::FromStr;

use chumsky::{prelude::*, text::*};
use num_bigint::BigUint;

use crate::data::tokens::{AbsoluteIndent, ErrorToken, ExpectedToken, InertToken, Spanned, Token};

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

fn count_indent(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

pub fn lexer<'a>() -> impl Parser<
    'a,
    &'a str,
    Vec<Spanned<Result<ExpectedToken<AbsoluteIndent>, ErrorToken>>>,
    extra::Err<Rich<'a, char>>,
> {
    // Keywords
    // Why does lambda need to be between ""?
    let lambda = text::keyword("fn").to(Token::Fn);

    let arrow = just('-').then_ignore(just('>')).to(Token::Arrow);
    let double_arrow = just('=').then_ignore(just('>')).to(Token::DoubleArrow);
    let assign = just(':').then_ignore(just('=')).to(Token::Assign);

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
        "def" => Token::Def,
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

    let newline = newline()
        .ignore_then(just('\t').repeated().count())
        .map(|n| Token::Newline(AbsoluteIndent(n)));

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
    .map_with(|token, e| (Ok(ExpectedToken::Token(token)), e.span()));

    // inert tokens
    let space = whitespace().at_least(1).count().map_with(|amount, e| {
        (
            Ok(ExpectedToken::InertToken(InertToken::Space(amount))),
            e.span(),
        )
    });

    let comment = just("//")
        .ignore_then(comment_content)
        .map_with(|content, e| {
            (
                Ok(ExpectedToken::InertToken(InertToken::Comment(content))),
                e.span(),
            )
        });

    let expected_token = choice((token, space, comment));

    let invalid = any().map_with(|c, e| (Err(ErrorToken::InvalidChar(c)), e.span()));

    let normal_token = expected_token.or(invalid).repeated().collect::<Vec<_>>();
    normal_token
}
