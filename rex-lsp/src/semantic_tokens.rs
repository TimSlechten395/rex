use anyhow::bail;
use rex::{ErrorToken, RealToken, SpannedResultSugarExpr, Token, lexer::Spanned};
use tower_lsp_server::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities,
};

use crate::{
    Backend,
    helper::{char_to_pos, map_index},
};

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
pub fn token_index(token: &Result<Token, ErrorToken>) -> Option<u32> {
    match token {
        Ok(tok) => match tok {
            Token::RealToken(real_token) => {
                match real_token {
                    RealToken::Type => Some(4),
                    RealToken::Number(_) => Some(3),
                    RealToken::Ident(_) => Some(2),
                    RealToken::String(_) => Some(12),
                    RealToken::Dot => Some(1),
                    RealToken::Colon => Some(1),
                    RealToken::Arrow => Some(1),
                    RealToken::DoubleArrow => Some(1),
                    RealToken::Pipe => Some(1),
                    RealToken::Comma => Some(1),
                    RealToken::Eq => Some(1),
                    // Token::Car => todo!(),
                    // Token::Cdr => todo!(),
                    // Token::Cons => todo!(),
                    RealToken::Fn => Some(0),
                    RealToken::Loop => Some(0),
                    RealToken::While => Some(0),
                    RealToken::For => Some(0),
                    RealToken::Break => Some(0),
                    RealToken::Let => Some(0),
                    RealToken::In => Some(0),
                    _ => None,
                }
            }
            Token::InertToken(inert_token) => match inert_token {
                rex::InertToken::Comment(_) => Some(11),
                rex::InertToken::Space(_) => None,
                rex::InertToken::NewLine(_) => None,
            },
        },
        Err(_) => Some(14),
    }
}
pub fn semantic_token(
    ast: SpannedResultSugarExpr,
    token: Spanned<Result<Token, ErrorToken>>,
    token_idx: Option<usize>,
) -> Option<u32> {
    match token.0 {
        Ok(tok) => match tok {
            Token::RealToken(real_token) => {
                match real_token {
                    RealToken::Type => Some(4),
                    RealToken::Number(_) => Some(3),
                    RealToken::Ident(_) => {
                        let node_path = ast.clone().search(token_idx?)?;
                        let node = ast.traverse(node_path).ok()?;
                        match node.0.0 {
                            Ok(node) => match node {
                                rex::SugarExpr::Var(_) => Some(2),
                                rex::SugarExpr::MultiLambda(items, _) => Some(8),
                                rex::SugarExpr::MultiPi(items, _) => Some(8),
                                _ => None,
                            },
                            Err(_) => None,
                        }
                    }
                    RealToken::String(_) => Some(12),
                    RealToken::Dot => Some(1),
                    RealToken::Colon => Some(1),
                    RealToken::Arrow => Some(1),
                    RealToken::DoubleArrow => Some(1),
                    RealToken::Pipe => Some(1),
                    RealToken::Comma => Some(1),
                    RealToken::Eq => Some(1),
                    // Token::Car => todo!(),
                    // Token::Cdr => todo!(),
                    // Token::Cons => todo!(),
                    RealToken::Fn => Some(0),
                    RealToken::Loop => Some(0),
                    RealToken::While => Some(0),
                    RealToken::For => Some(0),
                    RealToken::Break => Some(0),
                    RealToken::Let => Some(0),
                    RealToken::In => Some(0),
                    _ => None,
                }
            }
            Token::InertToken(inert_token) => match inert_token {
                rex::InertToken::Comment(_) => Some(11),
                rex::InertToken::Space(_) => None,
                rex::InertToken::NewLine(_) => None,
            },
        },
        Err(_) => Some(14),
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

    let Some(sugar_ast) = backend.sugar_asts.get(&uri) else {
        bail!("Failed to get ast")
    };

    let tokens = tokens.clone();

    let mut semantic_tokens = Vec::new();
    let mut prev_line = 0;
    let mut prev_column = 0;

    for (i, token) in tokens.iter().enumerate() {
        let ast_index = map_index(&tokens, i);

        let token_type = semantic_token(sugar_ast.clone(), token.clone(), ast_index);
        if let Some(token_type) = token_type {
            let start = token.1.start;
            let end = token.1.end;

            let (line, column) = char_to_pos(&text, start);

            // Yes delta_start that make so much sense. why was indexing not enough?
            let delta_line = (line - prev_line) as u32;

            let delta_start = if delta_line == 0 {
                column - prev_column
            } else {
                column
            } as u32;

            let length = (end - start) as u32;

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
    }

    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: semantic_tokens,
    })))
}
