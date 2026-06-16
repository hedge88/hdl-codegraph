use hdl_graph_core::*;

use crate::server::ProjectState;

/// Get detailed information about a specific symbol: its kind, name, parent
/// scope, outgoing/incoming edges, and source file location.
pub fn run(state: &ProjectState, symbol: &str) -> String {
    // Try exact match first, then case-insensitive
    let candidates: Vec<_> = state
        .graph
        .all_nodes()
        .into_iter()
        .filter(|n| node_name_str(&state.symbols, &n.kind).as_deref() == Some(symbol))
        .collect();

    if candidates.is_empty() {
        // Case-insensitive fallback
        let lower = symbol.to_lowercase();
        let fuzzy: Vec<_> = state
            .graph
            .all_nodes()
            .into_iter()
            .filter(|n| {
                node_name_str(&state.symbols, &n.kind)
                    .map(|s| s.to_lowercase() == lower)
                    .unwrap_or(false)
            })
            .collect();
        if fuzzy.is_empty() {
            return format!("Symbol not found: {}", symbol);
        }
        return format_node_detail(&state, &fuzzy[0]);
    }

    if candidates.len() == 1 {
        return format_node_detail(&state, &candidates[0]);
    }

    // Multiple matches — list them all
    let mut out = format!("Multiple definitions found for '{}':\n\n", symbol);
    for (i, node) in candidates.iter().enumerate() {
        out.push_str(&format!("## Match {}\n\n", i + 1));
        out.push_str(&format_node_detail(state, node));
        out.push('\n');
    }
    out
}

fn format_node_detail(state: &ProjectState, node: &GraphNode) -> String {
    let kind = kind_display_name(&node.kind);
    let name = node_name_str(&state.symbols, &node.kind)
        .unwrap_or_else(|| format!("#{}", node.id));
    let mut out = format!("# {} `{}`\n\n", kind, name);

    out.push_str(&format!("**Node ID:** {}\n", node.id));

    // Parent scope
    if let Some(scope_id) = node.scope_id {
        if let Ok(Some(parent)) = state.graph.get_node(scope_id) {
            let parent_name = node_name_str(&state.symbols, &parent.kind)
                .unwrap_or_else(|| format!("#{}", parent.id));
            let parent_kind = kind_display_name(&parent.kind);
            out.push_str(&format!("**Scope:** {} `{}`\n", parent_kind, parent_name));
        }
    }

    // Source file
    if let Some(file) = find_file_for_node(state, node.id) {
        out.push_str(&format!("**File:** {}\n", file));
    }

    // Type-specific details
    match &node.kind {
        NodeKind::ModulePort { direction, .. } => {
            out.push_str(&format!("**Direction:** {:?}\n", direction));
        }
        NodeKind::SignalDecl { kind, .. } => {
            out.push_str(&format!("**Signal type:** {:?}\n", kind));
        }
        NodeKind::ModuleInstance { module_type, .. } => {
            let t = state.symbols.resolve(*module_type).unwrap_or("?");
            out.push_str(&format!("**Instantiates:** `{}`\n", t));
        }
        NodeKind::Class { parent, .. } => {
            if let Some(p) = parent {
                let p_name = state.symbols.resolve(*p).unwrap_or("?");
                out.push_str(&format!("**Extends:** `{}`\n", p_name));
            }
        }
        NodeKind::Function { is_task, .. } => {
            let kind = if *is_task { "task" } else { "function" };
            out.push_str(&format!("**Type:** {}\n", kind));
        }
        NodeKind::Method { is_virtual, .. } => {
            if *is_virtual {
                out.push_str("**Virtual:** yes\n");
            }
        }
        NodeKind::TLMPort { direction, .. } => {
            out.push_str(&format!("**TLM Direction:** {:?}\n", direction));
        }
        NodeKind::FactoryReg {
            type_name,
            base_type,
        } => {
            let tn = state.symbols.resolve(*type_name).unwrap_or("?");
            let bt = state.symbols.resolve(*base_type).unwrap_or("?");
            out.push_str(&format!("**Registers:** `{}` extends `{}`\n", tn, bt));
        }
        NodeKind::FactoryOverride {
            original_type,
            override_type,
        } => {
            let ot = state.symbols.resolve(*original_type).unwrap_or("?");
            let ov = state.symbols.resolve(*override_type).unwrap_or("?");
            out.push_str(&format!("**Override:** `{}` -> `{}`\n", ot, ov));
        }
        _ => {}
    }

    out.push('\n');

    // Outgoing edges
    if let Ok(outgoing) = state.graph.get_outgoing(node.id) {
        if !outgoing.is_empty() {
            out.push_str(&format!("## Outgoing edges ({})\n\n", outgoing.len()));
            for edge in &outgoing {
                if let Ok(Some(target)) = state.graph.get_node(edge.target) {
                    let t_name = node_name_str(&state.symbols, &target.kind)
                        .unwrap_or_else(|| format!("#{}", target.id));
                    let t_kind = kind_display_name(&target.kind);
                    out.push_str(&format!(
                        "  - {} `{}` → {} `{}`\n",
                        edge.edge_type.name(),
                        name,
                        t_kind,
                        t_name
                    ));
                }
            }
            out.push('\n');
        }
    }

    // Incoming edges (limited to 20 for readability)
    if let Ok(incoming) = state.graph.get_incoming(node.id) {
        if !incoming.is_empty() {
            let display = if incoming.len() > 20 {
                20
            } else {
                incoming.len()
            };
            out.push_str(&format!(
                "## Incoming edges ({}{})\n\n",
                display,
                if incoming.len() > 20 {
                    format!(" of {}", incoming.len())
                } else {
                    String::new()
                }
            ));
            for edge in incoming.iter().take(20) {
                if let Ok(Some(source)) = state.graph.get_node(edge.source) {
                    let s_name = node_name_str(&state.symbols, &source.kind)
                        .unwrap_or_else(|| format!("#{}", source.id));
                    let s_kind = kind_display_name(&source.kind);
                    out.push_str(&format!(
                        "  - {} `{}` → {} `{}`\n",
                        s_kind, s_name, edge.edge_type.name(), name
                    ));
                }
            }
            if incoming.len() > 20 {
                out.push_str(&format!("  ... and {} more\n", incoming.len() - 20));
            }
            out.push('\n');
        }
    }

    out
}

