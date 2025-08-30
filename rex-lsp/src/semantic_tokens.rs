use anyhow::anyhow;
use rex::Token;
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
pub fn token_index(token: &Token) -> u32 {
    match token {
        Token::Type => 4,
        Token::Number(_) => 3,
        Token::Ident(_) => 2,
        Token::Bool(_) => 13,
        Token::String(_) => 12,
        Token::Lambda => 0,
        Token::Dot => 1,
        Token::Colon => 1,
        Token::SemiColon => 1,
        Token::Arrow => 1,
        Token::Pipe => 1,
        Token::Star => 1,
        Token::Comma => 1,
        Token::LParen => 1,
        Token::RParen => 1,
        Token::LBrace => 1,
        Token::RBrace => 1,
        Token::LBracket => 1,
        Token::RBracket => 1,
        Token::Comment(_) => 11,
        // Token::Car => todo!(),
        // Token::Cdr => todo!(),
        Token::Eq => 0,
        // Token::Cons => todo!(),
        Token::Loop => 0,
        Token::While => 0,
        Token::For => 0,
        Token::Break => 0,
        Token::Let => 0,
        Token::In => 0,
        _ => todo!(),
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
) -> tower_lsp_server::jsonrpc::Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri;
    let Some(text) = backend.files.get(&uri) else {
        return Err(jsonrpc::Error::internal_error());
    };
    let text = &text.to_string();

    let Some(tokens) = backend.tokens.get(&uri) else {
        return Err(jsonrpc::Error::internal_error());
    };
    let tokens = tokens.clone();

    // TODO: this is very fast
    fn byte_offset_to_line(text: &str, byte_offset: usize) -> usize {
        text[..byte_offset].bytes().filter(|&b| b == b'\n').count()
    }
    fn byte_offset_to_col(text: &str, byte_offset: usize) -> usize {
        let line_start = text[..byte_offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
        byte_offset - line_start // in bytes
    }

    let mut semantic_tokens = Vec::new();
    let mut prev_line = 0;
    let mut prev_start = 0;

    for token in tokens {
        let line = byte_offset_to_line(text, token.1.start);
        let start = byte_offset_to_col(text, token.1.start);

        let delta_line = (line - prev_line) as u32;
        let delta_start = if delta_line == 0 {
            start - prev_start
        } else {
            start
        } as u32;

        let length = (token.1.end - token.1.start) as u32;

        let token_type = token_index(&token.0);

        semantic_tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: 0,
        });

        prev_line = line;
        prev_start = start;
    }

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: semantic_tokens,
    })))
}
