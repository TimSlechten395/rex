use anyhow::{anyhow, bail};
use rex::{ErrorToken, Token};
use tower_lsp_server::{
    jsonrpc,
    lsp_types::{
        SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
        SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
        SemanticTokensParams, SemanticTokensResult, SemanticTokensServerCapabilities,
    },
};

use crate::Backend;

pub fn semantics() -> Option<SemanticTokensServerCapabilities> {
    Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
        SemanticTokensOptions {
            legend: semantic_tokens_legend(),
            full: Some(SemanticTokensFullOptions::Bool(true)),
            ..Default::default()
        },
    ))
}

// TODO: restructure Token type so this works better
pub fn token_index(token: &Result<Token, ErrorToken>) -> u32 {
    match token {
        Ok(tok) => match tok {
            Token::Type => 4,
            Token::Number(_) => 3,
            Token::Ident(_) => 2,
            Token::String(_) => 12,
            Token::Dot => 1,
            Token::Colon => 1,
            Token::SemiColon => 1,
            Token::Arrow => 1,
            Token::DoubleArrow => 1,
            Token::Pipe => 1,
            Token::Star => 1,
            Token::Comma => 1,
            Token::LParen => 1,
            Token::RParen => 1,
            Token::LBrace => 1,
            Token::RBrace => 1,
            Token::LBracket => 1,
            Token::RBracket => 1,
            // Token::Car => todo!(),
            // Token::Cdr => todo!(),
            Token::Eq => 1,
            // Token::Cons => todo!(),
            Token::Fn => 0,
            Token::Loop => 0,
            Token::While => 0,
            Token::For => 0,
            Token::Break => 0,
            Token::Let => 0,
            Token::In => 0,
            _ => 0,
        },
        Err(e) => match e {
            ErrorToken::Comment(_) => 11,
            ErrorToken::ErrorToken(_) => 14,
            ErrorToken::Space(_) => 15,
            ErrorToken::NewLine(_) => 15,
        },
    }
}

pub fn semantic_tokens_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::TYPE,
            SemanticTokenType::ENUM,
            SemanticTokenType::STRUCT,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::new("error"),
            // SemanticTokenType::PROPERTY.as_str().to_string(),
            // SemanticTokenType::ENUM_MEMBER.as_str().to_string(),
            // SemanticTokenType::EVENT.as_str().to_string(),
            // SemanticTokenType::FUNCTION.as_str().to_string(),
            // SemanticTokenType::METHOD.as_str().to_string(),
            // SemanticTokenType::MACRO.as_str().to_string(),
            // SemanticTokenType::DECORATOR.as_str().to_string(),
        ],
        //TODO: Why is documentation here?
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ], // fill with standard modifiers if needed
    }
}

pub async fn semantic_tokens_full(
    backend: &Backend,
    params: SemanticTokensParams,
) -> anyhow::Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri;
    let Some(text) = backend.files.get(&uri) else {
        bail!("Failed to get text")
    };

    let Some(tokens) = backend.tokens.get(&uri) else {
        bail!("Failed to get tokens")
    };

    // let Some(sugar_ast) = backend.sugar_asts.get(&uri) else {
    //     bail!("Failed to get ast")
    // };

    let tokens = tokens.clone();

    let mut semantic_tokens = Vec::new();
    let mut prev_line = 0;
    let mut prev_column = 0;

    for token in tokens {
        let start = token.1.start;
        let end = token.1.end;

        let line = text.char_to_line(start);
        let line_start = text.line_to_char(line);
        let column = start - line_start;

        // Yes delta_start that make so much sense. why was indexing not enough?
        let delta_line = (line - prev_line) as u32;

        let delta_start = if delta_line == 0 {
            column - prev_column
        } else {
            column
        } as u32;

        let length = (end - start) as u32;

        let token_type = token_index(&token.0);

        semantic_tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = line;
        prev_column = column;
    }

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: semantic_tokens,
    })))
}
