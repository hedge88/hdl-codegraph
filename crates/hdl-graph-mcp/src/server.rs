use std::collections::HashMap;

use anyhow::Result;
use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use rmcp::{
    handler::server::wrapper::Parameters,
    model::*,
    schemars,
    serve_server,
    tool, tool_handler, tool_router,
    transport::stdio,
    ServerHandler,
};
use serde::Deserialize;

use crate::tools;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct ProjectState {
    pub graph: InMemoryGraph,
    pub symbols: SymbolTable,
    pub file_map: HashMap<String, u64>,
}

// ---------------------------------------------------------------------------
// MCP Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct HdlMcpServer {
    state: std::sync::Arc<ProjectState>,
}

// -- Parameter types --

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchParams {
    /// Search pattern (supports glob: * and ?)
    query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct HierarchyParams {
    /// Module, class, package, or interface name
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CallersParams {
    /// Symbol name to find callers/references for
    symbol: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CalleesParams {
    /// Function or task name
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DriversParams {
    /// Signal name
    signal: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct UvmParams {
    /// Analysis type: "factory", "tlm", "config", or "hierarchy"
    analysis: String,
    /// Search term (type name, component name, or config path)
    query: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExploreParams {
    /// Module or class name to explore
    name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct StatsParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ImpactParams {
    /// Symbol name to analyze blast radius for
    symbol: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct NodeInfoParams {
    /// Symbol name to get details for
    symbol: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct FilesParams {
    /// Optional glob pattern to filter files (e.g. "*.sv", "rtl/*")
    pattern: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DefParams {
    /// Symbol name to look up the definition of
    symbol: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct InstParams {
    /// Module type name to find instantiations of
    module_type: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct CheckParams {}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExportParams {
    /// Export format: "scip", "json", or "markdown"
    format: String,
    /// Output file path
    output: String,
}

// -- Tool router --

#[tool_router]
impl HdlMcpServer {
    pub fn new(state: ProjectState) -> Self {
        Self {
            state: std::sync::Arc::new(state),
        }
    }

    #[tool(description = "Search symbols by name pattern. Supports glob wildcards (* and ?). Returns matching nodes with their kind and ID.")]
    async fn hdl_search(&self, Parameters(SearchParams { query }): Parameters<SearchParams>) -> String {
        tools::search::run(&self.state, &query)
    }

    #[tool(description = "Show the hierarchy tree of a module, class, package, or interface. Returns the containment tree with instances, ports, signals, and sub-blocks.")]
    async fn hdl_hierarchy(&self, Parameters(HierarchyParams { name }): Parameters<HierarchyParams>) -> String {
        tools::hierarchy::run(&self.state, &name)
    }

    #[tool(description = "Find all callers and references to a symbol. Shows which modules/functions reference, drive, call, instantiate, or connect to the given symbol.")]
    async fn hdl_callers(&self, Parameters(CallersParams { symbol }): Parameters<CallersParams>) -> String {
        tools::callers::run(&self.state, &symbol)
    }

    #[tool(description = "Find what a function/task calls. Shows outgoing call edges from the named function.")]
    async fn hdl_callees(&self, Parameters(CalleesParams { name }): Parameters<CalleesParams>) -> String {
        tools::callees::run(&self.state, &name)
    }

    #[tool(description = "Trace signal drivers and readers. Shows which always blocks, assignments, or modules drive or read the given signal.")]
    async fn hdl_drivers(&self, Parameters(DriversParams { signal }): Parameters<DriversParams>) -> String {
        tools::drivers::run(&self.state, &signal)
    }

    #[tool(description = "UVM analysis. analysis: 'factory' (factory registrations/overrides), 'tlm' (TLM port connections), 'config' (config_db set/get), 'hierarchy' (UVM class hierarchy). query: type name, component name, or config path (optional for hierarchy).")]
    async fn hdl_uvm(&self, Parameters(UvmParams { analysis, query }): Parameters<UvmParams>) -> String {
        tools::uvm::run(&self.state, &analysis, query.as_deref())
    }

    #[tool(description = "Explore a module or class in detail. Returns ports, signals, instances, always blocks, and connected modules in one call.")]
    async fn hdl_explore(&self, Parameters(ExploreParams { name }): Parameters<ExploreParams>) -> String {
        tools::explore::run(&self.state, &name)
    }

    #[tool(description = "Get graph statistics: node/edge counts broken down by kind (modules, signals, instances, UVM components, etc.).")]
    async fn hdl_stats(&self, Parameters(StatsParams {}): Parameters<StatsParams>) -> String {
        tools::stats::run(&self.state)
    }

    #[tool(description = "Analyze the blast radius of changing a symbol. Returns all downstream nodes that would be affected: direct references, transitive callers, connected signals, instantiated modules, and UVM overrides. Uses BFS up to depth 3.")]
    async fn hdl_impact(&self, Parameters(ImpactParams { symbol }): Parameters<ImpactParams>) -> String {
        tools::impact::run(&self.state, &symbol)
    }

    #[tool(description = "Get detailed information about a specific symbol: its kind, name, parent scope, type-specific attributes, source file, and all outgoing/incoming edges.")]
    async fn hdl_node(&self, Parameters(NodeInfoParams { symbol }): Parameters<NodeInfoParams>) -> String {
        tools::node_info::run(&self.state, &symbol)
    }

    #[tool(description = "List indexed files with per-file node/edge statistics. Optional glob pattern to filter (e.g. '*.sv', 'rtl/*'). Returns a table of files with module, class, instance, and signal counts.")]
    async fn hdl_files(&self, Parameters(FilesParams { pattern }): Parameters<FilesParams>) -> String {
        tools::files::run(&self.state, pattern.as_deref())
    }

    #[tool(description = "Find the definition location of a symbol (module, class, port, signal, function). Returns the kind, name, and node ID of matching definitions.")]
    async fn hdl_def(&self, Parameters(DefParams { symbol }): Parameters<DefParams>) -> String {
        tools::def::run(&self.state, &symbol)
    }

    #[tool(description = "Find all instantiations of a given module type across the project. Returns instance name, parent module, and file path for each match.")]
    async fn hdl_inst(&self, Parameters(InstParams { module_type }): Parameters<InstParams>) -> String {
        tools::inst::run(&self.state, &module_type)
    }

    #[tool(description = "Run graph consistency checks: dangling edges, unresolved module instances, orphan nodes, and unresolved class parents. Returns a summary of issues found.")]
    async fn hdl_check(&self, Parameters(CheckParams {}): Parameters<CheckParams>) -> String {
        tools::check::run(&self.state)
    }

    #[tool(description = "Export the code graph to a file. format: 'scip' (for Sourcegraph), 'json' (full graph), or 'markdown' (documentation). output: file path to write to.")]
    async fn hdl_export(&self, Parameters(ExportParams { format, output }): Parameters<ExportParams>) -> String {
        tools::export::run(&self.state, &format, &output)
    }
}

#[tool_handler(router = Self::tool_router())]
impl ServerHandler for HdlMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::default()
            .with_instructions("HDL Code Graph — Verilog/SystemVerilog/UVM code intelligence. Provides module hierarchy, signal flow, UVM factory/TLM/config_db analysis, and symbol search for HDL codebases.")
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub async fn run_server(project: &std::path::Path, include_dirs: &[String]) -> Result<()> {
    eprintln!("Building HDL code graph for {}...", project.display());

    let state = load_project(project, include_dirs)?;

    eprintln!(
        "Index complete: {} files, {} nodes, {} edges. Starting MCP server...",
        state.file_map.len(),
        state.graph.node_count(),
        state.graph.edge_count()
    );

    let server = HdlMcpServer::new(state);
    let service = serve_server(server, stdio()).await?;
    service.waiting().await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Project loading (ported from CLI)
// ---------------------------------------------------------------------------

fn load_project(project: &std::path::Path, include_dirs: &[String]) -> Result<ProjectState> {
    let mut scanner = hdl_graph_parse::FileScanner::new()?;
    let mut extractor = hdl_graph_parse::GraphExtractor::new();
    let mut graph = InMemoryGraph::new();
    let mut file_map = HashMap::new();

    let defines = collect_defines(project, include_dirs);

    for path in &collect_sv_files(project, include_dirs) {
        let rel = path
            .strip_prefix(project)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let preprocessed =
            hdl_graph_parse::preprocessor::preprocess(&source, &rel, &defines, include_dirs);
        let expanded = &preprocessed.expanded_source;

        let tree = scanner.parse_source(expanded);
        let (nodes, edges) = extractor.extract(&tree, expanded.as_bytes(), 0);
        let source_file_id = nodes.first().map(|n| n.id).unwrap_or(0);
        file_map.insert(rel, source_file_id);
        for n in nodes {
            graph.add_node(n).ok();
        }
        for e in edges {
            graph.add_edge(e).ok();
        }
    }

    Ok(ProjectState {
        graph,
        symbols: extractor.symbols,
        file_map,
    })
}

fn collect_sv_files(project: &std::path::Path, include_dirs: &[String]) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![project.to_path_buf()];
    for d in include_dirs {
        dirs.push(project.join(d));
    }
    let mut files = Vec::new();
    for dir in &dirs {
        if !dir.exists() {
            continue;
        }
        for entry in walkdir::WalkDir::new(dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if p.is_file() && is_sv_file(p) {
                files.push(p.to_path_buf());
            }
        }
    }
    files
}

fn is_sv_file(path: &std::path::Path) -> bool {
    match path.extension().and_then(|e| e.to_str()) {
        Some("sv") | Some("svh") | Some("svi") | Some("v") | Some("vh") | Some("pkg") => true,
        _ => false,
    }
}

fn collect_defines(project: &std::path::Path, _include_dirs: &[String]) -> HashMap<String, String> {
    let mut defines = HashMap::new();
    // UVM standard defines
    defines.insert("UVM_NO_DEPRECATED".to_string(), "1".to_string());
    defines.insert("UVM_OBJECT_MUST_HAVE_CONSTRUCTOR".to_string(), "1".to_string());
    // Try to load from config
    let config_path = project.join(".hdl-graph").join("config.toml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(config) = toml::from_str::<hdl_graph_core::ProjectConfig>(&content) {
            for d in &config.index.defines {
                if let Some((k, v)) = d.split_once('=') {
                    defines.insert(k.to_string(), v.to_string());
                } else {
                    defines.insert(d.to_string(), "1".to_string());
                }
            }
        }
    }
    defines
}
