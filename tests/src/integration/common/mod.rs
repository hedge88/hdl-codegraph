use hdl_graph_core::*;
use hdl_graph_parse::GraphExtractor;
use hdl_graph_parse::preprocessor::preprocess;
use hdl_graph_storage::InMemoryGraph;
use std::collections::HashMap;
use std::path::Path;

/// Shared project state for integration tests (mirrors CLI's ProjectState).
pub struct ProjectState {
    pub graph: InMemoryGraph,
    pub symbols: SymbolTable,
    pub file_map: HashMap<String, u64>,
}

/// Parse a single SV source string into nodes and edges.
/// Returns (nodes, edges) — use extractor.symbols for name resolution.
pub fn parse_sv_to_graph(source: &str, file_id: u64) -> (Vec<GraphNode>, Vec<Edge>, GraphExtractor) {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&hdl_graph_grammar::language_ref())
        .expect("Failed to set tree-sitter language");
    let tree = parser
        .parse(source, None)
        .expect("Failed to parse source");
    assert!(
        !tree.root_node().has_error(),
        "Parse error in source:\n{}",
        source
    );
    let mut extractor = GraphExtractor::new();
    let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), file_id);
    (nodes, edges, extractor)
}

/// Preprocess UVM macros then parse into nodes and edges.
/// This runs the 4-pass UVM preprocessor (macro expansion) before tree-sitter parsing,
/// which is necessary for extracting UVM-specific nodes like FactoryReg, FactoryCreate,
/// ConfigDBSet, ConfigDBGet, TLMBinding, etc.
pub fn preprocess_and_parse(source: &str, file_id: u64) -> (Vec<GraphNode>, Vec<Edge>, GraphExtractor) {
    let result = preprocess(source, "test.sv", &HashMap::new(), &[]);
    parse_sv_to_graph(&result.expanded_source, file_id)
}

/// Preprocess UVM macros then index a directory of SV files into a ProjectState.
/// This runs the 4-pass UVM preprocessor on each file before parsing.
pub fn index_project_with_preprocessing(fixtures_dir: &Path) -> ProjectState {
    let mut graph = InMemoryGraph::new();
    let mut symbols = SymbolTable::new();
    let mut file_map = HashMap::new();

    let mut file_id: u64 = 1;
    let mut sv_files: Vec<_> = Vec::new();
    collect_sv_files(fixtures_dir, &mut sv_files);
    sv_files.sort();

    for path in &sv_files {
        let source = std::fs::read_to_string(path).expect(&format!("Failed to read {:?}", path));
        let rel_path = path
            .strip_prefix(fixtures_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Preprocess UVM macros
        let result = preprocess(&source, &rel_path, &HashMap::new(), &[]);
        let expanded = &result.expanded_source;

        // Create source file node
        let file_node_id = graph
            .add_node(GraphNode {
                id: 0,
                kind: NodeKind::SourceFile,
                scope_id: None,
                ..Default::default()
            })
            .unwrap();
        file_map.insert(rel_path, file_node_id);

        // Parse and extract
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&hdl_graph_grammar::language_ref())
            .expect("Failed to set tree-sitter language");
        if let Some(tree) = parser.parse(expanded, None) {
            if tree.root_node().has_error() {
                file_id += 1;
                continue;
            }
            let mut extractor = GraphExtractor::new();
            let (nodes, edges) = extractor.extract(&tree, expanded.as_bytes(), file_id);

            // Remap and merge nodes
            let mut id_map: HashMap<u64, u64> = HashMap::new();
            for node in &nodes {
                let remapped_kind = remap_node_kind(&extractor.symbols, &mut symbols, &node.kind);
                let new_node = GraphNode {
                    id: 0,
                    kind: remapped_kind,
                    scope_id: node.scope_id,
                    file_id: node.file_id,
                    line: node.line,
                    col: node.col,
                };
                let new_id = graph.add_node(new_node).unwrap();
                id_map.insert(node.id, new_id);
            }

            // Add file -> top-level Contains edges
            for node in &nodes {
                if matches!(
                    node.kind,
                    NodeKind::Module { .. }
                        | NodeKind::Class { .. }
                        | NodeKind::Package { .. }
                        | NodeKind::Interface { .. }
                ) && node.scope_id.is_none()
                {
                    if let Some(&new_target) = id_map.get(&node.id) {
                        graph
                            .add_edge(Edge {
                                source: file_node_id,
                                target: new_target,
                                edge_type: EdgeType::Contains,
                            })
                            .unwrap();
                    }
                }
            }

            // Remap and merge edges
            for edge in &edges {
                if let (Some(&new_src), Some(&new_tgt)) = (id_map.get(&edge.source), id_map.get(&edge.target)) {
                    graph
                        .add_edge(Edge {
                            source: new_src,
                            target: new_tgt,
                            edge_type: edge.edge_type,
                        })
                        .unwrap();
                }
            }
        }
        file_id += 1;
    }

    ProjectState {
        graph,
        symbols,
        file_map,
    }
}

