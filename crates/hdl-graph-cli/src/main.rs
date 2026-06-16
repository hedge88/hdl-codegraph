use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::time::Duration;
use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use sha2::{Sha256, Digest};
use notify::{Watcher, RecursiveMode, Event, EventKind};

// ---------------------------------------------------------------------------
// CLI argument structure
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(
    name = "hdl-graph",
    version,
    about = "HDL Code Graph — Verilog / SystemVerilog / UVM code intelligence",
    long_about = "hdl-graph parses HDL source files (.sv, .svh, .v, .vh) into a queryable\n\
                  code graph.  It supports definition lookup, hierarchy inspection, signal\n\
                  driver tracing, UVM factory/TLM/config analysis, SCIP export, and an\n\
                  embedded LSP server for editor integration."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, env = "HDL_GRAPH_DB", global = true,
          help = "Path to the graph database file (default: in-memory)")]
    db: Option<PathBuf>,

    #[arg(long, default_value = ".", global = true,
          help = "Root directory of the HDL project")]
    project: PathBuf,

    #[arg(long, global = true,
          help = "Additional directories to scan for .sv/.svh/.v/.vh files")]
    include_dirs: Vec<String>,

    #[arg(long, global = true,
          help = "Path to the UVM library home directory")]
    uvm_home: Option<PathBuf>,

    #[arg(long, global = true,
          help = "Preprocessor defines (e.g. --defines UVM_NO_DPI)")]
    defines: Vec<String>,

    #[arg(long, default_value_t = num_cpus(), global = true,
          help = "Number of parallel indexing jobs")]
    jobs: usize,

    #[arg(short, long, global = true,
          help = "Enable debug-level logging")]
    verbose: bool,

    #[arg(long, default_value = "text", global = true,
          help = "Output format: text (human-readable) or json (machine-readable)")]
    format: OutputFormat,
}

#[derive(Clone)]
enum OutputFormat { Text, Json }

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!("invalid format: {s}")),
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new .hdl-graph project with default config.toml
    Init {
        /// Target directory (default: current directory)
        dir: Option<PathBuf>,
    },
    /// Parse HDL sources and build the code graph index
    Index {
        /// Watch for file changes and incrementally re-index
        #[arg(long)]
        watch: bool,
    },
    /// Query the code graph for definitions, references, and hierarchies
    #[command(subcommand)]
    Query(QueryCommands),
    /// UVM-specific analysis: factory, TLM, config DB, and type hierarchy
    #[command(subcommand)]
    Uvm(UvmCommands),
    /// Start the LSP language server on stdio for editor integration
    Watch,
    /// Search for symbols by name pattern (case-insensitive substring match)
    Search {
        /// Pattern to match against symbol names
        pattern: String,
    },
    /// Display graph statistics: node/edge counts and breakdown by kind
    Stats,
    /// Run cross-reference consistency checks on the graph
    Check {
        /// CI mode: exit with non-zero status on failure
        #[arg(long)]
        ci: bool,
    },
    /// Export the code graph to a file (SCIP, JSON, or Markdown)
    #[command(subcommand)]
    Export(ExportCommands),
    /// Print the hdl-graph version string
    Version,
    /// List indexed files with per-file statistics
    Files {
        /// Optional glob pattern to filter files (e.g. "*.sv", "rtl/*")
        pattern: Option<String>,
    },
}

#[derive(Subcommand)]
enum QueryCommands {
    /// Find the definition location of a symbol (module, class, port, signal, function)
    Def {
        /// Symbol name to look up
        symbol: String,
        /// Optional scope to narrow the search (e.g. file:line)
        scope: Option<String>,
    },
    /// Find all references to a symbol across the project
    Refs {
        /// Symbol name to search for
        symbol: String,
        /// Optional scope to narrow the search
        scope: Option<String>,
    },
    /// Show the module / class instantiation hierarchy tree
    Hierarchy {
        /// Top-level module or class name
        name: String,
    },
    /// Show the call graph for a function or task
    Calls {
        /// Function or task name
        name: String,
    },
    /// Trace signal drivers (writes) and readers (reads)
    Drivers {
        /// Signal name to trace
        signal: String,
    },
    /// Find all instantiations of a given module type across the project
    Inst {
        /// Module type name (e.g. fifo, apb_slave)
        module_type: String,
    },
    /// Explore a module or class in detail: ports, signals, instances, always blocks
    Explore {
        /// Module, class, package, or interface name
        name: String,
    },
    /// Analyze the blast radius of changing a symbol (BFS up to depth 3)
    Impact {
        /// Symbol name to analyze
        symbol: String,
    },
    /// Get detailed information about a specific symbol
    Node {
        /// Symbol name
        symbol: String,
    },
}

#[derive(Subcommand)]
enum UvmCommands {
    /// Show factory registrations, overrides, and create calls for a UVM type
    Factory {
        /// UVM type name (e.g. my_driver, my_scoreboard)
        type_name: String,
    },
    /// Show TLM port connections for a UVM component
    Tlm {
        /// UVM component name (e.g. env, agent, scoreboard)
        component: String,
    },
    /// Show uvm_config_db set/get operations matching a field path
    Config {
        /// Field path to match (supports * wildcard, e.g. "*.driver.vif")
        path: String,
    },
    /// Show the UVM class inheritance hierarchy (extends tree)
    Hierarchy,
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export in SCIP JSON format for Sourcegraph / GitHub Code Search
    Scip {
        /// Output file path for the SCIP index
        output: PathBuf,
    },
    /// Export full graph as JSON (nodes, edges, files, metadata)
    Json {
        /// Output file path for the JSON export
        output: PathBuf,
    },
    /// Export as human-readable Markdown documentation
    Markdown {
        /// Output path (single file, or directory with --per-module)
        output: PathBuf,
        /// Generate one .md file per module/class instead of a single file
        #[arg(long)]
        per_module: bool,
    },
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4)
}

// ---------------------------------------------------------------------------
// Project state
// ---------------------------------------------------------------------------

struct ProjectState {
    graph: InMemoryGraph,
    symbols: SymbolTable,
    file_map: HashMap<String, u64>,
}

