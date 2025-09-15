use std::sync::Arc;

use dashmap::DashMap;
use rex::sea_nodes::SeaOfNodes;
use rex_lsp::Backend;
use tokio::sync::Mutex;
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    eprintln!("Starting server");
    let (service, socket) = LspService::new(|client| Backend {
        client,
        files: Arc::new(DashMap::new()),
        tokens: Arc::new(DashMap::new()),
        sugar_asts: Arc::new(DashMap::new()),
        core_asts: Arc::new(DashMap::new()),
        sea_of_nodes: Arc::new(Mutex::new(SeaOfNodes::new())),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;

    eprintln!("ended server");
}
