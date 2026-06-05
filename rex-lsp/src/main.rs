use std::sync::Arc;

use dashmap::DashMap;
use rex_lsp::Backend;
use tokio::sync::Mutex;
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    eprintln!("Started server");
    let (service, socket) = LspService::new(|client| Backend {
        client,
        files: Arc::new(DashMap::new()),
        tokens: Arc::new(DashMap::new()),
        asts: Arc::new(DashMap::new()),
        named_exprs: Arc::new(DashMap::new()),
        exprs: Arc::new(DashMap::new()),
        //sea_of_nodes: Arc::new(Mutex::new(SeaOfNodes::new())),
        diagnostics: Arc::new(Mutex::new(Vec::new())),
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:27632")
        .await
        .unwrap();

    let (stream, _) = listener.accept().await.unwrap();
    let (read, write) = tokio::io::split(stream);

    Server::new(read, write, socket).serve(service).await;

    eprintln!("ended server");
}