/// Full state of an indexed project, used for incremental updates.
struct IndexedProject {
    graph: InMemoryGraph,
    scanner: hdl_graph_parse::FileScanner,
    extractor: hdl_graph_parse::GraphExtractor,
    file_map: HashMap<String, u64>,
    /// Maps file path -> list of node IDs allocated for that file.
    file_node_ids: HashMap<String, Vec<u64>>,
    /// Maps file path -> list of edges derived from that file.
    file_edges: HashMap<String, Vec<Edge>>,
    /// Maps file_id -> SHA-256 content hash.
    file_hashes: HashMap<u64, String>,
    /// Maps file path -> old tree-sitter tree for incremental parsing.
    file_trees: HashMap<String, hdl_graph_parse::Tree>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    match &cli.command {
        Commands::Init { dir } => {
            let default_dir = PathBuf::from(".");
            let target = dir.as_deref().unwrap_or(&default_dir);
            cmd_init(target)?;
            Ok(())
        }
        Commands::Index { watch } => {
            if *watch {
                cmd_watch(&cli.project, &cli.include_dirs)?;
            } else {
                cmd_index(&cli.project, &cli.include_dirs)?;
            }
            Ok(())
        }
        Commands::Query(q) => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            match q {
                QueryCommands::Def { symbol, scope } => {
                    cmd_def(&state, symbol, scope.as_deref())
                }
                QueryCommands::Refs { symbol, scope } => {
                    cmd_refs(&state, symbol, scope.as_deref())
                }
                QueryCommands::Hierarchy { name } => cmd_hierarchy(&state, name),
                QueryCommands::Calls { name } => cmd_calls(&state, name),
                QueryCommands::Drivers { signal } => cmd_drivers(&state, signal),
                QueryCommands::Inst { module_type } => cmd_inst(&state, module_type),
                QueryCommands::Explore { name } => cmd_explore(&state, name),
                QueryCommands::Impact { symbol } => cmd_impact(&state, symbol),
                QueryCommands::Node { symbol } => cmd_node(&state, symbol),
            }?;
            Ok(())
        }
        Commands::Uvm(u) => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            match u {
                UvmCommands::Factory { type_name } => {
                    cmd_uvm_factory(&state, type_name)?;
                }
                UvmCommands::Tlm { component } => {
                    cmd_uvm_tlm(&state, component)?;
                }
                UvmCommands::Config { path } => {
                    cmd_uvm_config(&state, path)?;
                }
                UvmCommands::Hierarchy => {
                    cmd_uvm_hierarchy(&state)?;
                }
            }
            Ok(())
        }
        Commands::Watch => {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();
            println!("Starting hdl-graph LSP server on stdio...");
            hdl_graph_lsp::run_server(stdin, stdout).await;
            Ok(())
        }
        Commands::Search { pattern } => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            cmd_search(&state, pattern)?;
            Ok(())
        }
        Commands::Stats => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            cmd_stats(&state)?;
            Ok(())
        }
        Commands::Check { ci } => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            cmd_check(&state, *ci)?;
            Ok(())
        }
        Commands::Export(e) => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            match e {
                ExportCommands::Scip { output } => {
                    hdl_graph_query::ScipExporter::export(
                        &state.graph, &state.symbols, &state.file_map, output
                    )?;
                    println!("Exported SCIP index to {}", output.display());
                }
                ExportCommands::Json { output } => {
                    hdl_graph_query::JsonExporter::export(
                        &state.graph, &state.symbols, &state.file_map, output
                    )?;
                    println!("Exported JSON graph to {}", output.display());
                }
                ExportCommands::Markdown { output, per_module } => {
                    let mode = if *per_module {
                        hdl_graph_query::MarkdownMode::PerModule
                    } else {
                        hdl_graph_query::MarkdownMode::Single
                    };
                    hdl_graph_query::MarkdownExporter::export(
                        &state.graph, &state.symbols, &state.file_map, output, mode
                    )?;
                    println!("Exported Markdown to {}", output.display());
                }
            }
            Ok(())
        }
        Commands::Version => {
            println!("hdl-graph {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Files { pattern } => {
            let state = load_or_build(&cli.project, &cli.include_dirs)?;
            cmd_files(&state, pattern.as_deref())?;
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn cmd_init(dir: &Path) -> anyhow::Result<()> {
    let cfg = dir.join(".hdl-graph");
    std::fs::create_dir_all(&cfg)?;
    let config = hdl_graph_core::ProjectConfig::default();
    let toml_str =
        toml::to_string_pretty(&config).map_err(|e| anyhow::anyhow!("config: {e}"))?;
    std::fs::write(cfg.join("config.toml"), toml_str)?;
    println!("Initialized HDL Code Graph project in {}", dir.display());
    Ok(())
}

fn cmd_index(project: &Path, include_dirs: &[String]) -> anyhow::Result<()> {
    let mut scanner = hdl_graph_parse::FileScanner::new()?;
    let mut extractor = hdl_graph_parse::GraphExtractor::new();
    let mut graph = InMemoryGraph::new();
    let mut file_map: HashMap<String, u64> = HashMap::new();

    let files = collect_sv_files(project, include_dirs);
    if files.is_empty() {
        println!("No .sv/.svh/.v files found");
        return Ok(());
    }

    println!("Indexing {} files...", files.len());
    for path in &files {
        let rel = path
            .strip_prefix(project)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        match scanner.parse_file(path) {
            Ok(tree) => {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), 0);
                // The first node returned is always the SourceFile node
                let source_file_id = nodes.first().map(|n| n.id).unwrap_or(0);
                file_map.insert(rel.clone(), source_file_id);
                for n in &nodes {
                    graph.add_node(n.clone()).ok();
                }
                for e in &edges {
                    graph.add_edge(e.clone()).ok();
                }
                println!(
                    "  {}: {} nodes, {} edges",
                    rel,
                    nodes.len(),
                    edges.len()
                );
            }
            Err(e) => eprintln!("  {}: ERROR — {e}", rel),
        }
    }

    println!(
        "\nGraph built: {} nodes, {} edges in {} files",
        graph.node_count(),
        graph.edge_count(),
        file_map.len()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Incremental indexing helpers
// ---------------------------------------------------------------------------

/// Compute a SHA-256 hex hash of byte content.
fn compute_file_hash(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Return true if `path` is an HDL source file.
fn is_sv_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e, "sv" | "svh" | "v" | "vh"))
}

/// Build the full graph for a project and return the indexed state.
fn index_project(project: &Path, include_dirs: &[String]) -> anyhow::Result<IndexedProject> {
    let mut scanner = hdl_graph_parse::FileScanner::new()?;
    let mut extractor = hdl_graph_parse::GraphExtractor::new();
    let mut graph = InMemoryGraph::new();
    let mut file_map: HashMap<String, u64> = HashMap::new();
    let mut file_node_ids: HashMap<String, Vec<u64>> = HashMap::new();
    let mut file_edges: HashMap<String, Vec<Edge>> = HashMap::new();
    let mut file_hashes: HashMap<u64, String> = HashMap::new();
    let mut file_trees: HashMap<String, hdl_graph_parse::Tree> = HashMap::new();

    let files = collect_sv_files(project, include_dirs);
    if files.is_empty() {
        println!("No .sv/.svh/.v files found");
        return Ok(IndexedProject {
            graph,
            scanner,
            extractor,
            file_map,
            file_node_ids,
            file_edges,
            file_hashes,
            file_trees,
        });
    }

    println!("Indexing {} files...", files.len());
    for path in &files {
        let rel = path
            .strip_prefix(project)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let tree = scanner.parse_source(&content);
                let (nodes, edges) = extractor.extract(&tree, content.as_bytes(), 0);
                let source_file_id = nodes.first().map(|n| n.id).unwrap_or(0);
                let node_ids: Vec<u64> = nodes.iter().map(|n| n.id).collect();

                let hash = compute_file_hash(content.as_bytes());

                file_map.insert(rel.clone(), source_file_id);
                file_node_ids.insert(rel.clone(), node_ids);
                file_edges.insert(rel.clone(), edges.clone());
                file_hashes.insert(source_file_id, hash);
                file_trees.insert(rel.clone(), tree);

                for n in &nodes {
                    graph.add_node(n.clone()).ok();
                }
                for e in &edges {
                    graph.add_edge(e.clone()).ok();
                }
                println!(
                    "  {}: {} nodes, {} edges",
                    rel,
                    nodes.len(),
                    edges.len()
                );
            }
            Err(e) => eprintln!("  {}: ERROR — {e}", rel),
        }
    }

    println!(
        "\nGraph built: {} nodes, {} edges in {} files",
        graph.node_count(),
        graph.edge_count(),
        file_map.len()
    );

    Ok(IndexedProject {
        graph,
        scanner,
        extractor,
        file_map,
        file_node_ids,
        file_edges,
        file_hashes,
        file_trees,
    })
}

