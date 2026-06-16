use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;

pub enum MarkdownMode {
    /// Single file output
    Single,
    /// One .md file per module/class
    PerModule,
}

pub struct MarkdownExporter;

impl MarkdownExporter {
    pub fn export(
        graph: &InMemoryGraph,
        symbols: &SymbolTable,
        file_map: &HashMap<String, u64>,
        output: &Path,
        mode: MarkdownMode,
    ) -> Result<()> {
        match mode {
            MarkdownMode::Single => export_single(graph, symbols, file_map, output),
            MarkdownMode::PerModule => export_per_module(graph, symbols, file_map, output),
        }
    }
}

fn export_single(
    graph: &InMemoryGraph,
    symbols: &SymbolTable,
    file_map: &HashMap<String, u64>,
    output: &Path,
) -> Result<()> {
    let mut out = String::new();
    out.push_str("# HDL Code Graph\n\n");

    // Module hierarchy
    out.push_str("## Module Hierarchy\n\n");
    for node in graph.all_nodes() {
        if let NodeKind::Module { name } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            out.push_str(&format!("- **{}**\n", n));
            print_tree_md(graph, symbols, node.id, 1, &mut out);
        }
    }
    out.push('\n');

    // Modules detail
    out.push_str("## Modules\n\n");
    for node in graph.all_nodes() {
        if let NodeKind::Module { name } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            out.push_str(&format!("### {}\n\n", n));
            render_module_detail(graph, symbols, node.id, &mut out);
        }
    }

    // Classes detail
    let classes: Vec<_> = graph
        .all_nodes()
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Class { .. }))
        .collect();
    if !classes.is_empty() {
        out.push_str("## UVM Classes\n\n");
        for node in classes {
            if let NodeKind::Class { name, parent } = &node.kind {
                let n = symbols.resolve(*name).unwrap_or("?");
                let p = parent
                    .as_ref()
                    .and_then(|p| symbols.resolve(*p))
                    .unwrap_or("");
                if p.is_empty() {
                    out.push_str(&format!("### {}\n\n", n));
                } else {
                    out.push_str(&format!("### {} extends {}\n\n", n, p));
                }
                render_class_detail(graph, symbols, node.id, &mut out);
            }
        }
    }

    // Packages
    let packages: Vec<_> = graph
        .all_nodes()
        .into_iter()
        .filter(|n| matches!(n.kind, NodeKind::Package { .. }))
        .collect();
    if !packages.is_empty() {
        out.push_str("## Packages\n\n");
        for node in packages {
            if let NodeKind::Package { name } = &node.kind {
                let n = symbols.resolve(*name).unwrap_or("?");
                out.push_str(&format!("### {}\n\n", n));
            }
        }
    }

    // Stats summary
    out.push_str("## Statistics\n\n");
    out.push_str(&format!("- Files: {}\n", file_map.len()));
    out.push_str(&format!("- Nodes: {}\n", graph.node_count()));
    out.push_str(&format!("- Edges: {}\n", graph.edge_count()));

    std::fs::write(output, out)?;
    Ok(())
}

fn export_per_module(
    graph: &InMemoryGraph,
    symbols: &SymbolTable,
    file_map: &HashMap<String, u64>,
    output: &Path,
) -> Result<()> {
    std::fs::create_dir_all(output)?;

    // index.md
    let mut index = String::new();
    index.push_str("# HDL Code Graph Index\n\n");
    index.push_str("## Modules\n\n");
    for node in graph.all_nodes() {
        if let NodeKind::Module { name } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            index.push_str(&format!("- [{}](./{}.md)\n", n, n));
        }
    }
    index.push_str("\n## Classes\n\n");
    for node in graph.all_nodes() {
        if let NodeKind::Class { name, .. } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            index.push_str(&format!("- [{}](./{}.md)\n", n, n));
        }
    }
    index.push_str("\n## Statistics\n\n");
    index.push_str(&format!("- Files: {}\n", file_map.len()));
    index.push_str(&format!("- Nodes: {}\n", graph.node_count()));
    index.push_str(&format!("- Edges: {}\n", graph.edge_count()));
    std::fs::write(output.join("index.md"), index)?;

    // Per-module files
    for node in graph.all_nodes() {
        if let NodeKind::Module { name } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            let mut out = format!("# Module: {}\n\n", n);
            render_module_detail(graph, symbols, node.id, &mut out);
            std::fs::write(output.join(format!("{}.md", n)), out)?;
        }
    }

    // Per-class files
    for node in graph.all_nodes() {
        if let NodeKind::Class { name, parent } = &node.kind {
            let n = symbols.resolve(*name).unwrap_or("?");
            let p = parent
                .as_ref()
                .and_then(|p| symbols.resolve(*p))
                .unwrap_or("");
            let mut out = if p.is_empty() {
                format!("# Class: {}\n\n", n)
            } else {
                format!("# Class: {} extends {}\n\n", n, p)
            };
            render_class_detail(graph, symbols, node.id, &mut out);
            std::fs::write(output.join(format!("{}.md", n)), out)?;
        }
    }

    Ok(())
}

