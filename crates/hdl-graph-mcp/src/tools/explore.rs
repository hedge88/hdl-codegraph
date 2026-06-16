use hdl_graph_core::*;
use crate::server::ProjectState;

/// Explore a module/class in detail: ports, signals, instances, always blocks,
/// and connected modules in one call.
pub fn run(state: &ProjectState, name: &str) -> String {
    // Find the target node
    let target = state.graph.all_nodes().into_iter().find(|node| {
        matches!(&node.kind,
            NodeKind::Module { name: n } | NodeKind::Class { name: n, .. }
            | NodeKind::Package { name: n } | NodeKind::Interface { name: n }
            if state.symbols.resolve(*n) == Some(name)
        )
    });

    let target = match target {
        Some(t) => t,
        None => return format!("Module/Class not found: {}", name),
    };

    let mut out = format!("# {}\n\n", name);

    // Classify children
    let mut ports = Vec::new();
    let mut signals = Vec::new();
    let mut instances = Vec::new();
    let mut always_blocks = 0;
    let mut assignments = 0;
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut others = 0;

    if let Ok(edges) = state.graph.get_outgoing(target.id) {
        for e in &edges {
            if let Ok(Some(child)) = state.graph.get_node(e.target) {
                match &child.kind {
                    NodeKind::ModulePort { name, direction } => {
                        let n = state.symbols.resolve(*name).unwrap_or("?");
                        let dir = format!("{:?}", direction).to_lowercase();
                        ports.push(format!("  - {} ({})", n, dir));
                    }
                    NodeKind::SignalDecl { name, kind } => {
                        let n = state.symbols.resolve(*name).unwrap_or("?");
                        let k = format!("{:?}", kind).to_lowercase();
                        signals.push(format!("  - {} ({})", n, k));
                    }
                    NodeKind::ModuleInstance { name, module_type } => {
                        let n = state.symbols.resolve(*name).unwrap_or("?");
                        let t = state.symbols.resolve(*module_type).unwrap_or("?");
                        instances.push(format!("  - {} : {}", n, t));
                    }
                    NodeKind::AlwaysBlock { .. } => always_blocks += 1,
                    NodeKind::Assignment => assignments += 1,
                    NodeKind::Function { name, is_task } => {
                        let n = state.symbols.resolve(*name).unwrap_or("?");
                        let kind = if *is_task { "task" } else { "function" };
                        functions.push(format!("  - {} {}", kind, n));
                    }
                    NodeKind::Class { name, .. } => {
                        let n = state.symbols.resolve(*name).unwrap_or("?");
                        classes.push(format!("  - class {}", n));
                    }
                    _ => others += 1,
                }
            }
        }
    }

    // Find parent file
    let parent_file = state.file_map.iter().find_map(|(path, &fid)| {
        if let Ok(edges) = state.graph.get_outgoing(fid) {
            for e in &edges {
                if e.target == target.id {
                    return Some(path.clone());
                }
            }
        }
        None
    });

    if let Some(f) = &parent_file {
        out.push_str(&format!("**File:** {}\n\n", f));
    }

    // Ports
    if !ports.is_empty() {
        out.push_str(&format!("## Ports ({})\n", ports.len()));
        for p in &ports {
            out.push_str(p);
            out.push('\n');
        }
        out.push('\n');
    }

    // Signals
    if !signals.is_empty() {
        out.push_str(&format!("## Signals ({})\n", signals.len()));
        for s in &signals {
            out.push_str(s);
            out.push('\n');
        }
        out.push('\n');
    }

    // Instances
    if !instances.is_empty() {
        out.push_str(&format!("## Instances ({})\n", instances.len()));
        for i in &instances {
            out.push_str(i);
            out.push('\n');
        }
        out.push('\n');
    }

    // Functions/Tasks
    if !functions.is_empty() {
        out.push_str(&format!("## Functions/Tasks ({})\n", functions.len()));
        for f in &functions {
            out.push_str(f);
            out.push('\n');
        }
        out.push('\n');
    }

    // Classes
    if !classes.is_empty() {
        out.push_str(&format!("## Nested Classes ({})\n", classes.len()));
        for c in &classes {
            out.push_str(c);
            out.push('\n');
        }
        out.push('\n');
    }

    // Summary
    out.push_str("## Summary\n");
    out.push_str(&format!("  Always blocks: {}\n", always_blocks));
    out.push_str(&format!("  Assignments:   {}\n", assignments));
    if others > 0 {
        out.push_str(&format!("  Other nodes:   {}\n", others));
    }

    out
}
