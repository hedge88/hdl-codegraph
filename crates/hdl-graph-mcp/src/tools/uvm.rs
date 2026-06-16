use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, analysis: &str, query: Option<&str>) -> String {
    match analysis {
        "factory" => uvm_factory(state, query.unwrap_or("*")),
        "tlm" => uvm_tlm(state, query.unwrap_or("*")),
        "config" => uvm_config(state, query.unwrap_or("*")),
        "hierarchy" => uvm_hierarchy(state),
        _ => format!(
            "Unknown UVM analysis type '{}'. Use: factory, tlm, config, hierarchy",
            analysis
        ),
    }
}

fn uvm_factory(state: &ProjectState, type_name: &str) -> String {
    let mut results = Vec::new();

    for node in state.graph.all_nodes() {
        match &node.kind {
            NodeKind::FactoryReg { type_name: tn, base_type } => {
                let tn_str = state.symbols.resolve(*tn).unwrap_or("?");
                let bt_str = state.symbols.resolve(*base_type).unwrap_or("?");
                let parent_name = find_parent_name(&state.graph, &state.symbols, node.id);
                let parent_class_name = parent_name.strip_prefix("class ").unwrap_or(&parent_name);
                if type_name == "*"
                    || tn_str == type_name
                    || bt_str == type_name
                    || parent_class_name == type_name
                {
                    results.push(format!("  Registration: {} extends {}", tn_str, bt_str));
                }
            }
            NodeKind::FactoryCreate { type_name: tn } => {
                let tn_str = state.symbols.resolve(*tn).unwrap_or("?");
                if type_name == "*" || tn_str == type_name {
                    results.push(format!("  Create: type_id::create(\"{}\")", tn_str));
                }
            }
            NodeKind::FactoryOverride { original_type, override_type } => {
                let ot = state.symbols.resolve(*original_type).unwrap_or("?");
                let ov = state.symbols.resolve(*override_type).unwrap_or("?");
                if type_name == "*" || ot == type_name || ov == type_name {
                    results.push(format!("  Override: {} -> {}", ot, ov));
                }
            }
            _ => {}
        }
    }

    if results.is_empty() {
        format!("No factory info found for '{}'", type_name)
    } else {
        let mut out = format!("UVM Factory ({})\n", type_name);
        for r in &results {
            out.push_str(r);
            out.push('\n');
        }
        out
    }
}

fn uvm_tlm(state: &ProjectState, component: &str) -> String {
    let mut results = Vec::new();

    for node in state.graph.all_nodes() {
        if let NodeKind::TLMPort { name, direction } = &node.kind {
            let n = state.symbols.resolve(*name).unwrap_or("?");
            if component != "*" && n != component {
                continue;
            }
            let dir = format!("{:?}", direction).to_lowercase();
            results.push(format!("  Port: {} ({})", n, dir));

            if let Ok(conns) = state.graph.get_outgoing(node.id) {
                for conn in &conns {
                    if conn.edge_type == EdgeType::TLMBinds {
                        if let Ok(Some(target)) = state.graph.get_node(conn.target) {
                            if let NodeKind::TLMPort { name: tn, .. } = &target.kind {
                                results.push(format!(
                                    "    -> connected to: {}",
                                    state.symbols.resolve(*tn).unwrap_or("?")
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    if results.is_empty() {
        format!("No TLM connections found for '{}'", component)
    } else {
        let mut out = format!("TLM Connections ({})\n", component);
        for r in &results {
            out.push_str(r);
            out.push('\n');
        }
        out
    }
}

fn uvm_config(state: &ProjectState, path: &str) -> String {
    let mut results = Vec::new();

    for node in state.graph.all_nodes() {
        match &node.kind {
            NodeKind::ConfigDBSet { field } => {
                let f = state.symbols.resolve(*field).unwrap_or("?");
                if path == "*" || f.contains(path) {
                    let parent = find_parent_name(&state.graph, &state.symbols, node.id);
                    results.push(format!("  SET: {} (in {})", f, parent));
                }
            }
            NodeKind::ConfigDBGet { field } => {
                let f = state.symbols.resolve(*field).unwrap_or("?");
                if path == "*" || f.contains(path) {
                    let parent = find_parent_name(&state.graph, &state.symbols, node.id);
                    results.push(format!("  GET: {} (in {})", f, parent));
                }
            }
            _ => {}
        }
    }

    if results.is_empty() {
        format!("No config_db entries found for '{}'", path)
    } else {
        let mut out = format!("ConfigDB ({})\n", path);
        for r in &results {
            out.push_str(r);
            out.push('\n');
        }
        out
    }
}

fn uvm_hierarchy(state: &ProjectState) -> String {
    // Collect all classes with optional parent
    let mut classes: Vec<(String, Option<String>)> = Vec::new();
    for node in state.graph.all_nodes() {
        if let NodeKind::Class { name, parent } = &node.kind {
            let n = state.symbols.resolve(*name).unwrap_or("?").to_string();
            let p = parent.as_ref().and_then(|p| state.symbols.resolve(*p).map(|s| s.to_string()));
            classes.push((n, p));
        }
    }

    if classes.is_empty() {
        return "No UVM classes found".to_string();
    }

    // Find root classes (no parent)
    let mut out = "UVM Class Hierarchy:\n".to_string();
    for (name, parent) in &classes {
        if parent.is_none() {
            out.push_str(&format!("  {}\n", name));
            print_type_children(&classes, name, 2, &mut out);
        }
    }
    out
}

fn print_type_children(classes: &[(String, Option<String>)], parent: &str, depth: usize, out: &mut String) {
    for (name, p) in classes {
        if p.as_deref() == Some(parent) {
            out.push_str(&format!("{}{}\n", "  ".repeat(depth), name));
            print_type_children(classes, name, depth + 1, out);
        }
    }
}

fn find_parent_name(graph: &InMemoryGraph, symbols: &SymbolTable, node_id: u64) -> String {
    let mut current = node_id;
    for _ in 0..10 {
        if let Ok(incoming) = graph.get_incoming(current) {
            for edge in &incoming {
                if edge.edge_type == EdgeType::Contains {
                    if let Ok(Some(parent)) = graph.get_node(edge.source) {
                        let name = match &parent.kind {
                            NodeKind::Class { name, .. } => {
                                format!("class {}", symbols.resolve(*name).unwrap_or("?"))
                            }
                            NodeKind::Module { name } => {
                                format!("module {}", symbols.resolve(*name).unwrap_or("?"))
                            }
                            _ => {
                                current = edge.source;
                                continue;
                            }
                        };
                        return name;
                    }
                }
            }
        }
        break;
    }
    "unknown".to_string()
}
