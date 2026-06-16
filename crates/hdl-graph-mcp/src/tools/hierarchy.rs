use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, name: &str) -> String {
    for node in state.graph.all_nodes() {
        let matches = match &node.kind {
            NodeKind::Module { name: n } => state.symbols.resolve(*n) == Some(name),
            NodeKind::Class { name: n, .. } => state.symbols.resolve(*n) == Some(name),
            NodeKind::Package { name: n } => state.symbols.resolve(*n) == Some(name),
            NodeKind::Interface { name: n } => state.symbols.resolve(*n) == Some(name),
            _ => false,
        };
        if matches {
            let mut out = format!("{}\n", name);
            print_tree(&state.graph, &state.symbols, node.id, 1, &mut out);
            return out;
        }
    }
    format!("Module/Class not found: {}", name)
}

fn print_tree(
    graph: &InMemoryGraph,
    symbols: &SymbolTable,
    node_id: u64,
    depth: usize,
    out: &mut String,
) {
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
                out.push_str(&format!("{}{} {}\n", "  ".repeat(depth), "|--", label));
                print_tree(graph, symbols, e.target, depth + 1, out);
            }
        }
    }
}
