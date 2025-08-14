use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
}

impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Advertise a couple of features so the client knows what to ask for.
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                ..Default::default()
            },
            ..Default::default()
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

    // Publish diagnostics on open/change: warn if a line contains "TODO".
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.publish_todo_diagnostics(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.publish_todo_diagnostics(params.text_document.uri, change.text)
                .await;
        }
    }
}

impl Backend {
    async fn publish_todo_diagnostics(&self, uri: Url, text: String) {
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
    // stdio transport; most editors can launch LSP servers this way
    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
