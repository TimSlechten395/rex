use anyhow::bail;
use either::Either::Left;
use rex::{
    data::{
        ast::{self, SpannedFixAst},
        expr::{self, ExprF, NamedDefs, SpannedExpr},
        tokens::{ErrorToken, GToken, InertToken, Spanned, ValidToken},
    },
    helper::map_index,
};
use tower_lsp_server::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities,
};

use crate::{Backend, helper::char_to_pos};

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
pub fn token_index<T>(token: GToken<T>) -> Option<u32> {
    match token {
        GToken::ValidToken(real_token) => match real_token {
            ValidToken::Type => Some(4),
            ValidToken::Number(_) => Some(3),
            ValidToken::Ident(_) => Some(2),
            ValidToken::String(_) => Some(12),
            ValidToken::Dot => Some(1),
            ValidToken::Colon => Some(1),
            ValidToken::Arrow => Some(1),
            ValidToken::DoubleArrow => Some(1),
            ValidToken::Pipe => Some(1),
            ValidToken::Comma => Some(1),
            ValidToken::Eq => Some(1),
            ValidToken::Fn => Some(0),
            ValidToken::Loop => Some(0),
            ValidToken::While => Some(0),
            ValidToken::For => Some(0),
            ValidToken::Break => Some(0),
            ValidToken::Let => Some(0),
            ValidToken::In => Some(0),
            _ => None,
        },
        GToken::InertToken(inert_token) => match inert_token {
            InertToken::Comment(_) => Some(11),
            InertToken::Space(_) => None,
            // InertToken::NewLine(_) => None,
        },
        GToken::ErrorToken(_) => Some(14),
    }
}
pub fn semantic_token<T>(
    ast: Vec<SpannedFixAst>,
    expr: NamedDefs,
    token: Spanned<GToken<T>>,
    token_idx: Option<usize>,
) -> Option<u32> {
    match token.0 {
        GToken::ValidToken(real_token) => {
            use ValidToken::*;
            match real_token {
                Type => Some(4),
                Number(_) => Some(3),
                Ident(_) => {
                    let Some(node_path) =
                        ast::search_list(ast.clone(), token_idx?).into_iter().next()
                    else {
                        return None;
                    };

                    let Some(node_path) = expr::search_defs(expr.clone(), node_path.clone())
                        .into_iter()
                        .next()
                    else {
                        return None;
                    };

                    let node = expr::traverse_defs(expr, node_path.clone());
                    match node {
                        Ok(node) => {
                            let Left(node) = node else {
                                return None;
                            };
                            match node.0.0 {
                                ExprF::Var { .. } => Some(2),
                                ExprF::Lambda { .. } | ExprF::Pi { .. } => Some(8),
                                _ => None,
                            }
                        }
                        Err(_) => None,
                    }
                }
                String(_) => Some(12),
                Dot => Some(1),
                Colon => Some(1),
                Arrow => Some(1),
                DoubleArrow => Some(1),
                Pipe => Some(1),
                Comma => Some(1),
                Eq => Some(1),
                // Token::Car => todo!(),
                // Token::Cdr => todo!(),
                // Token::Cons => todo!(),
                Fn => Some(0),
                Loop => Some(0),
                While => Some(0),
                For => Some(0),
                Break => Some(0),
                Let => Some(0),
                In => Some(0),
                Def => Some(0),
                _ => None,
            }
        }
        GToken::InertToken(inert_token) => match inert_token {
            InertToken::Comment(_) => Some(11),
            InertToken::Space(_) => None,
            // InertToken::NewLine(_) => None,
        },
        GToken::ErrorToken(_) => Some(14),
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

    let Some(ast) = backend.asts.get(&uri) else {
        bail!("Failed to get ast")
    };

    let Some(expr) = backend.named_exprs.get(&uri) else {
        bail!("Failed to get ast")
    };

    let tokens = tokens.clone();

    let mut semantic_tokens = Vec::new();
    let mut prev_line = 0;
    let mut prev_column = 0;

    for (i, token) in tokens.iter().enumerate() {
        let ast_index = map_index(&tokens, i);

        let token_type = semantic_token(ast.clone(), expr.clone(), token.clone(), ast_index);
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
