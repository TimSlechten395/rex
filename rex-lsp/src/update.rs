use chumsky::Parser;
use rex::{Context, sea_nodes::lower_expr};
use tower_lsp_server::lsp_types::Uri;

use crate::Backend;

pub async fn update(backend: &Backend, uri: Uri, text: String) {
    let mut sea = backend.sea_of_nodes.lock().await;
    {
        let mut tokens = Vec::new();
        let lexer = rex::lexer();
        let parser = rex::parser();

        if let Ok(spanned_tokens) = lexer.parse(&text).into_result() {
            backend.tokens.insert(uri.clone(), spanned_tokens.clone());
            tokens.extend(spanned_tokens.into_iter().map(|t| t.0));
            if let Ok(sugar_ast) = parser.parse(&tokens).into_result() {
                let mut ctx = Context::new();
                let ast_tree = rex::desugar(sugar_ast, &mut ctx);
                let ast = lower_expr(&ast_tree, &mut sea);
                backend.asts.insert(uri.clone(), ast.clone());
            }
        }
    }
}
