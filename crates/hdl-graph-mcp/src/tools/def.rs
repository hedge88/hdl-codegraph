use hdl_graph_core::*;
use crate::server::ProjectState;

/// Find the definition location of a symbol (module, class, port, signal, function).
pub fn run(state: &ProjectState, symbol: &str) -> String {
    let mut results = Vec::new();

    for (_file, fid) in &state.file_map {
        let outgoing = match state.graph.get_outgoing(*fid) {
            Ok(o) => o,
            Err(_) => continue,
        };
        for edge in &outgoing {
            if edge.edge_type != EdgeType::Contains {
                continue;
            }
            if let Ok(Some(node)) = state.graph.get_node(edge.target) {
                // Check top-level containers
                let top_name = match &node.kind {
                    NodeKind::Module { name }
                    | NodeKind::Class { name, .. }
                    | NodeKind::Package { name } => state.symbols.resolve(*name),
                    _ => None,
                };
                if let Some(n) = top_name {
                    if n == symbol {
                        let kind = helpers::node_kind_str(&node.kind);
                        results.push(format!("{} {}  (file_id: {}, node_id: {})", kind, symbol, fid, node.id));
                    }
                }

                // Check children (ports, signals, instances)
                if let Ok(children) = state.graph.get_outgoing(edge.target) {
                    for ce in &children {
                        if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                            let name = match &child.kind {
                                NodeKind::ModulePort { name, .. }
                                | NodeKind::SignalDecl { name, .. }
                                | NodeKind::ModuleInstance { name, .. } => {
                                    state.symbols.resolve(*name)
                                }
                                _ => None,
                            };
                            if name == Some(symbol) {
                                let kind = helpers::node_kind_str(&child.kind);
                                results.push(format!("{} {}  (node_id: {})", kind, symbol, child.id));
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        format!("Definition not found: {}", symbol)
    } else {
        let mut out = format!("Definition of '{}':\n", symbol);
        for r in &results {
            out.push_str(&format!("  {}\n", r));
        }
        out
    }
}
