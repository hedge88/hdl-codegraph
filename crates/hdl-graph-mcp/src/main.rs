use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: hdl-graph-mcp <project-dir> [include-dirs...]");
        std::process::exit(1);
    }

    let project = PathBuf::from(&args[1]);
    let include_dirs: Vec<String> = args[2..].to_vec();

    hdl_graph_mcp::run_server(&project, &include_dirs).await
}