fn kind_display_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::SourceFile => "file",
        NodeKind::Module { .. } => "module",
        NodeKind::ModulePort { .. } => "port",
        NodeKind::ModuleInstance { .. } => "instance",
        NodeKind::PortConnection { .. } => "port_connection",
        NodeKind::GenerateBlock { .. } => "generate",
        NodeKind::AlwaysBlock { .. } => "always",
        NodeKind::SignalDecl { .. } => "signal",
        NodeKind::Assignment => "assign",
        NodeKind::Function { .. } => "function",
        NodeKind::BeginBlock { .. } => "begin",
        NodeKind::VariableRef { .. } => "variable_ref",
        NodeKind::CallSite { .. } => "call_site",
        NodeKind::Class { .. } => "class",
        NodeKind::Method { .. } => "method",
        NodeKind::Property { .. } => "property",
        NodeKind::Package { .. } => "package",
        NodeKind::PackageImport { .. } => "import",
        NodeKind::Interface { .. } => "interface",
        NodeKind::Modport { .. } => "modport",
        NodeKind::FactoryReg { .. } => "factory_reg",
        NodeKind::FactoryCreate { .. } => "factory_create",
        NodeKind::FactoryOverride { .. } => "factory_override",
        NodeKind::TLMPort { .. } => "tlm_port",
        NodeKind::TLMBinding => "tlm_binding",
        NodeKind::ConfigDBSet { .. } => "config_db_set",
        NodeKind::ConfigDBGet { .. } => "config_db_get",
        NodeKind::AssertProperty => "assert",
        NodeKind::SequenceDecl { .. } => "sequence",
        NodeKind::PropertyDecl { .. } => "property",
        NodeKind::CoverGroup { .. } => "covergroup",
        NodeKind::CoverPoint { .. } => "coverpoint",
        NodeKind::Parameter { .. } => "parameter",
        NodeKind::DPIImport { .. } => "dpi_import",
        NodeKind::BindDirective { .. } => "bind",
        NodeKind::ConfigBlock { .. } => "config",
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
        | NodeKind::TLMPort { name, .. }
        | NodeKind::Parameter { name, .. }
        | NodeKind::SequenceDecl { name, .. }
        | NodeKind::PropertyDecl { name, .. }
        | NodeKind::CoverGroup { name, .. }
        | NodeKind::CoverPoint { name, .. }
        | NodeKind::Modport { name }
        | NodeKind::DPIImport { function_name: name }
        | NodeKind::ConfigDBSet { field: name }
        | NodeKind::ConfigDBGet { field: name } => symbols.resolve(*name).map(|s| s.to_string()),
        NodeKind::FactoryReg { type_name, .. } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryCreate { type_name } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryOverride { original_type, .. } => {
            symbols.resolve(*original_type).map(|s| s.to_string())
        }
        _ => None,
    }
}

fn find_file_for_node(state: &ProjectState, node_id: u64) -> Option<String> {
    for (path, &fid) in &state.file_map {
        if fid == node_id {
            return Some(path.clone());
        }
        // Walk containment edges (up to 3 hops)
        if let Ok(edges) = state.graph.get_outgoing(fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if e.target == node_id {
                        return Some(path.clone());
                    }
                    if let Ok(inner) = state.graph.get_outgoing(e.target) {
                        for ie in &inner {
                            if ie.edge_type == EdgeType::Contains {
                                if ie.target == node_id {
                                    return Some(path.clone());
                                }
                                // One more hop
                                if let Ok(deep) = state.graph.get_outgoing(ie.target) {
                                    for de in &deep {
                                        if de.edge_type == EdgeType::Contains
                                            && de.target == node_id
                                        {
                                            return Some(path.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}
