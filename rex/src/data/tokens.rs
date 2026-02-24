use std::fmt::Display;
use std::ops::Range;

use anyhow::anyhow;
use chumsky::span::SimpleSpan;
use num_bigint::BigUint;
use thiserror::Error;

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
            Token::Hashtag => Token::Hashtag,
            Token::Type => Token::Type,
            Token::Mod => Token::Mod,
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
#[derive(Debug, Clone, Error)]
pub enum ErrorToken {
    #[error("invalid char {0:?}")]
    InvalidChar(char),

    #[error("misplaced tabs {0:?}")]
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
            Token::Hashtag => write!(f, "#"),

            Token::Type => write!(f, "Type"),
            Token::Mod => write!(f, "Mod"),

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

// maps span from Token to ExpectedToken spans are inclusive
pub fn tok_span_to_result_tok_span(
    span: Range<usize>,
    full_tokens: &[Spanned<ExpectedToken<AbsoluteIndent>>],
) -> anyhow::Result<Range<usize>> {
    let mut ok_count = 0;
    let mut start_idx = None;
    let mut end_idx = None;
    for (i, t) in full_tokens.iter().enumerate() {
        if matches!(t.0, ExpectedToken::Token(_)) {
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

    let end = end_idx.ok_or(anyhow!(
        "end index not in range: {:?} len was {:?}",
        span.end,
        ok_count
    ))?;
    Ok(start..end)
}

// tok_span is inclusive char_span is exclusive
pub fn result_tok_span_to_char_span(
    span: Range<usize>,
    toks: &[Spanned<ExpectedToken<AbsoluteIndent>>],
) -> anyhow::Result<Range<usize>> {
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
