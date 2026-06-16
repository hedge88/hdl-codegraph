use std::collections::HashSet;
use hdl_graph_core::*;
use crate::server::ProjectState;

/// Run cross-reference consistency checks on the graph.
pub fn run(state: &ProjectState) -> String {
    let mut out = String::from("Graph Consistency Check\n=======================\n\n");

    let all_nodes = state.graph.all_nodes();
    let node_ids: HashSet<u64> = all_nodes.iter().map(|n| n.id).collect();

    // Collect all edges
    let mut all_edges: Vec<Edge> = Vec::new();
    for node in &all_nodes {
        if let Ok(outgoing) = state.graph.get_outgoing(node.id) {
            all_edges.extend(outgoing);
        }
    }

    let mut total_issues: usize = 0;

    // (a) Dangling edges
    let dangling: Vec<_> = all_edges.iter()
        .filter(|e| !node_ids.contains(&e.source) || !node_ids.contains(&e.target))
        .collect();
    let dangling_count = dangling.len();
    total_issues += dangling_count;
    out.push_str(&format!("Dangling edges: {}\n", dangling_count));
    for d in &dangling {
        out.push_str(&format!("  edge {} -> {} ({})\n", d.source, d.target, d.edge_type.name()));
    }

    // (b) Unresolved module instantiations
    let defined_modules: HashSet<String> = all_nodes.iter()
        .filter_map(|n| {
            if let NodeKind::Module { name } = &n.kind {
                state.symbols.resolve(*name).map(|s| s.to_string())
            } else { None }
        })
        .collect();

    let mut unresolved_instances = Vec::new();
    for node in &all_nodes {
        if let NodeKind::ModuleInstance { name, module_type } = &node.kind {
            let mt_name = state.symbols.resolve(*module_type).unwrap_or("?");
            if !defined_modules.contains(mt_name) {
                let inst_name = state.symbols.resolve(*name).unwrap_or("?");
                let parent = helpers::find_containing_module(&state.graph, &state.symbols, node.id);
                unresolved_instances.push(format!("  {}: {} (in module {})", inst_name, mt_name, parent));
            }
        }
    }
    let unresolved_count = unresolved_instances.len();
    total_issues += unresolved_count;
    out.push_str(&format!("Unresolved instances: {}\n", unresolved_count));
    for u in &unresolved_instances {
        out.push_str(&format!("{}\n", u));
    }

    // (c) Orphan nodes
    let orphans: Vec<_> = all_nodes.iter()
        .filter(|n| !matches!(n.kind, NodeKind::SourceFile))
        .filter(|n| {
            let has_out = state.graph.get_outgoing(n.id).map(|v| !v.is_empty()).unwrap_or(false);
            let has_in = state.graph.get_incoming(n.id).map(|v| !v.is_empty()).unwrap_or(false);
            !has_out && !has_in
        })
        .collect();
    let orphan_count = orphans.len();
    total_issues += orphan_count;
    out.push_str(&format!("Orphan nodes: {}\n", orphan_count));
    for o in &orphans {
        let label = helpers::node_label(o, &state.symbols);
        out.push_str(&format!("  {} (id: {})\n", label, o.id));
    }

    // (d) Unresolved class parents
    let defined_classes: HashSet<String> = all_nodes.iter()
        .filter_map(|n| {
            if let NodeKind::Class { name, .. } = &n.kind {
                state.symbols.resolve(*name).map(|s| s.to_string())
            } else { None }
        })
        .collect();

    let mut unresolved_parents = Vec::new();
    for node in &all_nodes {
        if let NodeKind::Class { name, parent } = &node.kind {
            if let Some(parent_sym) = parent {
                let parent_name = state.symbols.resolve(*parent_sym).unwrap_or("?");
                if !defined_classes.contains(parent_name) {
                    let class_name = state.symbols.resolve(*name).unwrap_or("?");
                    unresolved_parents.push(format!("  {}: parent '{}' not found", class_name, parent_name));
                }
            }
        }
    }
    let parent_count = unresolved_parents.len();
    total_issues += parent_count;
    out.push_str(&format!("Unresolved parents: {}\n", parent_count));
    for p in &unresolved_parents {
        out.push_str(&format!("{}\n", p));
    }

    // Summary
    out.push('\n');
    if total_issues == 0 {
        out.push_str("All checks passed\n");
    } else {
        out.push_str(&format!("Summary: {} issue(s) found\n", total_issues));
    }

    out
}
