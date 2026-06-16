use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, signal: &str) -> String {
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

    if drivers.is_empty() && readers.is_empty() {
        return format!("No drivers/readers found for: {}", signal);
    }

    let mut out = String::new();
    if !drivers.is_empty() {
        out.push_str(&format!("Drivers of {}:\n", signal));
        for d in &drivers {
            out.push_str(&format!("  {}\n", d));
        }
    }
    if !readers.is_empty() {
        out.push_str(&format!("Readers of {}:\n", signal));
        for r in &readers {
            out.push_str(&format!("  {}\n", r));
        }
    }
    out
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
                let tname = node_name_str(symbols, &target.kind);
                if tname.as_deref() == Some(signal) {
                    let source = graph.get_node(node_id).ok().flatten();
                    let sname = source
                        .as_ref()
                        .and_then(|s| node_name_str(symbols, &s.kind));
                    let label = sname.as_deref().unwrap_or("?");
                    match edge.edge_type {
                        EdgeType::Drives => drivers.push(label.to_string()),
                        EdgeType::References => readers.push(label.to_string()),
                        _ => {}
                    }
                }
            }
        }
        if edge.edge_type == EdgeType::Contains {
            find_drivers_recursive(graph, symbols, edge.target, signal, drivers, readers);
        }
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
