use tower_lsp::{LspService, Server};

mod backend;

pub use backend::Backend;

pub async fn run_server(stdin: tokio::io::Stdin, stdout: tokio::io::Stdout) {
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
