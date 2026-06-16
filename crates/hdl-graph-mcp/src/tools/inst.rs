use hdl_graph_core::*;
use crate::server::ProjectState;

/// Find all instantiations of a given module type across the project.
pub fn run(state: &ProjectState, module_type: &str) -> String {
    let mut results = Vec::new();

    for (file, fid) in &state.file_map {
        let outgoing = match state.graph.get_outgoing(*fid) {
            Ok(o) => o,
            Err(_) => continue,
        };
        for e in &outgoing {
            if e.edge_type != EdgeType::Contains {
                continue;
            }
            if let Ok(Some(module_node)) = state.graph.get_node(e.target) {
                let parent_name = match &module_node.kind {
                    NodeKind::Module { name } => state.symbols.resolve(*name).unwrap_or("?"),
                    _ => continue,
                };
                if let Ok(children) = state.graph.get_outgoing(e.target) {
                    for ce in &children {
                        if let Ok(Some(child)) = state.graph.get_node(ce.target) {
                            if let NodeKind::ModuleInstance { name, module_type: mt } = &child.kind {
                                let t = state.symbols.resolve(*mt).unwrap_or("?");
                                if t == module_type {
                                    let n = state.symbols.resolve(*name).unwrap_or("?");
                                    results.push(format!("  {} (in module {}, file {})", n, parent_name, file));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        format!("No instantiations found for: {}", module_type)
    } else {
        let mut out = format!("Instantiations of {}:\n", module_type);
        for r in &results {
            out.push_str(&format!("{}\n", r));
        }
        out
    }
}
