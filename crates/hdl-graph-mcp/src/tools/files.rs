use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;

use crate::server::ProjectState;

/// Get the indexed file structure, optionally filtered by extension or pattern.
/// Returns the list of indexed files with their node/edge counts.
pub fn run(state: &ProjectState, pattern: Option<&str>) -> String {
    let glob_re = pattern.and_then(|p| {
        if p.is_empty() || p == "*" {
            return None;
        }
        let mut regex_str = String::from("(?i)");
        for ch in p.chars() {
            match ch {
                '*' => regex_str.push_str(".*"),
                '?' => regex_str.push('.'),
                '.' | '(' | ')' | '[' | ']' | '{' | '}' | '+' | '^' | '$' | '|' | '\\' => {
                    regex_str.push('\\');
                    regex_str.push(ch);
                }
                _ => regex_str.push(ch),
            }
        }
        regex::Regex::new(&regex_str).ok()
    });

    let mut files: Vec<FileInfo> = Vec::new();

    for (path, &fid) in &state.file_map {
        if let Some(ref re) = glob_re {
            if !re.is_match(path) {
                continue;
            }
        }

        let mut nodes = 0u32;
        let mut modules = 0u32;
        let mut classes = 0u32;
        let mut instances = 0u32;
        let mut signals = 0u32;

        // Count nodes contained in this file
        if let Ok(edges) = state.graph.get_outgoing(fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if let Ok(Some(child)) = state.graph.get_node(e.target) {
                        nodes += 1;
                        match &child.kind {
                            NodeKind::Module { .. } => modules += 1,
                            NodeKind::Class { .. } => classes += 1,
                            NodeKind::ModuleInstance { .. } => instances += 1,
                            NodeKind::SignalDecl { .. } => signals += 1,
                            _ => {}
                        }
                        // Count children of children
                        if let Ok(inner) = state.graph.get_outgoing(e.target) {
                            for ie in &inner {
                                if ie.edge_type == EdgeType::Contains {
                                    if state.graph.get_node(ie.target).ok().flatten().is_some() {
                                        nodes += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        files.push(FileInfo {
            path: path.clone(),
            nodes,
            modules,
            classes,
            instances,
            signals,
        });
    }

    // Sort by path
    files.sort_by(|a, b| a.path.cmp(&b.path));

    if files.is_empty() {
        return "No indexed files found.".to_string();
    }

    let mut out = format!("Indexed files ({})\n\n", files.len());

    // Summary table
    out.push_str("| File | Nodes | Modules | Classes | Instances | Signals |\n");
    out.push_str("|------|-------|---------|---------|-----------|--------|\n");

    let mut total_nodes = 0u32;
    let mut total_modules = 0u32;
    let mut total_classes = 0u32;
    let mut total_instances = 0u32;
    let mut total_signals = 0u32;

    for f in &files {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            f.path, f.nodes, f.modules, f.classes, f.instances, f.signals
        ));
        total_nodes += f.nodes;
        total_modules += f.modules;
        total_classes += f.classes;
        total_instances += f.instances;
        total_signals += f.signals;
    }

    out.push_str(&format!(
        "| **Total** | **{}** | **{}** | **{}** | **{}** | **{}** |\n",
        total_nodes, total_modules, total_classes, total_instances, total_signals
    ));

    out
}

struct FileInfo {
    path: String,
    nodes: u32,
    modules: u32,
    classes: u32,
    instances: u32,
    signals: u32,
}