fn render_module_detail(graph: &InMemoryGraph, symbols: &SymbolTable, node_id: u64, out: &mut String) {
    let mut ports = Vec::new();
    let mut signals = Vec::new();
    let mut instances = Vec::new();
    let mut always_blocks = 0;
    let mut assignments = 0;
    let mut functions = Vec::new();

    if let Ok(edges) = graph.get_outgoing(node_id) {
        for e in &edges {
            if let Ok(Some(child)) = graph.get_node(e.target) {
                match &child.kind {
                    NodeKind::ModulePort { name, direction } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let dir = format!("{:?}", direction).to_lowercase();
                        ports.push(format!("  - `{}` ({})", n, dir));
                    }
                    NodeKind::SignalDecl { name, kind } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let k = format!("{:?}", kind).to_lowercase();
                        signals.push(format!("  - `{}` ({})", n, k));
                    }
                    NodeKind::ModuleInstance { name, module_type } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let t = symbols.resolve(*module_type).unwrap_or("?");
                        instances.push(format!("  - `{}` : `{}`", n, t));
                    }
                    NodeKind::AlwaysBlock { .. } => always_blocks += 1,
                    NodeKind::Assignment => assignments += 1,
                    NodeKind::Function { name, is_task } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let kind = if *is_task { "task" } else { "function" };
                        functions.push(format!("  - {} `{}`", kind, n));
                    }
                    _ => {}
                }
            }
        }
    }

    if !ports.is_empty() {
        out.push_str(&format!("**Ports** ({}):\n", ports.len()));
        for p in &ports {
            out.push_str(p);
            out.push('\n');
        }
        out.push('\n');
    }
    if !signals.is_empty() {
        out.push_str(&format!("**Signals** ({}):\n", signals.len()));
        for s in &signals {
            out.push_str(s);
            out.push('\n');
        }
        out.push('\n');
    }
    if !instances.is_empty() {
        out.push_str(&format!("**Instances** ({}):\n", instances.len()));
        for i in &instances {
            out.push_str(i);
            out.push('\n');
        }
        out.push('\n');
    }
    if !functions.is_empty() {
        out.push_str(&format!("**Functions/Tasks** ({}):\n", functions.len()));
        for f in &functions {
            out.push_str(f);
            out.push('\n');
        }
        out.push('\n');
    }
    if always_blocks > 0 || assignments > 0 {
        out.push_str(&format!(
            "**Always blocks:** {}, **Assignments:** {}\n\n",
            always_blocks, assignments
        ));
    }
}

fn render_class_detail(graph: &InMemoryGraph, symbols: &SymbolTable, node_id: u64, out: &mut String) {
    let mut methods = Vec::new();
    let mut properties = Vec::new();

    if let Ok(edges) = graph.get_outgoing(node_id) {
        for e in &edges {
            if let Ok(Some(child)) = graph.get_node(e.target) {
                match &child.kind {
                    NodeKind::Method { name, is_virtual } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let virt = if *is_virtual { "virtual " } else { "" };
                        methods.push(format!("  - {}{}()", virt, n));
                    }
                    NodeKind::Property { name } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        properties.push(format!("  - `{}`", n));
                    }
                    _ => {}
                }
            }
        }
    }

    if !properties.is_empty() {
        out.push_str(&format!("**Properties** ({}):\n", properties.len()));
        for p in &properties {
            out.push_str(p);
            out.push('\n');
        }
        out.push('\n');
    }
    if !methods.is_empty() {
        out.push_str(&format!("**Methods** ({}):\n", methods.len()));
        for m in &methods {
            out.push_str(m);
            out.push('\n');
        }
        out.push('\n');
    }
}

fn print_tree_md(
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
                    NodeKind::ModuleInstance { name, module_type } => {
                        let n = symbols.resolve(*name).unwrap_or("?");
                        let t = symbols.resolve(*module_type).unwrap_or("?");
                        format!("`{}` : `{}`", n, t)
                    }
                    NodeKind::Module { name } => {
                        format!("module `{}`", symbols.resolve(*name).unwrap_or("?"))
                    }
                    _ => continue,
                };
                out.push_str(&format!("{}- {}\n", "  ".repeat(depth), label));
                print_tree_md(graph, symbols, e.target, depth + 1, out);
            }
        }
    }
}
