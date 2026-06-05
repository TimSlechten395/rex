use std::fmt::Display;
use std::ops::{Range, RangeInclusive};

use anyhow::anyhow;
use chumsky::span::SimpleSpan;
use num_bigint::BigUint;
use thiserror::Error;

pub type Spanned<T> = (T, SimpleSpan);

pub type Token = GToken<AbsoluteIndent>;

#[derive(Debug, Clone, PartialEq)]
pub enum GToken<Tab> {
    ValidToken(ValidToken<Tab>),
    InertToken(InertToken),
    ErrorToken(ErrorToken),
}

impl ValidToken<RelativeIndent> {
    pub fn from_generic(tok: ValidToken<usize>) -> Self {
        use ValidToken::*;
        match tok {
            Newline(_) => {
                panic!("Use tabs_to_indent to convert Newline(usize) into Newline(RelativeIndent)")
            }

            Ident(s) => Ident(s),
            Number(n) => Number(n),
            String(s) => String(s),

            Def => Def,
            Fn => Fn,
            Dot => Dot,
            Colon => Colon,
            SemiColon => SemiColon,
            Arrow => Arrow,
            DoubleArrow => DoubleArrow,
            Pipe => Pipe,
            Star => Star,
            Comma => Comma,
            LParen => LParen,
            RParen => RParen,
            LBrace => LBrace,
            RBrace => RBrace,
            LBracket => LBracket,
            RBracket => RBracket,
            Hashtag => Hashtag,
            Type => Type,
            Mod => Mod,
            Eq => Eq,
            Assign => Assign,

            Loop => Loop,
            While => While,
            For => For,
            Break => Break,
            Let => Let,
            In => In,
        }
    }
}

pub fn extract_good_toks<T>(toks: Vec<GToken<T>>) -> Vec<ValidToken<T>> {
    toks.into_iter()
        .filter_map(|x| match x {
            GToken::ValidToken(real_token) => Some(real_token),
            GToken::InertToken(_) => None,
            GToken::ErrorToken(_) => None,
        })
        .collect()
}

pub enum RelativeIndent {
    Indent(usize),
    Dedent(usize),
    Same,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AbsoluteIndent(pub usize);

// TODO: misplacedTabs does not belong here since its part of the second step
#[derive(Debug, Clone, Error, PartialEq)]
pub enum ErrorToken {
    #[error("invalid char {0:?}")]
    InvalidChar(char),

    #[error("misplaced tabs {0:?}")]
    MisplacedTabs(usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InertToken {
    Comment(String),
    Space(usize),
}

// generic over tab_kind
#[derive(Debug, Clone, PartialEq)]
pub enum ValidToken<Indent> {
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
    Star,     // '*'
    Comma,    // ','
    LParen,   // '('
    RParen,   // ')'
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    Hashtag,  // #

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
    Mod,
}

impl Display for ValidToken<AbsoluteIndent> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidToken::Ident(name) => write!(f, "{}", name),
            ValidToken::Number(num) => write!(f, "{}", num),
            ValidToken::String(s) => write!(f, "\"{}\"", s),

            ValidToken::Fn => write!(f, "fn"),
            ValidToken::Def => write!(f, "Def"),
            ValidToken::Dot => write!(f, "."),
            ValidToken::Colon => write!(f, ":"),
            ValidToken::SemiColon => write!(f, ";"),
            ValidToken::Arrow => write!(f, "->"),
            ValidToken::DoubleArrow => write!(f, "=>"),
            ValidToken::Pipe => write!(f, "|"),
            ValidToken::Star => write!(f, "*"),
            ValidToken::Comma => write!(f, ","),
            ValidToken::LParen => write!(f, "("),
            ValidToken::RParen => write!(f, ")"),
            ValidToken::LBrace => write!(f, "{{"),
            ValidToken::RBrace => write!(f, "}}"),
            ValidToken::LBracket => write!(f, "["),
            ValidToken::RBracket => write!(f, "]"),
            ValidToken::Hashtag => write!(f, "#"),

            ValidToken::Type => write!(f, "Type"),
            ValidToken::Mod => write!(f, "Mod"),

            ValidToken::Eq => write!(f, "="),
            ValidToken::Assign => write!(f, "="),
            ValidToken::Newline(AbsoluteIndent(n)) => {
                let tabs = "\t".repeat(*n);
                write!(f, "\n{tabs}")
            }

            ValidToken::Loop => write!(f, "loop"),
            ValidToken::While => write!(f, "while"),
            ValidToken::For => write!(f, "for"),
            ValidToken::Break => write!(f, "break"),
            ValidToken::Let => write!(f, "let"),
            ValidToken::In => write!(f, "in"),
        }
    }
}

// maps span from Token to ExpectedToken spans are inclusive
pub fn tok_span_to_result_tok_span(
    span: RangeInclusive<usize>,
    full_tokens: &[Spanned<GToken<AbsoluteIndent>>],
) -> anyhow::Result<RangeInclusive<usize>> {
    let mut ok_count = 0;
    let mut start_idx = None;
    let mut end_idx = None;
    for (i, t) in full_tokens.iter().enumerate() {
        if matches!(t.0, GToken::ValidToken(_)) {
            if ok_count == *span.start() {
                start_idx = Some(i)
            }
            if ok_count == *span.end() {
                end_idx = Some(i);
                break;
            }
            ok_count += 1;
        }
    }

    let start = start_idx.ok_or(anyhow!("start index not in range"))?;

    let end = end_idx.ok_or(anyhow!(
        "end index not in range: {:?} len was {:?}",
        span.end(),
        ok_count
    ))?;
    Ok(start..=end)
}

// tok_span is inclusive char_span is exclusive
pub fn result_tok_span_to_char_span(
    span: RangeInclusive<usize>,
    toks: &[Spanned<GToken<AbsoluteIndent>>],
) -> anyhow::Result<Range<usize>> {
    let start = toks
        .get(*span.start())
        .ok_or(anyhow!("start token not in stream: {:?}", span.start()))?
        .1
        .start;
    let end = toks
        .get(*span.end())
        .ok_or(anyhow!(
            "end token not in stream: {:?} len: {:?}",
            span.end(),
            toks.len()
        ))?
        .1
        .end;
    Ok(Range { start, end: end })
}