/// Incrementally re-index changed files.
///
/// For each changed file:
/// 1. Read new content, compute hash
/// 2. If hash matches stored hash, skip (no change)
/// 3. Use tree-sitter's incremental parse if an old tree is available
/// 4. Extract new nodes/edges and produce a ChangeSet
/// 5. Apply the ChangeSet to the in-memory graph
/// 6. Update metadata maps
fn cmd_index_incremental(
    project: &Path,
    _include_dirs: &[String],
    state: &mut IndexedProject,
    changed_files: &[PathBuf],
) -> anyhow::Result<()> {
    for path in changed_files {
        let rel = path
            .strip_prefix(project)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Read content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  {}: read error — {e}", rel);
                continue;
            }
        };

        // Compute hash to detect actual changes
        let new_hash = compute_file_hash(content.as_bytes());
        let file_id = *state.file_map.get(&rel).unwrap_or(&0);
        if file_id != 0 {
            if let Some(stored) = state.file_hashes.get(&file_id) {
                if *stored == new_hash {
                    continue; // Content unchanged
                }
            }
        }

        // Parse with incremental tree-sitter, reusing the old tree if available.
        // This gives O(log n) parsing for typical edits via tree-sitter.
        let old_tree = state.file_trees.remove(&rel);
        let tree = state.scanner.parse_source_incremental(
            &content,
            old_tree.as_ref(),
        );

        // Get old node IDs and edges for this file
        let old_node_ids = state.file_node_ids.remove(&rel).unwrap_or_default();
        let old_edges = state.file_edges.remove(&rel).unwrap_or_default();

        // Extract changeset: full rebuild of this file's subgraph
        let changeset = state.extractor.extract_changeset(
            &tree,
            content.as_bytes(),
            file_id,
            &old_node_ids,
            &old_edges,
        );

        // Capture new node IDs before applying (extract_changeset calls
        // extract internally so the new IDs are already allocated)
        let new_node_ids: Vec<u64> = changeset.added_nodes.iter().map(|(id, _)| *id).collect();
        let new_edges: Vec<Edge> = changeset.added_edges.clone();
        let new_file_id = changeset
            .added_nodes
            .first()
            .map(|(id, _)| *id)
            .unwrap_or(0);

        // Apply to graph
        changeset.apply_to(&mut state.graph)?;

        // Update metadata
        if new_file_id != 0 {
            state.file_map.insert(rel.clone(), new_file_id);
            state.file_hashes.insert(new_file_id, new_hash);
        }
        state.file_node_ids.insert(rel.clone(), new_node_ids.clone());
        state.file_edges.insert(rel.clone(), new_edges);
        state.file_trees.insert(rel.clone(), tree);

        println!(
            "  updated {}: {} nodes, {} edges (removed {} old nodes)",
            rel,
            new_node_ids.len(),
            changeset.added_edges.len(),
            old_node_ids.len(),
        );
    }

    Ok(())
}

