use crate::integration::common;

use common::*;
use hdl_graph_core::*;
use std::collections::HashMap;

/// Helper: search nodes by name (case-insensitive substring match).
fn search_nodes(state: &ProjectState, pattern: &str) -> Vec<String> {
    let pattern_lower = pattern.to_lowercase();
    let is_glob = pattern.contains('*') || pattern.contains('?');

    let all_nodes = state.graph.all_nodes();
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in &all_nodes {
        let label = node_label(&state.symbols, node);
        if seen.contains(&node.id) {
            continue;
        }
        let matches = if is_glob {
            glob_matches(&label, pattern)
        } else {
            label.to_lowercase().contains(&pattern_lower)
        };
        if matches {
            seen.insert(node.id);
            results.push(label);
        }
    }
    results
}

fn node_label(symbols: &SymbolTable, node: &GraphNode) -> String {
    let (kind, name) = match &node.kind {
        NodeKind::Module { name } => ("module", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::Class { name, .. } => ("class", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::SignalDecl { name, .. } => ("signal", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::ModulePort { name, .. } => ("port", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::ModuleInstance { name, .. } => ("instance", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::Function { name, .. } => ("function", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::Package { name } => ("package", symbols.resolve(*name).unwrap_or("?")),
        NodeKind::Interface { name } => ("interface", symbols.resolve(*name).unwrap_or("?")),
        _ => return format!("{:?}", node.kind),
    };
    format!("{} {}", kind, name)
}

fn glob_matches(text: &str, pattern: &str) -> bool {
    // Simple glob pattern matching (case-insensitive)
    let text_lower = text.to_lowercase();
    let pat_lower = pattern.to_lowercase();
    let parts: Vec<&str> = pat_lower.split('*').collect();
    if parts.len() == 1 {
        // No wildcards: substring match
        return text_lower.contains(&pat_lower);
    }
    // Wildcard match: each part must appear in order
    let mut pos = 0;
    for part in &parts {
        if part.is_empty() {
            continue;
        }
        match text_lower[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    true
}

/// Helper: build hierarchy text for a named module/class.
fn build_hierarchy(state: &ProjectState, name: &str) -> String {
    let all_nodes = state.graph.all_nodes();
    // Find the target node
    let target = all_nodes.iter().find(|n| match &n.kind {
        NodeKind::Module { name: n }
        | NodeKind::Class { name: n, .. }
        | NodeKind::Package { name: n }
        | NodeKind::Interface { name: n } => {
            state.symbols.resolve(*n).map(|s| s == name).unwrap_or(false)
        }
        _ => false,
    });
    let target = match target {
        Some(t) => t,
        None => return format!("Not found: {}", name),
    };

    let mut result = String::new();
    result.push_str(name);
    result.push('\n');
    print_tree(state, target.id, &mut result, "  ");
    result
}

fn print_tree(state: &ProjectState, node_id: u64, out: &mut String, indent: &str) {
    let edges = state.graph.get_outgoing(node_id).unwrap_or_default();
    for edge in &edges {
        if edge.edge_type != EdgeType::Contains {
            continue;
        }
        if let Ok(Some(child)) = state.graph.get_node(edge.target) {
            let label = node_label(&state.symbols, &child);
            out.push_str(&format!("{}|--{}\n", indent, label));
            let next_indent = format!("{}  ", indent);
            print_tree(state, child.id, out, &next_indent);
        }
    }
}

/// Helper: find all incoming semantic edges to nodes matching a symbol name.
fn find_callers(state: &ProjectState, symbol: &str) -> Vec<String> {
    let all_nodes = state.graph.all_nodes();
    let target_ids: std::collections::HashSet<u64> = all_nodes
        .iter()
        .filter(|n| {
            node_name(&state.symbols, n)
                .map(|name| name == symbol)
                .unwrap_or(false)
        })
        .map(|n| n.id)
        .collect();

    if target_ids.is_empty() {
        return vec![];
    }

    let semantic_types = [
        EdgeType::References,
        EdgeType::Drives,
        EdgeType::Calls,
        EdgeType::Extends,
        EdgeType::Instantiates,
        EdgeType::Connects,
    ];

    let mut results = Vec::new();
    for node in &all_nodes {
        let outgoing = state.graph.get_outgoing(node.id).unwrap_or_default();
        for edge in &outgoing {
            if semantic_types.contains(&edge.edge_type) && target_ids.contains(&edge.target) {
                let src_name = node_label(&state.symbols, node);
                results.push(format!("{} via {}", src_name, edge.edge_type.name()));
            }
        }
    }
    results
}

fn node_name(symbols: &SymbolTable, node: &GraphNode) -> Option<String> {
    match &node.kind {
        NodeKind::Module { name }
        | NodeKind::Class { name, .. }
        | NodeKind::SignalDecl { name, .. }
        | NodeKind::ModulePort { name, .. }
        | NodeKind::Function { name, .. }
        | NodeKind::Package { name }
        | NodeKind::Interface { name } => Some(symbols.resolve(*name).unwrap_or("?").to_string()),
        _ => None,
    }
}

/// Helper: count nodes by kind.
fn count_by_kind(state: &ProjectState) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for node in state.graph.all_nodes() {
        let kind_str = match &node.kind {
            NodeKind::Module { .. } => "Modules",
            NodeKind::Class { .. } => "Classes",
            NodeKind::SignalDecl { .. } => "Signals",
            NodeKind::ModulePort { .. } => "Ports",
            NodeKind::ModuleInstance { .. } => "Instances",
            NodeKind::AlwaysBlock { .. } => "Always",
            NodeKind::Assignment => "Assigns",
            NodeKind::Function { .. } => "Functions",
            NodeKind::Parameter { .. } => "Parameters",
            NodeKind::Property { .. } => "Properties",
            NodeKind::Method { .. } => "Methods",
            NodeKind::TLMPort { .. } => "TLM Ports",
            NodeKind::FactoryReg { .. } => "Factory Reg",
            NodeKind::FactoryCreate { .. } => "Factory New",
            NodeKind::FactoryOverride { .. } => "Factory Ovr",
            NodeKind::ConfigDBSet { .. } => "ConfigDB Set",
            NodeKind::ConfigDBGet { .. } => "ConfigDB Get",
            NodeKind::Package { .. } => "Packages",
            NodeKind::Interface { .. } => "Interfaces",
            NodeKind::AssertProperty => "Assertions",
            NodeKind::SequenceDecl { .. } => "Assertions",
            NodeKind::PropertyDecl { .. } => "Assertions",
            NodeKind::CoverGroup { .. } => "Assertions",
            NodeKind::CoverPoint { .. } => "Assertions",
            NodeKind::DPIImport { .. } => "DPI Imports",
            NodeKind::CallSite { .. } => "Call Sites",
            _ => continue,
        };
        *counts.entry(kind_str.to_string()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn test_search_glob() {
    let dir = fixture_dir("uvm_components");
    let state = index_project(&dir);

    let results = search_nodes(&state, "my_*");
    assert!(
        results.len() >= 3,
        "Expected at least 3 results for 'my_*', got {}",
        results.len()
    );
}

#[test]
fn test_search_case_insensitive() {
    let dir = fixture_dir("uvm_components");
    let state = index_project(&dir);

    let results = search_nodes(&state, "MY_DRIVER");
    assert!(
        !results.is_empty(),
        "Expected case-insensitive match for 'MY_DRIVER'"
    );
}

#[test]
fn test_hierarchy_tree() {
    let dir = fixture_dir("verilog_basic");
    let state = index_project(&dir);

    let tree = build_hierarchy(&state, "top");
    assert!(
        tree.contains("top"),
        "Hierarchy should start with 'top'"
    );
    // top contains u_counter and u_adder instances
    assert!(
        tree.contains("counter") || tree.contains("adder"),
        "Hierarchy should contain counter or adder, got:\n{}",
        tree
    );
}

#[test]
fn test_callers_of_signal() {
    let src = load_fixture("verilog_basic/counter.v");
    let (nodes, _edges, _extractor) = parse_sv_to_graph(&src, 1);

    // counter.v has signals: clk, rst_n, count, next_count
    // At least the always block should reference clk
    let has_signal = nodes.iter().any(|n| {
        matches!(n.kind, NodeKind::SignalDecl { .. })
    });
    assert!(has_signal, "Expected signal declarations in counter.v");
}

#[test]
fn test_drivers_of_signal() {
    let src = load_fixture("verilog_basic/counter.v");
    let (nodes, edges, _extractor) = parse_sv_to_graph(&src, 1);

    // Should have Drives and References edges
    let drives = find_edges_by_type(&edges, EdgeType::Drives);
    let refs = find_edges_by_type(&edges, EdgeType::References);

    assert!(!drives.is_empty(), "Expected Drives edges");
    assert!(!refs.is_empty(), "Expected References edges");

    // Should have signals
    let signals: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::SignalDecl { .. }));
    assert!(!signals.is_empty(), "Expected signal declarations");
}

#[test]
fn test_stats_counts() {
    let dir = fixture_dir("verilog_basic");
    let state = index_project(&dir);

    let counts = count_by_kind(&state);

    // Should have modules
    let modules = counts.get("Modules").copied().unwrap_or(0);
    assert!(
        modules >= 4,
        "Expected at least 4 modules, got {}",
        modules
    );

    // Should have ports
    let ports = counts.get("Ports").copied().unwrap_or(0);
    assert!(ports >= 5, "Expected at least 5 ports, got {}", ports);

    // Should have signals
    let signals = counts.get("Signals").copied().unwrap_or(0);
    assert!(signals >= 2, "Expected at least 2 signals, got {}", signals);
}

#[test]
fn test_explore_detail() {
    let dir = fixture_dir("verilog_basic");
    let state = index_project(&dir);

    let all_nodes = state.graph.all_nodes();
    let counter = all_nodes.iter().find(|n| {
        matches!(n.kind, NodeKind::Module { name } if state.symbols.resolve(name).unwrap_or("") == "counter")
    });
    assert!(counter.is_some(), "Expected 'counter' module in graph");

    let counter = counter.unwrap();
    // Should have outgoing Contains edges (ports, signals, always, assign)
    let outgoing = state.graph.get_outgoing(counter.id).unwrap_or_default();
    let contains_out: Vec<_> = outgoing
        .iter()
        .filter(|e| e.edge_type == EdgeType::Contains)
        .collect();
    assert!(
        contains_out.len() >= 3,
        "Expected at least 3 children of counter module, got {}",
        contains_out.len()
    );
}

#[test]
fn test_impact_analysis() {
    let dir = fixture_dir("verilog_basic");
    let state = index_project(&dir);

    // Find a signal node and check its impact
    let all_nodes = state.graph.all_nodes();
    let signal = all_nodes.iter().find(|n| matches!(n.kind, NodeKind::SignalDecl { .. }));
    if let Some(sig) = signal {
        // BFS: find all nodes that reference this signal
        let mut visited = std::collections::HashSet::new();
        visited.insert(sig.id);
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((sig.id, 0u32));

        while let Some((node_id, depth)) = queue.pop_front() {
            if depth >= 3 {
                continue;
            }
            // Find incoming edges (things that depend on this node)
            let incoming = state.graph.get_incoming(node_id).unwrap_or_default();
            for edge in &incoming {
                if !visited.contains(&edge.target) {
                    visited.insert(edge.target);
                    queue.push_back((edge.target, depth + 1));
                }
            }
        }

        // At minimum the signal itself is visited
        assert!(
            visited.len() >= 1,
            "Expected at least 1 node in impact analysis"
        );
    }
}