/// Parse a single SV source string without asserting no parse errors.
/// Returns None if parsing fails (for edge case testing).
pub fn parse_sv_to_graph_lenient(
    source: &str,
    file_id: u64,
) -> Option<(Vec<GraphNode>, Vec<Edge>, GraphExtractor)> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&hdl_graph_grammar::language_ref())
        .expect("Failed to set tree-sitter language");
    let tree = parser.parse(source, None)?;
    if tree.root_node().has_error() {
        return None;
    }
    let mut extractor = GraphExtractor::new();
    let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), file_id);
    Some((nodes, edges, extractor))
}

/// Remap an InternedString from one SymbolTable to another.
fn remap_interned(src: &SymbolTable, dst: &mut SymbolTable, id: InternedString) -> InternedString {
    match src.resolve(id) {
        Some(s) => dst.intern(s),
        None => id,
    }
}

/// Remap all InternedString fields in a NodeKind from src to dst symbol table.
fn remap_node_kind(src: &SymbolTable, dst: &mut SymbolTable, kind: &NodeKind) -> NodeKind {
    match kind {
        NodeKind::SourceFile => NodeKind::SourceFile,
        NodeKind::Module { name } => NodeKind::Module { name: remap_interned(src, dst, *name) },
        NodeKind::ModulePort { name, direction } => NodeKind::ModulePort { name: remap_interned(src, dst, *name), direction: direction.clone() },
        NodeKind::ModuleInstance { name, module_type } => NodeKind::ModuleInstance { name: remap_interned(src, dst, *name), module_type: remap_interned(src, dst, *module_type) },
        NodeKind::PortConnection { port_name, actual } => NodeKind::PortConnection { port_name: remap_interned(src, dst, *port_name), actual: remap_interned(src, dst, *actual) },
        NodeKind::GenerateBlock { kind } => NodeKind::GenerateBlock { kind: kind.clone() },
        NodeKind::AlwaysBlock { kind } => NodeKind::AlwaysBlock { kind: kind.clone() },
        NodeKind::SignalDecl { name, kind } => NodeKind::SignalDecl { name: remap_interned(src, dst, *name), kind: kind.clone() },
        NodeKind::Assignment => NodeKind::Assignment,
        NodeKind::Function { name, is_task } => NodeKind::Function { name: remap_interned(src, dst, *name), is_task: *is_task },
        NodeKind::BeginBlock { label } => NodeKind::BeginBlock { label: label.map(|l| remap_interned(src, dst, l)) },
        NodeKind::VariableRef { name } => NodeKind::VariableRef { name: remap_interned(src, dst, *name) },
        NodeKind::CallSite { target } => NodeKind::CallSite { target: remap_interned(src, dst, *target) },
        NodeKind::Class { name, parent } => NodeKind::Class { name: remap_interned(src, dst, *name), parent: parent.map(|p| remap_interned(src, dst, p)) },
        NodeKind::Method { name, is_virtual } => NodeKind::Method { name: remap_interned(src, dst, *name), is_virtual: *is_virtual },
        NodeKind::Property { name } => NodeKind::Property { name: remap_interned(src, dst, *name) },
        NodeKind::Package { name } => NodeKind::Package { name: remap_interned(src, dst, *name) },
        NodeKind::PackageImport { source } => NodeKind::PackageImport { source: remap_interned(src, dst, *source) },
        NodeKind::Interface { name } => NodeKind::Interface { name: remap_interned(src, dst, *name) },
        NodeKind::Modport { name } => NodeKind::Modport { name: remap_interned(src, dst, *name) },
        NodeKind::FactoryReg { type_name, base_type } => NodeKind::FactoryReg { type_name: remap_interned(src, dst, *type_name), base_type: remap_interned(src, dst, *base_type) },
        NodeKind::FactoryCreate { type_name } => NodeKind::FactoryCreate { type_name: remap_interned(src, dst, *type_name) },
        NodeKind::FactoryOverride { original_type, override_type } => NodeKind::FactoryOverride { original_type: remap_interned(src, dst, *original_type), override_type: remap_interned(src, dst, *override_type) },
        NodeKind::TLMPort { name, direction } => NodeKind::TLMPort { name: remap_interned(src, dst, *name), direction: direction.clone() },
        NodeKind::TLMBinding => NodeKind::TLMBinding,
        NodeKind::ConfigDBSet { field } => NodeKind::ConfigDBSet { field: remap_interned(src, dst, *field) },
        NodeKind::ConfigDBGet { field } => NodeKind::ConfigDBGet { field: remap_interned(src, dst, *field) },
        NodeKind::AssertProperty => NodeKind::AssertProperty,
        NodeKind::SequenceDecl { name } => NodeKind::SequenceDecl { name: remap_interned(src, dst, *name) },
        NodeKind::PropertyDecl { name } => NodeKind::PropertyDecl { name: remap_interned(src, dst, *name) },
        NodeKind::CoverGroup { name } => NodeKind::CoverGroup { name: remap_interned(src, dst, *name) },
        NodeKind::CoverPoint { name } => NodeKind::CoverPoint { name: remap_interned(src, dst, *name) },
        NodeKind::Parameter { name } => NodeKind::Parameter { name: remap_interned(src, dst, *name) },
        NodeKind::DPIImport { function_name } => NodeKind::DPIImport { function_name: remap_interned(src, dst, *function_name) },
        NodeKind::BindDirective { module_type } => NodeKind::BindDirective { module_type: remap_interned(src, dst, *module_type) },
        NodeKind::ConfigBlock { name } => NodeKind::ConfigBlock { name: remap_interned(src, dst, *name) },
    }
}