/// Watch the project directory for file changes and incrementally re-index.
fn cmd_watch(project: &Path, include_dirs: &[String]) -> anyhow::Result<()> {
    println!("Building initial index...");
    let mut state = index_project(project, include_dirs)?;

    println!("\nWatching for file changes...");
    println!("  (press Ctrl+C to stop)\n");

    // Set up notify file watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)
        .map_err(|e| anyhow::anyhow!("Failed to create file watcher: {e}"))?;

    // Watch the project root recursively
    watcher
        .watch(project, RecursiveMode::Recursive)
        .map_err(|e| anyhow::anyhow!("Failed to watch {}: {e}", project.display()))?;

    // Also watch each include directory
    for dir in include_dirs {
        let dir_path = project.join(dir);
        if dir_path.exists() {
            watcher
                .watch(&dir_path, RecursiveMode::Recursive)
                .map_err(|e| anyhow::anyhow!("Failed to watch {}: {e}", dir_path.display()))?;
        }
    }

    // Small debounce: collect events over a 100ms window
    let mut pending: HashMap<PathBuf, std::time::Instant> = HashMap::new();
    const DEBOUNCE_MS: u64 = 100;

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        for path in event.paths {
                            if is_sv_file(&path) {
                                pending.insert(path, std::time::Instant::now());
                            }
                        }
                    }
                    _ => {}
                }

                // Check for debounced file changes
                let now = std::time::Instant::now();
                let ready: Vec<PathBuf> = pending
                    .iter()
                    .filter(|(_, t)| now.duration_since(**t) >= Duration::from_millis(DEBOUNCE_MS))
                    .map(|(p, _)| p.clone())
                    .collect();

                if !ready.is_empty() {
                    for path in &ready {
                        pending.remove(path);
                        // Re-read project-relative paths for changed files
                        println!("Change detected: {}", path.display());
                    }
                    if let Err(e) = cmd_index_incremental(project, include_dirs, &mut state, &ready) {
                        eprintln!("  incremental index error: {e}");
                    }
                }
            }
            Ok(Err(e)) => {
                eprintln!("watch error: {e}");
            }
            Err(std::sync::mpsc::RecvError) => {
                // Channel closed — watcher stopped
                println!("File watcher stopped.");
                break;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

fn collect_sv_files(project: &Path, include_dirs: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let dirs: Vec<PathBuf> = if include_dirs.is_empty() {
        vec![project.to_path_buf()]
    } else {
        include_dirs.iter().map(|d| project.join(d)).collect()
    };

    for dir in &dirs {
        if dir.exists() {
            for entry in walkdir::WalkDir::new(dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "sv" | "svh" | "v" | "vh") {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    if files.is_empty() {
        for entry in walkdir::WalkDir::new(project)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if matches!(ext, "sv" | "svh" | "v" | "vh") {
                    files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    files
}

// ---------------------------------------------------------------------------
// Graph loading / building
// ---------------------------------------------------------------------------

fn load_or_build(project: &Path, include_dirs: &[String]) -> anyhow::Result<ProjectState> {
    let mut scanner = hdl_graph_parse::FileScanner::new()?;
    let mut extractor = hdl_graph_parse::GraphExtractor::new();
    let mut graph = InMemoryGraph::new();
    let mut file_map = HashMap::new();

    for path in &collect_sv_files(project, include_dirs) {
        let rel = path
            .strip_prefix(project)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();
        if let Ok(tree) = scanner.parse_file(path) {
            let source = std::fs::read_to_string(path).unwrap_or_default();
            let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), 0);
            // The first node is always the SourceFile node
            let source_file_id = nodes.first().map(|n| n.id).unwrap_or(0);
            file_map.insert(rel, source_file_id);
            for n in nodes {
                graph.add_node(n).ok();
            }
            for e in edges {
                graph.add_edge(e).ok();
            }
        }
    }

    Ok(ProjectState {
        graph,
        symbols: extractor.symbols,
        file_map,
    })
}

// ---------------------------------------------------------------------------
// Query commands
// ---------------------------------------------------------------------------

fn cmd_def(state: &ProjectState, symbol: &str, _scope: Option<&str>) -> anyhow::Result<()> {
    let mut found = false;

    for (_file, fid) in &state.file_map {
        let outgoing = state.graph.get_outgoing(*fid)?;
        for edge in &outgoing {
            if edge.edge_type == EdgeType::Contains {
                if let Ok(Some(node)) = state.graph.get_node(edge.target) {
                    // Check for module/class/package name match
                    let top_name = match &node.kind {
                        NodeKind::Module { name }
                        | NodeKind::Class { name, .. }
                        | NodeKind::Package { name } => {
                            Some(state.symbols.resolve(*name))
                        }
                        _ => None,
                    };
                    if let Some(Some(n)) = top_name {
                        if n == symbol {
                            let kind = node_kind_str(&node.kind);
                            println!("{} {}  (file_id: {}, node_id: {})", kind, symbol, fid, node.id);
                            found = true;
                        }
                    }

                    // Search within this container for ports/signals/instances
                    if let Ok(children) = state.graph.get_outgoing(edge.target) {
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                let name = match &child.kind {
                                    NodeKind::ModulePort { name, .. }
                                    | NodeKind::SignalDecl { name, .. } => {
                                        state.symbols.resolve(*name).map(|s| s.to_string())
                                    }
                                    NodeKind::ModuleInstance { name, .. } => {
                                        state.symbols.resolve(*name).map(|s| s.to_string())
                                    }
                                    _ => None,
                                };
                                if name.as_deref() == Some(symbol) {
                                    let kind = node_kind_str(&child.kind);
                                    println!(
                                        "{} {}  (node_id: {})",
                                        kind, symbol, child.id
                                    );
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found {
        println!("Definition not found: {symbol}");
    }
    Ok(())
}

fn cmd_refs(state: &ProjectState, symbol: &str, _scope: Option<&str>) -> anyhow::Result<()> {
    // Step 1: Find all node IDs matching the symbol name
    let target_ids: Vec<u64> = state
        .graph
        .all_nodes()
        .iter()
        .filter(|n| node_name_str(&state.symbols, &n.kind).as_deref() == Some(symbol))
        .map(|n| n.id)
        .collect();

    if target_ids.is_empty() {
        println!("No references found for: {}", symbol);
        return Ok(());
    }

    let target_set: std::collections::HashSet<u64> = target_ids.iter().copied().collect();
    let mut found = false;

    // Step 2: Use get_incoming on target nodes for direct reference lookup
    for &tid in &target_ids {
        if let Ok(incoming) = state.graph.get_incoming(tid) {
            for edge in &incoming {
                if !is_ref_edge(edge.edge_type) {
                    continue;
                }
                if let Ok(Some(src)) = state.graph.get_node(edge.source) {
                    let kind = node_kind_str(&src.kind);
                    let src_name = node_name_str(&state.symbols, &src.kind)
                        .unwrap_or_else(|| format!("#{}", src.id));
                    let edge_label = edge.edge_type.name();
                    println!("{} {} -> {} -> {}", kind, src_name, edge_label, symbol);
                    found = true;
                }
            }
        }
    }

    // Step 3: Also scan outgoing edges from non-target nodes
    // (incoming edges on targets already covers edges between target nodes)
    for node in state.graph.all_nodes() {
        if target_set.contains(&node.id) {
            continue; // already covered by incoming scan
        }
        if let Ok(edges) = state.graph.get_outgoing(node.id) {
            for edge in &edges {
                if !is_ref_edge(edge.edge_type) {
                    continue;
                }
                if !target_set.contains(&edge.target) {
                    continue;
                }
                let kind = node_kind_str(&node.kind);
                let src_name = node_name_str(&state.symbols, &node.kind)
                    .unwrap_or_else(|| format!("#{}", node.id));
                let edge_label = edge.edge_type.name();
                println!("{} {} -> {} -> {}", kind, src_name, edge_label, symbol);
                found = true;
            }
        }
    }

    if !found {
        println!("No references found for: {}", symbol);
    }
    Ok(())
}

fn is_ref_edge(et: EdgeType) -> bool {
    matches!(
        et,
        EdgeType::References
            | EdgeType::Drives
            | EdgeType::Extends
            | EdgeType::Calls
            | EdgeType::ConfigSets
            | EdgeType::ConfigGets
            | EdgeType::Instantiates
            | EdgeType::Connects
            | EdgeType::FactoryRegisters
            | EdgeType::FactoryOverrides
            | EdgeType::TLMBinds
    )
}

fn node_name_str(symbols: &SymbolTable, kind: &NodeKind) -> Option<String> {
    match kind {
        NodeKind::Module { name }
        | NodeKind::Class { name, .. }
        | NodeKind::Package { name }
        | NodeKind::Interface { name }
        | NodeKind::Function { name, .. }
        | NodeKind::SignalDecl { name, .. }
        | NodeKind::ModulePort { name, .. }
        | NodeKind::ModuleInstance { name, .. }
        | NodeKind::Property { name }
        | NodeKind::VariableRef { name }
        | NodeKind::Method { name, .. }
        | NodeKind::TLMPort { name, .. }
        | NodeKind::SequenceDecl { name }
        | NodeKind::PropertyDecl { name }
        | NodeKind::CoverGroup { name, .. }
        | NodeKind::CoverPoint { name, .. }
        | NodeKind::Modport { name }
        | NodeKind::ConfigBlock { name } => symbols.resolve(*name).map(|s| s.to_string()),
        NodeKind::CallSite { target } => symbols.resolve(*target).map(|s| s.to_string()),
        NodeKind::DPIImport { function_name } => {
            symbols.resolve(*function_name).map(|s| s.to_string())
        }
        NodeKind::ConfigDBSet { field } | NodeKind::ConfigDBGet { field } => {
            symbols.resolve(*field).map(|s| s.to_string())
        }
        NodeKind::FactoryReg { type_name, .. } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryCreate { type_name } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryOverride { original_type, .. } => {
            symbols.resolve(*original_type).map(|s| s.to_string())
        }
        _ => None,
    }
}

fn cmd_hierarchy(state: &ProjectState, name: &str) -> anyhow::Result<()> {
    for (_file, fid) in &state.file_map {
        let outgoing = state.graph.get_outgoing(*fid)?;
        for edge in &outgoing {
            if edge.edge_type == EdgeType::Contains {
                if let Ok(Some(node)) = state.graph.get_node(edge.target) {
                    if matches!(&node.kind, NodeKind::Module { name: n }
                        if state.symbols.resolve(*n) == Some(name))
                    {
                        println!("{}", name);
                        print_tree(&state.graph, &state.symbols, edge.target, 1);
                        return Ok(());
                    }
                }
            }
        }
    }
    println!("Module not found: {name}");
    Ok(())
}

fn print_tree(graph: &InMemoryGraph, symbols: &SymbolTable, node_id: u64, depth: usize) {
    if let Ok(edges) = graph.get_outgoing(node_id) {
        for e in &edges {
            if let Ok(Some(child)) = graph.get_node(e.target) {
                let label = match &child.kind {
                    NodeKind::Module { name } => {
                        format!("module {}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    NodeKind::ModuleInstance { name, module_type } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let t = symbols.resolve(*module_type).unwrap_or("?");
                        format!("{}: {}", n, t)
                    }
                    NodeKind::SignalDecl { name, .. } => {
                        format!("{}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    NodeKind::ModulePort { name, .. } => {
                        format!("port {}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    NodeKind::AlwaysBlock { .. } => "always".to_string(),
                    NodeKind::Assignment => "assign".to_string(),
                    NodeKind::Class { name, .. } => {
                        format!("class {}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    NodeKind::Package { name } => {
                        format!("package {}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    NodeKind::Function { name, .. } => {
                        format!("function {}", symbols.resolve(*name).unwrap_or("?"))
                    }
                    _ => continue,
                };
                println!("{}{} {}", "  ".repeat(depth), "|--", label);
                print_tree(graph, symbols, e.target, depth + 1);
            }
        }
    }
}

fn cmd_calls(state: &ProjectState, name: &str) -> anyhow::Result<()> {
    let mut results = Vec::new();

    for node in state.graph.all_nodes() {
        // CallSite nodes targeting this name
        if let NodeKind::CallSite { target } = &node.kind {
            if state.symbols.resolve(*target) == Some(name) {
                if let Ok(incoming) = state.graph.get_incoming(node.id) {
                    for ie in &incoming {
                        if ie.edge_type == EdgeType::Contains {
                            if let Ok(Some(parent)) = state.graph.get_node(ie.source) {
                                let pname = node_name_str(&state.symbols, &parent.kind)
                                    .unwrap_or_else(|| format!("node #{}", parent.id));
                                results.push(format!("  called in {}", pname));
                            }
                        }
                    }
                }
            }
        }

        // Calls edges
        if let Ok(outgoing) = state.graph.get_outgoing(node.id) {
            for edge in &outgoing {
                if edge.edge_type == EdgeType::Calls {
                    if let Ok(Some(target)) = state.graph.get_node(edge.target) {
                        if node_name_str(&state.symbols, &target.kind).as_deref() == Some(name) {
                            let sname = node_name_str(&state.symbols, &node.kind)
                                .unwrap_or_else(|| format!("node #{}", node.id));
                            results.push(format!("  called from {}", sname));
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        println!("No call sites found for: {}", name);
    } else {
        println!("Call sites for {}:", name);
        for r in &results {
            println!("{}", r);
        }
    }

    Ok(())
}

fn cmd_drivers(state: &ProjectState, signal: &str) -> anyhow::Result<()> {
    let mut drivers = Vec::new();
    let mut readers = Vec::new();

    for (_file, fid) in &state.file_map {
        let outgoing = match state.graph.get_outgoing(*fid) {
            Ok(o) => o,
            Err(_) => continue,
        };
        for edge in &outgoing {
            if edge.edge_type != EdgeType::Contains {
                continue;
            }
            find_drivers_recursive(
                &state.graph,
                &state.symbols,
                edge.target,
                signal,
                &mut drivers,
                &mut readers,
            );
        }
    }

    println!("Drivers of {}:", signal);
    if drivers.is_empty() {
        println!("  (none)");
    } else {
        for d in &drivers {
            println!("  {}", d);
        }
    }

    println!("Readers of {}:", signal);
    if readers.is_empty() {
        println!("  (none)");
    } else {
        for r in &readers {
            println!("  {}", r);
        }
    }

    Ok(())
}

fn find_drivers_recursive(
    graph: &InMemoryGraph,
    symbols: &SymbolTable,
    node_id: u64,
    signal: &str,
    drivers: &mut Vec<String>,
    readers: &mut Vec<String>,
) {
    let edges = match graph.get_outgoing(node_id) {
        Ok(e) => e,
        Err(_) => return,
    };

    for edge in &edges {
        if matches!(edge.edge_type, EdgeType::Drives | EdgeType::References) {
            if let Ok(Some(target)) = graph.get_node(edge.target) {
                if node_name_matches(symbols, &target.kind, signal) {
                    if let Ok(Some(source)) = graph.get_node(node_id) {
                        let label = node_label(&source, symbols);
                        match edge.edge_type {
                            EdgeType::Drives => drivers.push(label),
                            EdgeType::References => readers.push(label),
                            _ => {}
                        }
                    }
                }
            }
        }
        if edge.edge_type == EdgeType::Contains {
            find_drivers_recursive(graph, symbols, edge.target, signal, drivers, readers);
        }
    }
}

fn node_name_matches(symbols: &SymbolTable, kind: &NodeKind, target: &str) -> bool {
    let name = match kind {
        NodeKind::Module { name }
        | NodeKind::Class { name, .. }
        | NodeKind::Package { name }
        | NodeKind::Interface { name }
        | NodeKind::Function { name, .. }
        | NodeKind::SignalDecl { name, .. }
        | NodeKind::ModulePort { name, .. }
        | NodeKind::ModuleInstance { name, .. }
        | NodeKind::Property { name }
        | NodeKind::VariableRef { name }
        | NodeKind::CallSite { target: name }
        | NodeKind::Method { name, .. }
        | NodeKind::TLMPort { name, .. } => symbols.resolve(*name),
        _ => None,
    };
    name == Some(target)
}

fn cmd_inst(state: &ProjectState, module_type: &str) -> anyhow::Result<()> {
    let mut found = false;
    println!("Instantiations of {}:", module_type);
    for (_file, fid) in &state.file_map {
        let outgoing = state.graph.get_outgoing(*fid)?;
        for e in &outgoing {
            if e.edge_type == EdgeType::Contains {
                if let Ok(Some(module_node)) = state.graph.get_node(e.target) {
                    if let Ok(children) = state.graph.get_outgoing(e.target) {
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                if let NodeKind::ModuleInstance {
                                    name,
                                    module_type: mt,
                                } = &child.kind
                                {
                                    let t = state.symbols.resolve(*mt).unwrap_or("?");
                                    if t == module_type {
                                        let n = state.symbols.resolve(*name).unwrap_or("?");
                                        let mn = match &module_node.kind {
                                            NodeKind::Module { name } => {
                                                state.symbols.resolve(*name).unwrap_or("?")
                                            }
                                            _ => "?",
                                        };
                                        println!(
                                            "  {} (in module {}, file {})",
                                            n, mn, _file
                                        );
                                        found = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if !found {
        println!("  (none found)");
    }
    Ok(())
}

fn cmd_search(state: &ProjectState, pattern: &str) -> anyhow::Result<()> {
    println!("Search results for '{}':", pattern);
    for (_file, fid) in &state.file_map {
        let outgoing = state.graph.get_outgoing(*fid)?;
        for e in &outgoing {
            if e.edge_type == EdgeType::Contains {
                if let Ok(Some(node)) = state.graph.get_node(e.target) {
                    let label = node_label(&node, &state.symbols).to_lowercase();
                    if label.contains(&pattern.to_lowercase()) {
                        println!("  {} (id: {})", label, node.id);
                    }
                    // Search children too
                    if let Ok(children) = state.graph.get_outgoing(e.target) {
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                let label =
                                    node_label(&child, &state.symbols).to_lowercase();
                                if label.contains(&pattern.to_lowercase()) {
                                    println!("  {} (id: {})", label, child.id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn cmd_stats(state: &ProjectState) -> anyhow::Result<()> {
    let mut mods = 0;
    let mut sigs = 0;
    let mut insts = 0;
    let mut ports = 0;
    let mut classes = 0;
    let mut packages = 0;
    let mut funcs = 0;
    let mut always = 0;
    let mut assigns = 0;

    for (_file, fid) in &state.file_map {
        if let Ok(edges) = state.graph.get_outgoing(*fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(node)) = state.graph.get_node(e.target) {
                        match &node.kind {
                            NodeKind::Module { .. } => mods += 1,
                            NodeKind::Class { .. } => classes += 1,
                            NodeKind::Package { .. } => packages += 1,
                            _ => {}
                        }
                        if let Ok(kids) = state.graph.get_outgoing(e.target) {
                            for ke in &kids {
                                if let Ok(Some(kid)) = state.graph.get_node(ke.target) {
                                    match &kid.kind {
                                        NodeKind::SignalDecl { .. } => sigs += 1,
                                        NodeKind::ModuleInstance { .. } => insts += 1,
                                        NodeKind::ModulePort { .. } => ports += 1,
                                        NodeKind::AlwaysBlock { .. } => always += 1,
                                        NodeKind::Assignment => assigns += 1,
                                        NodeKind::Function { .. } => funcs += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Graph Statistics:");
    println!("  Files:     {}", state.file_map.len());
    println!("  Nodes:     {}", state.graph.node_count());
    println!("  Edges:     {}", state.graph.edge_count());
    println!("  Modules:   {mods}");
    println!("  Ports:     {ports}");
    println!("  Signals:   {sigs}");
    println!("  Instances: {insts}");
    println!("  Always:    {always}");
    println!("  Assigns:   {assigns}");
    println!("  Classes:   {classes}");
    println!("  Packages:  {packages}");
    println!("  Functions: {funcs}");
    Ok(())
}

// ---------------------------------------------------------------------------
// Check command
// ---------------------------------------------------------------------------

fn cmd_check(state: &ProjectState, ci: bool) -> anyhow::Result<()> {
    println!("Graph Consistency Check");
    println!("=======================");
    println!();

    let all_nodes = state.graph.all_nodes();
    let node_ids: std::collections::HashSet<u64> = all_nodes.iter().map(|n| n.id).collect();

    // Collect all edges by iterating outgoing from every node
    let mut all_edges: Vec<Edge> = Vec::new();
    for node in &all_nodes {
        if let Ok(outgoing) = state.graph.get_outgoing(node.id) {
            all_edges.extend(outgoing);
        }
    }

    let mut total_issues: usize = 0;

    // (a) Dangling edges: verify both source and target nodes exist
    let mut dangling_details: Vec<String> = Vec::new();
    for edge in &all_edges {
        if !node_ids.contains(&edge.source) || !node_ids.contains(&edge.target) {
            dangling_details.push(format!(
                "  edge {} -> {} ({})",
                edge.source,
                edge.target,
                edge.edge_type.name()
            ));
        }
    }
    let dangling_count = dangling_details.len();
    total_issues += dangling_count;
    println!("Dangling edges: {dangling_count}");
    for d in &dangling_details {
        println!("{d}");
    }

    // (b) Unresolved module instantiations
    let defined_modules: std::collections::HashSet<String> = all_nodes
        .iter()
        .filter_map(|n| {
            if let NodeKind::Module { name } = &n.kind {
                state.symbols.resolve(*name).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    let mut unresolved_instances: Vec<String> = Vec::new();
    for node in &all_nodes {
        if let NodeKind::ModuleInstance { name, module_type } = &node.kind {
            let mt_name = state.symbols.resolve(*module_type).unwrap_or("?");
            if !defined_modules.contains(mt_name) {
                let inst_name = state.symbols.resolve(*name).unwrap_or("?");
                let parent_module = find_containing_module(state, node.id);
                unresolved_instances.push(format!(
                    "  {}: {} (in module {})",
                    inst_name, mt_name, parent_module
                ));
            }
        }
    }
    let unresolved_count = unresolved_instances.len();
    total_issues += unresolved_count;
    println!("Unresolved instances: {unresolved_count}");
    for u in &unresolved_instances {
        println!("{u}");
    }

    // (c) Orphan nodes: no incoming AND no outgoing edges (excluding SourceFile)
    let mut orphan_details: Vec<String> = Vec::new();
    for node in &all_nodes {
        if matches!(&node.kind, NodeKind::SourceFile) {
            continue;
        }
        let has_outgoing = state
            .graph
            .get_outgoing(node.id)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        let has_incoming = state
            .graph
            .get_incoming(node.id)
            .map(|v| !v.is_empty())
            .unwrap_or(false);
        if !has_outgoing && !has_incoming {
            let label = node_label(node, &state.symbols);
            orphan_details.push(format!("  {} (id: {})", label, node.id));
        }
    }
    let orphan_count = orphan_details.len();
    total_issues += orphan_count;
    println!("Orphan nodes: {orphan_count}");
    for o in &orphan_details {
        println!("{o}");
    }

    // (d) Unresolved class parents
    let defined_classes: std::collections::HashSet<String> = all_nodes
        .iter()
        .filter_map(|n| {
            if let NodeKind::Class { name, .. } = &n.kind {
                state.symbols.resolve(*name).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    let mut unresolved_parents: Vec<String> = Vec::new();
    for node in &all_nodes {
        if let NodeKind::Class { name, parent } = &node.kind {
            if let Some(parent_sym) = parent {
                let parent_name = state.symbols.resolve(*parent_sym).unwrap_or("?");
                if !defined_classes.contains(parent_name) {
                    let class_name = state.symbols.resolve(*name).unwrap_or("?");
                    unresolved_parents.push(format!(
                        "  {}: parent '{}' not found",
                        class_name, parent_name
                    ));
                }
            }
        }
    }
    let unresolved_parent_count = unresolved_parents.len();
    total_issues += unresolved_parent_count;
    println!("Unresolved parents: {unresolved_parent_count}");
    for p in &unresolved_parents {
        println!("{p}");
    }

    // Summary
    println!();
    if total_issues == 0 {
        println!("All checks passed");
    } else {
        println!("Summary: {total_issues} issue(s) found");
        if ci {
            println!();
            println!("WARNING: CI mode — {total_issues} issue(s) detected.");
            println!("CI exit behavior not yet implemented; returning Ok(()) for now.");
        }
    }

    Ok(())
}

/// Find the name of the containing module for a given node by tracing
/// incoming Contains edges up the tree.
fn find_containing_module(state: &ProjectState, node_id: u64) -> String {
    if let Ok(incoming) = state.graph.get_incoming(node_id) {
        for edge in &incoming {
            if edge.edge_type == EdgeType::Contains {
                if let Ok(Some(parent)) = state.graph.get_node(edge.source) {
                    if let NodeKind::Module { name } = &parent.kind {
                        return state.symbols.resolve(*name).unwrap_or("?").to_string();
                    }
                    // Recurse upward if the parent is not a module
                    return find_containing_module(state, edge.source);
                }
            }
        }
    }
    "?".to_string()
}

// ---------------------------------------------------------------------------
// Commands ported from MCP
// ---------------------------------------------------------------------------

fn cmd_explore(state: &ProjectState, name: &str) -> anyhow::Result<()> {
    use hdl_graph_core::helpers::*;

    // Find the target node
    let target = state.graph.all_nodes().into_iter().find(|n| {
        matches!(&n.kind,
            NodeKind::Module { .. } | NodeKind::Class { .. }
            | NodeKind::Package { .. } | NodeKind::Interface { .. })
            && node_name_from_kind(&n.kind, &state.symbols).as_deref() == Some(name)
    });

    let target = match target {
        Some(n) => n,
        None => {
            println!("Module/Class not found: {}", name);
            return Ok(());
        }
    };

    println!("# {}", name);

    // Find source file
    if let Some(file) = find_file_for_node(&state.graph, &state.file_map, target.id) {
        println!("**File:** {}", file);
    }

    // Classify children
    let mut ports = Vec::new();
    let mut signals = Vec::new();
    let mut instances = Vec::new();
    let mut functions = Vec::new();
    let mut always_count = 0u32;
    let mut assign_count = 0u32;

    if let Ok(edges) = state.graph.get_outgoing(target.id) {
        for e in &edges {
            if e.edge_type != EdgeType::Contains { continue; }
            if let Ok(Some(child)) = state.graph.get_node(e.target) {
                match &child.kind {
                    NodeKind::ModulePort { name: n, direction } => {
                        let pn = state.symbols.resolve(*n).unwrap_or("?");
                        ports.push(format!("  - {} ({:?})", pn, direction));
                    }
                    NodeKind::SignalDecl { name: n, kind } => {
                        let sn = state.symbols.resolve(*n).unwrap_or("?");
                        signals.push(format!("  - {} ({:?})", sn, kind));
                    }
                    NodeKind::ModuleInstance { name: n, module_type } => {
                        let iname = state.symbols.resolve(*n).unwrap_or("?");
                        let mtype = state.symbols.resolve(*module_type).unwrap_or("?");
                        instances.push(format!("  - {} : {}", iname, mtype));
                    }
                    NodeKind::Function { name: n, is_task } => {
                        let fname = state.symbols.resolve(*n).unwrap_or("?");
                        let kind = if *is_task { "task" } else { "function" };
                        functions.push(format!("  - {} {}", kind, fname));
                    }
                    NodeKind::AlwaysBlock { .. } => always_count += 1,
                    NodeKind::Assignment => assign_count += 1,
                    _ => {}
                }
            }
        }
    }

    println!("\n## Ports ({})", ports.len());
    for p in &ports { println!("{}", p); }

    println!("\n## Signals ({})", signals.len());
    for s in &signals { println!("{}", s); }

    println!("\n## Instances ({})", instances.len());
    for i in &instances { println!("{}", i); }

    println!("\n## Functions/Tasks ({})", functions.len());
    for f in &functions { println!("{}", f); }

    println!("\n## Summary");
    println!("  Always blocks: {}", always_count);
    println!("  Assignments: {}", assign_count);

    Ok(())
}

fn cmd_impact(state: &ProjectState, symbol: &str) -> anyhow::Result<()> {
    use hdl_graph_core::helpers::*;
    use std::collections::{HashSet, VecDeque};

    // Find target nodes
    let target_ids: Vec<u64> = state.graph.all_nodes().iter()
        .filter(|n| node_name_from_kind(&n.kind, &state.symbols).as_deref() == Some(symbol))
        .map(|n| n.id)
        .collect();

    if target_ids.is_empty() {
        println!("Symbol not found: {}", symbol);
        return Ok(());
    }

    let target_set: HashSet<u64> = target_ids.iter().copied().collect();
    let mut visited: HashSet<u64> = target_ids.iter().copied().collect();
    let mut queue: VecDeque<(u64, usize)> = target_ids.iter().map(|&id| (id, 0)).collect();
    let mut results: Vec<(usize, String)> = Vec::new();

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth >= 3 { continue; }

        // Follow incoming edges
        if let Ok(incoming) = state.graph.get_incoming(node_id) {
            for edge in &incoming {
                if !is_impact_edge(edge.edge_type) { continue; }
                if visited.contains(&edge.source) { continue; }
                visited.insert(edge.source);

                if let Ok(Some(src)) = state.graph.get_node(edge.source) {
                    if !target_set.contains(&src.id) {
                        let kind = node_kind_str(&src.kind);
                        let sname = node_name_from_kind(&src.kind, &state.symbols)
                            .unwrap_or_else(|| format!("#{}", src.id));
                        results.push((depth + 1, format!("  - {} `{}` (via {})", kind, sname, edge.edge_type.name())));
                    }
                    queue.push_back((edge.source, depth + 1));
                }
            }
        }

        // For signals, also follow outgoing drives/connects
        if let Ok(Some(node)) = state.graph.get_node(node_id) {
            if matches!(node.kind, NodeKind::SignalDecl { .. }) {
                if let Ok(outgoing) = state.graph.get_outgoing(node_id) {
                    for edge in &outgoing {
                        if !matches!(edge.edge_type, EdgeType::Drives | EdgeType::Connects) { continue; }
                        if visited.contains(&edge.target) { continue; }
                        visited.insert(edge.target);
                        if let Ok(Some(tgt)) = state.graph.get_node(edge.target) {
                            let kind = node_kind_str(&tgt.kind);
                            let tname = node_name_from_kind(&tgt.kind, &state.symbols)
                                .unwrap_or_else(|| format!("#{}", tgt.id));
                            results.push((depth + 1, format!("  - {} `{}` (via {})", kind, tname, edge.edge_type.name())));
                            queue.push_back((edge.target, depth + 1));
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        println!("No downstream impact found for: {}", symbol);
        return Ok(());
    }

    println!("Impact analysis for '{}':", symbol);
    for depth in 1..=3 {
        let items: Vec<_> = results.iter().filter(|(d, _)| *d == depth).collect();
        if items.is_empty() { continue; }
        let heading = if depth == 1 { "Direct impact (depth 1)" } else { "Transitive" };
        println!("\n## {} (depth {})", heading, depth);
        for (_, line) in items { println!("{}", line); }
    }
    println!("\nTotal affected nodes: {}", results.len());

    Ok(())
}

fn cmd_node(state: &ProjectState, symbol: &str) -> anyhow::Result<()> {
    use hdl_graph_core::helpers::*;

    let candidates: Vec<_> = state.graph.all_nodes().into_iter()
        .filter(|n| node_name_from_kind(&n.kind, &state.symbols).as_deref() == Some(symbol))
        .collect();

    if candidates.is_empty() {
        // Case-insensitive fallback
        let lower = symbol.to_lowercase();
        let fuzzy: Vec<_> = state.graph.all_nodes().into_iter()
            .filter(|n| node_name_from_kind(&n.kind, &state.symbols)
                .map(|s| s.to_lowercase() == lower).unwrap_or(false))
            .collect();
        if fuzzy.is_empty() {
            println!("Symbol not found: {}", symbol);
            return Ok(());
        }
        return print_node_detail(&state, &fuzzy[0]);
    }

    if candidates.len() == 1 {
        return print_node_detail(&state, &candidates[0]);
    }

    println!("Multiple definitions found for '{}':\n", symbol);
    for (i, node) in candidates.iter().enumerate() {
        println!("## Match {}\n", i + 1);
        print_node_detail(state, node)?;
    }
    Ok(())
}

fn print_node_detail(state: &ProjectState, node: &GraphNode) -> anyhow::Result<()> {
    use hdl_graph_core::helpers::*;

    let kind = kind_display_name(&node.kind);
    let name = node_name_from_kind(&node.kind, &state.symbols)
        .unwrap_or_else(|| format!("#{}", node.id));

    println!("# {} `{}`\n", kind, name);
    println!("**Node ID:** {}", node.id);

    if let Some(scope_id) = node.scope_id {
        if let Ok(Some(parent)) = state.graph.get_node(scope_id) {
            let pname = node_name_from_kind(&parent.kind, &state.symbols)
                .unwrap_or_else(|| format!("#{}", parent.id));
            let pkind = kind_display_name(&parent.kind);
            println!("**Scope:** {} `{}`", pkind, pname);
        }
    }

    if let Some(file) = find_file_for_node(&state.graph, &state.file_map, node.id) {
        println!("**File:** {}", file);
    }

    // Type-specific details
    match &node.kind {
        NodeKind::ModulePort { direction, .. } => println!("**Direction:** {:?}", direction),
        NodeKind::SignalDecl { kind, .. } => println!("**Signal type:** {:?}", kind),
        NodeKind::ModuleInstance { module_type, .. } => {
            let t = state.symbols.resolve(*module_type).unwrap_or("?");
            println!("**Instantiates:** `{}`", t);
        }
        NodeKind::Class { parent, .. } => {
            if let Some(p) = parent {
                println!("**Extends:** `{}`", state.symbols.resolve(*p).unwrap_or("?"));
            }
        }
        NodeKind::Function { is_task, .. } => {
            println!("**Type:** {}", if *is_task { "task" } else { "function" });
        }
        NodeKind::Method { is_virtual, .. } => {
            if *is_virtual { println!("**Virtual:** yes"); }
        }
        NodeKind::TLMPort { direction, .. } => println!("**TLM Direction:** {:?}", direction),
        NodeKind::FactoryReg { type_name, base_type } => {
            let tn = state.symbols.resolve(*type_name).unwrap_or("?");
            let bt = state.symbols.resolve(*base_type).unwrap_or("?");
            println!("**Registers:** `{}` extends `{}`", tn, bt);
        }
        NodeKind::FactoryOverride { original_type, override_type } => {
            let ot = state.symbols.resolve(*original_type).unwrap_or("?");
            let ov = state.symbols.resolve(*override_type).unwrap_or("?");
            println!("**Override:** `{}` -> `{}`", ot, ov);
        }
        _ => {}
    }

    // Outgoing edges
    if let Ok(outgoing) = state.graph.get_outgoing(node.id) {
        if !outgoing.is_empty() {
            println!("\n## Outgoing edges ({})\n", outgoing.len());
            for edge in &outgoing {
                if let Ok(Some(target)) = state.graph.get_node(edge.target) {
                    let t_name = node_name_from_kind(&target.kind, &state.symbols)
                        .unwrap_or_else(|| format!("#{}", target.id));
                    let t_kind = kind_display_name(&target.kind);
                    println!("  - {} `{}` → {} `{}`", edge.edge_type.name(), name, t_kind, t_name);
                }
            }
        }
    }

    // Incoming edges (capped at 20)
    if let Ok(incoming) = state.graph.get_incoming(node.id) {
        if !incoming.is_empty() {
            let display = incoming.len().min(20);
            let suffix = if incoming.len() > 20 {
                format!(" of {}", incoming.len())
            } else {
                String::new()
            };
            println!("\n## Incoming edges ({}{})\n", display, suffix);
            for edge in incoming.iter().take(20) {
                if let Ok(Some(source)) = state.graph.get_node(edge.source) {
                    let s_name = node_name_from_kind(&source.kind, &state.symbols)
                        .unwrap_or_else(|| format!("#{}", source.id));
                    let s_kind = kind_display_name(&source.kind);
                    println!("  - {} `{}` → {} `{}`", s_kind, s_name, edge.edge_type.name(), name);
                }
            }
            if incoming.len() > 20 {
                println!("  ... and {} more", incoming.len() - 20);
            }
        }
    }

    Ok(())
}

fn cmd_files(state: &ProjectState, pattern: Option<&str>) -> anyhow::Result<()> {
    use hdl_graph_core::helpers::*;

    let files: Vec<_> = state.file_map.iter()
        .filter(|(path, _)| {
            match pattern {
                Some(p) if !p.is_empty() && p != "*" => glob_match(p, path),
                _ => true,
            }
        })
        .collect();

    if files.is_empty() {
        println!("No indexed files found.");
        return Ok(());
    }

    println!("Indexed files ({})\n", files.len());
    println!("| File | Nodes | Modules | Classes | Instances | Signals |");
    println!("|------|-------|---------|---------|-----------|---------|");

    let mut total_nodes = 0u32;
    let mut total_modules = 0u32;
    let mut total_classes = 0u32;
    let mut total_instances = 0u32;
    let mut total_signals = 0u32;

    for (path, &fid) in &files {
        let mut nodes = 0u32;
        let mut modules = 0u32;
        let mut classes = 0u32;
        let mut instances = 0u32;
        let mut signals = 0u32;

        if let Ok(edges) = state.graph.get_outgoing(fid) {
            for e in &edges {
                if e.edge_type != EdgeType::Contains { continue; }
                nodes += 1;
                if let Ok(Some(child)) = state.graph.get_node(e.target) {
                    match &child.kind {
                        NodeKind::Module { .. } => modules += 1,
                        NodeKind::Class { .. } => classes += 1,
                        NodeKind::ModuleInstance { .. } => instances += 1,
                        NodeKind::SignalDecl { .. } => signals += 1,
                        _ => {}
                    }
                    // Count one more level
                    if let Ok(inner) = state.graph.get_outgoing(child.id) {
                        for ie in &inner {
                            if ie.edge_type == EdgeType::Contains {
                                nodes += 1;
                                if let Ok(Some(grandchild)) = state.graph.get_node(ie.target) {
                                    match &grandchild.kind {
                                        NodeKind::Module { .. } => modules += 1,
                                        NodeKind::Class { .. } => classes += 1,
                                        NodeKind::ModuleInstance { .. } => instances += 1,
                                        NodeKind::SignalDecl { .. } => signals += 1,
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        println!("| {} | {} | {} | {} | {} | {} |", path, nodes, modules, classes, instances, signals);
        total_nodes += nodes;
        total_modules += modules;
        total_classes += classes;
        total_instances += instances;
        total_signals += signals;
    }

    println!("| **Total** | **{}** | **{}** | **{}** | **{}** | **{}** |",
        total_nodes, total_modules, total_classes, total_instances, total_signals);

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn node_kind_str(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Module { .. } => "module",
        NodeKind::ModulePort { .. } => "port",
        NodeKind::SignalDecl { .. } => "signal",
        NodeKind::ModuleInstance { .. } => "instance",
        NodeKind::Class { .. } => "class",
        NodeKind::Package { .. } => "package",
        NodeKind::Function { .. } => "function",
        NodeKind::Interface { .. } => "interface",
        _ => "symbol",
    }
}

// ---------------------------------------------------------------------------
// UVM command implementations
// ---------------------------------------------------------------------------

fn cmd_uvm_factory(state: &ProjectState, type_name: &str) -> anyhow::Result<()> {
    println!("UVM Factory: {type_name}");
    let mut found = false;

    for (_file, fid) in &state.file_map {
        if let Ok(edges) = state.graph.get_outgoing(*fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(node)) = state.graph.get_node(e.target) {
                        let children = state.graph.get_outgoing(e.target)?;
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                match &child.kind {
                                    NodeKind::FactoryReg { type_name: tn, base_type } => {
                                        let tn_str = state.symbols.resolve(*tn).unwrap_or("?");
                                        let bt_str = state.symbols.resolve(*base_type).unwrap_or("?");
                                        if tn_str == type_name || bt_str == type_name {
                                            println!("  Registration: {} extends {}", tn_str, bt_str);
                                            found = true;
                                        }
                                    }
                                    NodeKind::FactoryCreate { type_name: tn } => {
                                        let tn_str = state.symbols.resolve(*tn).unwrap_or("?");
                                        if tn_str == type_name {
                                            println!("  Create: type_id::create(\"{}\")", tn_str);
                                            found = true;
                                        }
                                    }
                                    NodeKind::FactoryOverride { original_type, override_type } => {
                                        let ot = state.symbols.resolve(*original_type).unwrap_or("?");
                                        let ov = state.symbols.resolve(*override_type).unwrap_or("?");
                                        if ot == type_name || ov == type_name {
                                            println!("  Override: {} → {}", ot, ov);
                                            found = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found {
        println!("  (no factory info found for '{type_name}')");
    }
    Ok(())
}

fn cmd_uvm_tlm(state: &ProjectState, component: &str) -> anyhow::Result<()> {
    println!("TLM Connections for: {component}");
    let mut found = false;

    for (_file, fid) in &state.file_map {
        if let Ok(edges) = state.graph.get_outgoing(*fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(node)) = state.graph.get_node(e.target) {
                        let children = state.graph.get_outgoing(e.target)?;
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                if let NodeKind::TLMPort { name, direction } = &child.kind {
                                    let n = state.symbols.resolve(*name).unwrap_or("?");
                                    let dir = format!("{:?}", direction).to_lowercase();
                                    println!("  Port: {n} ({dir})");
                                    found = true;

                                    // Find connections
                                    if let Ok(conns) = state.graph.get_outgoing(child.id) {
                                        for conn in &conns {
                                            if conn.edge_type == EdgeType::TLMBinds {
                                                if let Ok(Some(target)) = state.graph.get_node(conn.target) {
                                                    if let NodeKind::TLMPort { name: tn, .. } = &target.kind {
                                                        println!("    → connected to: {}", state.symbols.resolve(*tn).unwrap_or("?"));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found {
        println!("  (no TLM ports found for '{component}')");
    }
    Ok(())
}

fn cmd_uvm_config(state: &ProjectState, path: &str) -> anyhow::Result<()> {
    println!("Config DB operations matching: {path}");
    let mut found = false;

    for (_file, fid) in &state.file_map {
        if let Ok(edges) = state.graph.get_outgoing(*fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(node)) = state.graph.get_node(e.target) {
                        let children = state.graph.get_outgoing(e.target)?;
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                match &child.kind {
                                    NodeKind::ConfigDBSet { field } => {
                                        let f = state.symbols.resolve(*field).unwrap_or("?");
                                        if f.contains(path) || path == "*" {
                                            let node_name = node_label(&node, &state.symbols);
                                            println!("  SET   {f}  (in {node_name})");
                                            found = true;
                                        }
                                    }
                                    NodeKind::ConfigDBGet { field } => {
                                        let f = state.symbols.resolve(*field).unwrap_or("?");
                                        if f.contains(path) || path == "*" {
                                            let node_name = node_label(&node, &state.symbols);
                                            println!("  GET   {f}  (in {node_name})");
                                            found = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !found {
        println!("  (no config DB operations matching '{path}')");
    }
    Ok(())
}

fn cmd_uvm_hierarchy(state: &ProjectState) -> anyhow::Result<()> {
    println!("UVM Type Hierarchy:");
    let mut uvm_types: Vec<(String, Option<String>)> = Vec::new();

    for (_file, fid) in &state.file_map {
        if let Ok(edges) = state.graph.get_outgoing(*fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(node)) = state.graph.get_node(e.target) {
                        let children = state.graph.get_outgoing(e.target)?;
                        for ce in &children {
                            if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                                match &child.kind {
                                    NodeKind::Class { name, parent } => {
                                        let n = state.symbols.resolve(*name).unwrap_or("?").to_string();
                                        let p = parent.and_then(|p| state.symbols.resolve(p)).map(|s| s.to_string());
                                        uvm_types.push((n, p));
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if uvm_types.is_empty() {
        println!("  (no class hierarchy found)");
    } else {
        // Print tree
        let top_level: Vec<String> = uvm_types.iter()
            .filter(|(_, p)| p.is_none())
            .map(|(n, _)| n.clone())
            .collect();

        for top in &top_level {
            println!("  {}", top);
            print_type_children(&uvm_types, top, 2);
        }
    }
    Ok(())
}

fn print_type_children(types: &[(String, Option<String>)], parent: &str, depth: usize) {
    let indent = "  ".repeat(depth);
    for (name, p) in types {
        if p.as_deref() == Some(parent) {
            println!("{}{}", indent, name);
            print_type_children(types, name, depth + 1);
        }
    }
}

fn node_label(node: &GraphNode, symbols: &SymbolTable) -> String {
    match &node.kind {
        NodeKind::Module { name } => {
            format!("module {}", symbols.resolve(*name).unwrap_or("?"))
        }
        NodeKind::ModuleInstance { name, module_type } => {
            format!(
                "{}: {}",
                symbols.resolve(*name).unwrap_or("?"),
                symbols.resolve(*module_type).unwrap_or("?")
            )
        }
        NodeKind::SignalDecl { name, .. } => {
            symbols.resolve(*name).unwrap_or("?").to_string()
        }
        NodeKind::ModulePort { name, .. } => {
            symbols.resolve(*name).unwrap_or("?").to_string()
        }
        NodeKind::Class { name, .. } => {
            format!("class {}", symbols.resolve(*name).unwrap_or("?"))
        }
        NodeKind::Package { name } => {
            format!("package {}", symbols.resolve(*name).unwrap_or("?"))
        }
        NodeKind::Function { name, .. } => {
            format!("function {}", symbols.resolve(*name).unwrap_or("?"))
        }
        NodeKind::Interface { name } => {
            format!("interface {}", symbols.resolve(*name).unwrap_or("?"))
        }
        _ => "?".to_string(),
    }
}
