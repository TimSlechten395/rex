use std::collections::HashMap;

use chumsky::Parser;
use dashmap::DashMap;
use rex::sea_nodes::{NodeId, SeaOfNodes};
use ropey::Rope;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use rex::{Token, lexer};

#[derive(Debug)]
struct Backend {
    client: Client,
    files: DashMap<Uri, Rope>,
    asts: DashMap<Uri, NodeId>,
    sea_of_nodes: SeaOfNodes,
}

// TODO: restructure Token type so this works better
fn token_index(token: &Token) -> u32 {
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

fn semantic_tokens_legend() -> SemanticTokensLegend {
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
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ], // fill with standard modifiers if needed
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "rex language server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                // Advertise a couple of features so the client knows what to ask for.
                position_encoding: Some(PositionEncodingKind::UTF8),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Show up in the client's log
        self.client
            .log_message(MessageType::INFO, "my-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let file = params.text_document;

        self.files
            .insert(file.uri.clone(), Rope::from_str(&file.text));
        self.publish_todo_diagnostics(file.uri, file.text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        let mut file = match self.files.get_mut(&uri) {
            Some(f) => f,
            None => panic!(),
        };

        for change in params.content_changes.iter() {
            if let Some(range) = &change.range {
                let start =
                    file.line_to_char(range.start.line as usize) + range.start.character as usize;
                let end = file.line_to_char(range.end.line as usize) + range.end.character as usize;
                file.remove(start..end);
                file.insert(start, &change.text);
            } else {
                *file = Rope::from_str(&change.text);
            }
        }

        if let Some(change) = params.content_changes.into_iter().last() {
            self.publish_todo_diagnostics(uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let file = params.text_document;
        self.files.remove(&file.uri);
    }

    // Simple completion: always offer the same two items.
    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".into(), "Greets the user".into()),
            CompletionItem::new_simple("Bye".into(), "Says farewell".into()),
        ])))
    }

    // Simple hover: show a fixed message.
    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "You're hovering over something!".into(),
            )),
            range: None,
        }))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let text = &self.files.get(&uri).unwrap().to_string();
        let tokens = lexer().parse(text).into_result().unwrap();

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
}

impl Backend {
    async fn publish_todo_diagnostics(&self, uri: Uri, text: String) {
        let mut diagnostics = Vec::new();
        for (i, line) in text.lines().enumerate() {
            if let Some(col) = line.find("TODO") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position::new(i as u32, col as u32),
                        end: Position::new(i as u32, (col + 4) as u32),
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some("my-lsp".into()),
                    message: "Found TODO".into(),
                    ..Default::default()
                });
            }
        }
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        files: DashMap::new(),
        asts: DashMap::new(),
        sea_of_nodes: SeaOfNodes::new(),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
