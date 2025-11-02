use std::fmt::Display;
use std::ops::Range;
use std::str::FromStr;

use anyhow::anyhow;
use chumsky::prelude::*;
use chumsky::span::SimpleSpan;
use chumsky::text::{digits, ident, newline, whitespace};
use num_bigint::BigUint;

pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone)]
pub enum ExpectedToken<Tab> {
    Token(Token<Tab>),
    InertToken(InertToken),
}

impl Token<RelativeIndent> {
    pub fn from_generic(tok: Token<usize>) -> Self {
        match tok {
            Token::Newline(_) => {
                panic!("Use tabs_to_indent to convert Newline(usize) into Newline(RelativeIndent)")
            }

            Token::Ident(s) => Token::Ident(s),
            Token::Number(n) => Token::Number(n),
            Token::String(s) => Token::String(s),

            Token::Def => Token::Def,
            Token::Fn => Token::Fn,
            Token::Dot => Token::Dot,
            Token::Colon => Token::Colon,
            Token::SemiColon => Token::SemiColon,
            Token::Arrow => Token::Arrow,
            Token::DoubleArrow => Token::DoubleArrow,
            Token::Pipe => Token::Pipe,
            Token::Star => Token::Star,
            Token::Comma => Token::Comma,
            Token::LParen => Token::LParen,
            Token::RParen => Token::RParen,
            Token::LBrace => Token::LBrace,
            Token::RBrace => Token::RBrace,
            Token::LBracket => Token::LBracket,
            Token::RBracket => Token::RBracket,
            Token::Type => Token::Type,
            Token::Eq => Token::Eq,
            Token::Assign => Token::Assign,

            Token::Loop => Token::Loop,
            Token::While => Token::While,
            Token::For => Token::For,
            Token::Break => Token::Break,
            Token::Let => Token::Let,
            Token::In => Token::In,
        }
    }
}

pub enum RelativeIndent {
    Indent(usize),
    Dedent(usize),
    Same,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbsoluteIndent(pub usize);

// TODO: misplacedTabs does not belong here since its part of the second step
#[derive(Debug, Clone)]
pub enum ErrorToken {
    InvalidChar(char),
    MisplacedTabs(usize),
}

#[derive(Debug, Clone)]
pub enum InertToken {
    Comment(String),
    Space(usize),
}

// generic over tab_kind
#[derive(Debug, Clone, PartialEq)]
pub enum Token<Indent> {
    Ident(String),   // variable names, e.g. x, foo
    Number(BigUint), // integer literals
    String(String),  // 'atoms

    Def,       // 'def'
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
    Assign, // ":="
    // This is kind of a hack
    Newline(Indent),

    // sugar keywords
    Loop,
    While,
    For,
    Break,
    Let,
    In,
}

pub fn extract_good_toks<T>(
    toks: Vec<(Result<ExpectedToken<T>, ErrorToken>, SimpleSpan)>,
) -> Vec<Token<T>> {
    toks.into_iter()
        .filter_map(|x| Result::ok(x.0))
        .filter_map(|x| match x {
            ExpectedToken::Token(real_token) => Some(real_token),
            ExpectedToken::InertToken(_) => None,
        })
        .collect()
}

impl Display for Token<AbsoluteIndent> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ident(name) => write!(f, "{}", name),
            Token::Number(num) => write!(f, "{}", num),
            Token::String(s) => write!(f, "\"{}\"", s),

            Token::Fn => write!(f, "fn"),
            Token::Def => write!(f, "Def"),
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
            Token::Assign => write!(f, "="),
            Token::Newline(AbsoluteIndent(n)) => {
                let tabs = "\t".repeat(*n);
                write!(f, "\n{tabs}")
            }

            Token::Loop => write!(f, "loop"),
            Token::While => write!(f, "while"),
            Token::For => write!(f, "for"),
            Token::Break => write!(f, "break"),
            Token::Let => write!(f, "let"),
            Token::In => write!(f, "in"),
        }
    }
}

// maps span from Vec<Token> to Vec<Result<Token>>
pub fn tok_span_to_result_tok_span(
    span: Range<usize>,
    full_tokens: &[(
        Result<ExpectedToken<AbsoluteIndent>, ErrorToken>,
        Range<usize>,
    )],
) -> anyhow::Result<Range<usize>> {
    let mut ok_count = 0;
    let mut start_idx = None;
    let mut end_idx = None;
    for (i, t) in full_tokens.iter().enumerate() {
        if matches!(t.0, Ok(ExpectedToken::Token(_))) {
            if ok_count == span.start {
                start_idx = Some(i)
            }
            if ok_count == span.end {
                end_idx = Some(i);
                break;
            }
            ok_count += 1;
        }
    }

    let start = start_idx.ok_or(anyhow!("start index not in range"))?;

    let end = end_idx.ok_or(anyhow!("end index not in range"))?;
    Ok(start..end)
}

pub fn result_tok_span_to_char_span(
    span: Range<usize>,
    toks: &[(
        Result<ExpectedToken<AbsoluteIndent>, ErrorToken>,
        Range<usize>,
    )],
) -> anyhow::Result<Range<usize>> {
    // our range is inclusive but chumsky span is exclusive
    let start = toks
        .get(span.start)
        .ok_or(anyhow!("start token not in stream: {:?}", span.start))?
        .1
        .start;
    let end = toks
        .get(span.end)
        .ok_or(anyhow!(
            "end token not in stream: {:?} len: {:?}",
            span.end,
            toks.len()
        ))?
        .1
        .end;
    Ok(Range {
        start,
        end: end - 1,
    })
}
