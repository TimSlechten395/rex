pub mod helper;
pub mod hover;
pub mod semantic_tokens;
pub mod update;

use std::sync::Arc;

use dashmap::DashMap;
use rex::sea_nodes::{NodeId, SeaOfNodes};
use rex::{ErrorToken, SpannedResultSugarExpr, Token};
use ropey::Rope;
use tokio::sync::Mutex;
use tower_lsp_server::lsp_types::*;
use tower_lsp_server::{Client, LanguageServer, jsonrpc};

use rex::lexer::Spanned;

use crate::semantic_tokens::{semantic_tokens_full, semantics};
use crate::update::update;

#[derive(Debug)]
pub struct Backend {
    pub client: Client,
    pub files: Arc<DashMap<Uri, Rope>>,
    pub tokens: Arc<DashMap<Uri, Vec<Spanned<Result<Token, ErrorToken>>>>>,
    // This is actually quite expensive to store like this
    pub sugar_asts: Arc<DashMap<Uri, SpannedResultSugarExpr>>,
    pub core_asts: Arc<DashMap<Uri, Vec<NodeId>>>,
    pub sea_of_nodes: Arc<Mutex<SeaOfNodes>>,
    pub diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "rex language server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            capabilities: ServerCapabilities {
                position_encoding: Some(PositionEncodingKind::UTF8),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: semantics(),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        // Show up in the client's log
        self.client
            .log_message(MessageType::INFO, "rex-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let file = params.text_document;

        if let Err(e) = update(&self, file.uri.clone(), file.text.clone()).await {
            self.client.log_message(MessageType::ERROR, e).await;
        }

        self.client
            .log_message(MessageType::INFO, "updated state")
            .await;

        // self.publish_todo_diagnostics(file.uri, file.text.clone())
        //     .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        // let mut file = match self.files.get_mut(&uri) {
        //     Some(f) => f,
        //     None => panic!(),
        // };

        // for change in params.content_changes.iter() {
        //     if let Some(range) = &change.range {
        //         let start =
        //             file.line_to_char(range.start.line as usize) + range.start.character as usize;
        //         let end = file.line_to_char(range.end.line as usize) + range.end.character as usize;
        //         file.remove(start..end);
        //         file.insert(start, &change.text);
        //     } else {
        //         *file = Rope::from_str(&change.text);
        //     }
        // }
        let change = params.content_changes.into_iter().next().unwrap();
        let file = change.text;

        if let Err(e) = update(&self, uri.clone(), file.to_string()).await {
            self.client.log_message(MessageType::ERROR, e).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let file = params.text_document;
        self.files.remove(&file.uri);
    }

    // Simple completion: always offer the same two items.
    async fn completion(
        &self,
        _params: CompletionParams,
    ) -> jsonrpc::Result<Option<CompletionResponse>> {
        Ok(Some(CompletionResponse::Array(vec![
            CompletionItem::new_simple("Hello".into(), "Greets the user".into()),
            CompletionItem::new_simple("Bye".into(), "Says farewell".into()),
        ])))
    }

    // Simple hover: show a fixed message.
    async fn hover(&self, params: HoverParams) -> jsonrpc::Result<Option<Hover>> {
        match hover::hover(self, params).await {
            Ok(hover) => Ok(hover),
            Err(e) => {
                self.client
                    .log_message(MessageType::ERROR, format!("Failed hover request: {:?}", e))
                    .await;
                Ok(None)
            }
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> jsonrpc::Result<Option<SemanticTokensResult>> {
        self.client
            .log_message(MessageType::INFO, "computing tokens")
            .await;
        let result = match semantic_tokens_full(&self, params.clone()).await {
            Ok(r) => Ok(r),
            Err(e) => {
                self.client
                    .log_message(
                        MessageType::INFO,
                        format!("failed to acquire semantic tokens: {:?}", e),
                    )
                    .await;
                Ok(None)
            }
        };

        self.client
            .log_message(MessageType::INFO, "done computing tokens")
            .await;

        result
    }
}
