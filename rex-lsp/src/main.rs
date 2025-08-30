use dashmap::DashMap;
use rex::sea_nodes::SeaOfNodes;
use rex_lsp::Backend;
use tokio::sync::Mutex;
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        files: DashMap::new(),
        asts: DashMap::new(),
        tokens: DashMap::new(),
        sea_of_nodes: Mutex::new(SeaOfNodes::new()),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
