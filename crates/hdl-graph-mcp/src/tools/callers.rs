use hdl_graph_core::*;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, symbol: &str) -> String {
    let target_ids: Vec<u64> = state
        .graph
        .all_nodes()
        .iter()
        .filter(|n| node_name_str(&state.symbols, &n.kind).as_deref() == Some(symbol))
        .map(|n| n.id)
        .collect();

    if target_ids.is_empty() {
        return format!("No references found for: {}", symbol);
    }

    let target_set: std::collections::HashSet<u64> = target_ids.iter().copied().collect();
    let mut results = Vec::new();

    for node in state.graph.all_nodes() {
        if let Ok(edges) = state.graph.get_outgoing(node.id) {
            for edge in &edges {
                if !matches!(
                    edge.edge_type,
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
                ) {
                    continue;
                }
                if !target_set.contains(&edge.target) {
                    continue;
                }
                let sname = node_name_str(&state.symbols, &node.kind)
                    .unwrap_or_else(|| format_node(&node, &state.symbols));
                let edge_label = edge.edge_type.name();
                results.push(format!("  {} {} {}", sname, edge_label, symbol));
            }
        }
    }

    if results.is_empty() {
        format!("No references found for: {}", symbol)
    } else {
        let mut out = format!("References to {}:\n", symbol);
        for r in &results {
            out.push_str(r);
            out.push('\n');
        }
        out
    }
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
        | NodeKind::TLMPort { name, .. } => symbols.resolve(*name).map(|s| s.to_string()),
        _ => None,
    }
}

fn format_node(node: &GraphNode, symbols: &SymbolTable) -> String {
    let kind_str = match &node.kind {
        NodeKind::Module { .. } => "module",
        NodeKind::Class { .. } => "class",
        NodeKind::Function { .. } => "function",
        NodeKind::AlwaysBlock { .. } => "always",
        NodeKind::Assignment => "assign",
        _ => "node",
    };
    match node_name_str(symbols, &node.kind) {
        Some(name) => format!("{} {}", kind_str, name),
        None => format!("{} #{}", kind_str, node.id),
    }
}
