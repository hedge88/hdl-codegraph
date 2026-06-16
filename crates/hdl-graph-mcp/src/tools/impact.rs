use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use std::collections::{HashSet, VecDeque};

use crate::server::ProjectState;

/// Analyze the blast radius of changing a symbol.
/// Returns all nodes that would be affected: direct references, transitive
/// callers, connected signals, instantiated modules, and UVM overrides.
pub fn run(state: &ProjectState, symbol: &str) -> String {
    // 1. Find all node IDs matching the symbol
    let target_ids: Vec<u64> = state
        .graph
        .all_nodes()
        .iter()
        .filter(|n| node_name_str(&state.symbols, &n.kind).as_deref() == Some(symbol))
        .map(|n| n.id)
        .collect();

    if target_ids.is_empty() {
        return format!("Symbol not found: {}", symbol);
    }

    // 2. BFS from target nodes following incoming semantic edges
    let mut visited: HashSet<u64> = HashSet::new();
    let mut queue: VecDeque<(u64, u32)> = VecDeque::new(); // (node_id, depth)
    let mut impacts: Vec<(String, u32)> = Vec::new(); // (description, depth)

    for &tid in &target_ids {
        visited.insert(tid);
        queue.push_back((tid, 0));
    }

    while let Some((node_id, depth)) = queue.pop_front() {
        if depth > 3 {
            continue; // Limit blast radius depth
        }

        // Find incoming edges (things that depend on this node)
        if let Ok(incoming) = state.graph.get_incoming(node_id) {
            for edge in &incoming {
                if !is_impact_edge(edge.edge_type) {
                    continue;
                }
                if let Ok(Some(source)) = state.graph.get_node(edge.source) {
                    if visited.contains(&source.id) {
                        continue;
                    }
                    visited.insert(source.id);

                    let label = format_impact_node(&state.symbols, &source, edge.edge_type);
                    impacts.push((label, depth));

                    // Continue BFS for transitive dependencies
                    queue.push_back((source.id, depth + 1));
                }
            }
        }

        // For signals: also check outgoing Drives/Connects edges
        // (changing a signal's type/width affects its drivers)
        if matches!(
            state.graph.get_node(node_id),
            Ok(Some(ref n)) if matches!(n.kind, NodeKind::SignalDecl { .. })
        ) {
            if let Ok(outgoing) = state.graph.get_outgoing(node_id) {
                for edge in &outgoing {
                    if matches!(edge.edge_type, EdgeType::Drives | EdgeType::Connects) {
                        if let Ok(Some(target)) = state.graph.get_node(edge.target) {
                            if !visited.contains(&target.id) {
                                visited.insert(target.id);
                                let label = format_impact_node(
                                    &state.symbols,
                                    &target,
                                    edge.edge_type,
                                );
                                impacts.push((label, depth + 1));
                            }
                        }
                    }
                }
            }
        }
    }

    if impacts.is_empty() {
        return format!("No downstream impact found for: {}", symbol);
    }

    // Sort by depth (direct impacts first)
    impacts.sort_by_key(|(_, d)| *d);

    let mut out = format!("Impact analysis for '{}':\n\n", symbol);

    // Group by depth
    let mut current_depth = 0;
    let mut depth_labels = ["Direct impact (depth 1)", "Transitive (depth 2)", "Transitive (depth 3)"];

    for (label, depth) in &impacts {
        if *depth != current_depth {
            current_depth = *depth;
            let heading = depth_labels
                .get((*depth as usize).saturating_sub(1))
                .unwrap_or(&"Transitive");
            out.push_str(&format!("## {}\n\n", heading));
        }
        out.push_str(&format!("  - {}\n", label));
    }

    out.push_str(&format!(
        "\nTotal affected nodes: {}\n",
        impacts.len()
    ));

    out
}

fn is_impact_edge(et: EdgeType) -> bool {
    matches!(
        et,
        EdgeType::References
            | EdgeType::Drives
            | EdgeType::Calls
            | EdgeType::Connects
            | EdgeType::Instantiates
            | EdgeType::Extends
            | EdgeType::Overrides
            | EdgeType::FactoryRegisters
            | EdgeType::FactoryOverrides
            | EdgeType::TLMBinds
            | EdgeType::ConfigSets
            | EdgeType::ConfigGets
            | EdgeType::Contains
    )
}

fn format_impact_node(symbols: &SymbolTable, node: &GraphNode, via: EdgeType) -> String {
    let kind = match &node.kind {
        NodeKind::Module { .. } => "module",
        NodeKind::Class { .. } => "class",
        NodeKind::Function { .. } => "function",
        NodeKind::SignalDecl { .. } => "signal",
        NodeKind::ModulePort { .. } => "port",
        NodeKind::ModuleInstance { .. } => "instance",
        NodeKind::AlwaysBlock { .. } => "always",
        NodeKind::Assignment => "assign",
        NodeKind::CallSite { .. } => "call_site",
        NodeKind::FactoryOverride { .. } => "factory_override",
        _ => "node",
    };
    let name = node_name_str(symbols, &node.kind)
        .unwrap_or_else(|| format!("#{}", node.id));
    let edge_label = via.name();
    format!("{} `{}` (via {})", kind, name, edge_label)
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
        | NodeKind::CallSite { target: name }
        | NodeKind::Method { name, .. }
        | NodeKind::TLMPort { name, .. }
        | NodeKind::Parameter { name, .. }
        | NodeKind::Modport { name }
        | NodeKind::DPIImport { function_name: name }
        | NodeKind::ConfigDBSet { field: name }
        | NodeKind::ConfigDBGet { field: name } => symbols.resolve(*name).map(|s| s.to_string()),
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
