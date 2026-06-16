use hdl_graph_core::*;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, name: &str) -> String {
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
        format!("No call sites found for: {}", name)
    } else {
        let mut out = format!("Call sites for {}:\n", name);
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