/// Index a directory of SV files into a ProjectState (mirrors CLI cmd_index).
/// Remaps InternedString values so all nodes share a single SymbolTable.
pub fn index_project(fixtures_dir: &Path) -> ProjectState {
    let mut graph = InMemoryGraph::new();
    let mut symbols = SymbolTable::new();
    let mut file_map = HashMap::new();

    let mut file_id: u64 = 1;
    let mut sv_files: Vec<_> = Vec::new();
    collect_sv_files(fixtures_dir, &mut sv_files);
    sv_files.sort();

    for path in &sv_files {
        let source = std::fs::read_to_string(path).expect(&format!("Failed to read {:?}", path));
        let rel_path = path
            .strip_prefix(fixtures_dir)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        // Create source file node
        let file_node_id = graph
            .add_node(GraphNode {
                id: 0,
                kind: NodeKind::SourceFile,
                scope_id: None,
                ..Default::default()
            })
            .unwrap();
        file_map.insert(rel_path, file_node_id);

        // Parse and extract
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&hdl_graph_grammar::language_ref())
            .expect("Failed to set tree-sitter language");
        if let Some(tree) = parser.parse(&source, None) {
            if tree.root_node().has_error() {
                file_id += 1;
                continue;
            }
            let mut extractor = GraphExtractor::new();
            let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), file_id);

            // Remap and merge nodes — map old IDs to new IDs
            let mut id_map: HashMap<u64, u64> = HashMap::new();
            for node in &nodes {
                let remapped_kind = remap_node_kind(&extractor.symbols, &mut symbols, &node.kind);
                let new_node = GraphNode {
                    id: 0, // auto-assign
                    kind: remapped_kind,
                    scope_id: node.scope_id,
                    file_id: node.file_id,
                    line: node.line,
                    col: node.col,
                };
                let new_id = graph.add_node(new_node).unwrap();
                id_map.insert(node.id, new_id);
            }

            // Add file -> top-level module/class Contains edges
            for node in &nodes {
                if matches!(
                    node.kind,
                    NodeKind::Module { .. }
                        | NodeKind::Class { .. }
                        | NodeKind::Package { .. }
                        | NodeKind::Interface { .. }
                ) && node.scope_id.is_none()
                {
                    if let Some(&new_target) = id_map.get(&node.id) {
                        graph
                            .add_edge(Edge {
                                source: file_node_id,
                                target: new_target,
                                edge_type: EdgeType::Contains,
                            })
                            .unwrap();
                    }
                }
            }

            // Remap and merge edges
            for edge in &edges {
                if let (Some(&new_src), Some(&new_tgt)) = (id_map.get(&edge.source), id_map.get(&edge.target)) {
                    graph
                        .add_edge(Edge {
                            source: new_src,
                            target: new_tgt,
                            edge_type: edge.edge_type,
                        })
                        .unwrap();
                }
            }
        }
        file_id += 1;
    }

    ProjectState {
        graph,
        symbols,
        file_map,
    }
}

fn collect_sv_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_sv_files(&path, out);
            } else {
                match path.extension().and_then(|e| e.to_str()) {
                    Some("sv" | "svh" | "svi" | "v" | "vh" | "pkg") => out.push(path),
                    _ => {}
                }
            }
        }
    }
}

/// Find all nodes matching a NodeKind pattern.
pub fn find_nodes_by_kind<'a>(nodes: &'a [GraphNode], pred: impl Fn(&NodeKind) -> bool) -> Vec<&'a GraphNode> {
    nodes.iter().filter(|n| pred(&n.kind)).collect()
}

/// Find all edges matching an EdgeType.
pub fn find_edges_by_type(edges: &[Edge], edge_type: EdgeType) -> Vec<&Edge> {
    edges.iter().filter(|e| e.edge_type == edge_type).collect()
}

/// Resolve an InternedString via a SymbolTable.
pub fn resolve_name(symbols: &SymbolTable, name: InternedString) -> String {
    symbols
        .resolve(name)
        .unwrap_or("<unresolved>")
        .to_string()
}

/// Resolve an InternedString via a GraphExtractor's symbol table.
pub fn resolve_from_extractor(extractor: &GraphExtractor, name: InternedString) -> String {
    extractor
        .symbols
        .resolve(name)
        .unwrap_or("<unresolved>")
        .to_string()
}

/// Load a fixture file by relative path under tests/fixtures/.
pub fn load_fixture(rel_path: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = Path::new(manifest_dir).join("fixtures").join(rel_path);
    std::fs::read_to_string(&path).expect(&format!("Failed to load fixture: {:?}", path))
}

/// Get the absolute path to a fixture directory.
pub fn fixture_dir(rel_path: &str) -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir)
        .join("fixtures")
        .join(rel_path)
}
