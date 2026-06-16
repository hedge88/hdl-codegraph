//! Shared helper functions used by both CLI and MCP frontends.

use crate::edge::EdgeType;
use crate::graph::Graph;
use crate::node::{GraphNode, NodeKind};
use crate::symbol::SymbolTable;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Name / label helpers
// ---------------------------------------------------------------------------

/// Map each NodeKind variant to a short lowercase string.
/// Covers all 30+ variants.
pub fn node_kind_str(kind: &NodeKind) -> &'static str {
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
        NodeKind::PropertyDecl { .. } => "property_decl",
        NodeKind::CoverGroup { .. } => "covergroup",
        NodeKind::CoverPoint { .. } => "coverpoint",
        NodeKind::Parameter { .. } => "parameter",
        NodeKind::DPIImport { .. } => "dpi_import",
        NodeKind::BindDirective { .. } => "bind",
        NodeKind::ConfigBlock { .. } => "config",
    }
}

/// Human-readable display name for a NodeKind (used in Markdown output).
pub fn kind_display_name(kind: &NodeKind) -> &'static str {
    node_kind_str(kind)
}

/// Return a human-readable label like `"module foo"` or `"fifo: my_fifo"`.
pub fn node_label(node: &GraphNode, symbols: &SymbolTable) -> String {
    let name = node.name_str(symbols);
    match &node.kind {
        NodeKind::Module { .. }
        | NodeKind::Class { .. }
        | NodeKind::Package { .. }
        | NodeKind::Interface { .. }
        | NodeKind::Function { .. } => {
            format!(
                "{} {}",
                node_kind_str(&node.kind),
                name.as_deref().unwrap_or("?")
            )
        }
        NodeKind::ModuleInstance { module_type, .. } => {
            let n = name.as_deref().unwrap_or("?");
            let t = symbols.resolve(*module_type).unwrap_or("?");
            format!("{}: {}", n, t)
        }
        NodeKind::SignalDecl { .. }
        | NodeKind::ModulePort { .. }
        | NodeKind::Property { .. }
        | NodeKind::VariableRef { .. }
        | NodeKind::Method { .. }
        | NodeKind::TLMPort { .. }
        | NodeKind::SequenceDecl { .. }
        | NodeKind::PropertyDecl { .. }
        | NodeKind::CoverGroup { .. }
        | NodeKind::CoverPoint { .. }
        | NodeKind::Modport { .. }
        | NodeKind::ConfigBlock { .. } => name.unwrap_or_else(|| format!("#{}", node.id)),
        _ => format!("{} #{}", node_kind_str(&node.kind), node.id),
    }
}

// ---------------------------------------------------------------------------
// File lookup
// ---------------------------------------------------------------------------

/// Walk containment edges from file nodes to find which file owns `node_id`.
/// Searches up to 3 hops deep.
pub fn find_file_for_node(
    graph: &dyn Graph,
    file_map: &HashMap<String, u64>,
    node_id: u64,
) -> Option<String> {
    for (path, &fid) in file_map {
        if fid == node_id {
            return Some(path.clone());
        }
        if let Ok(edges) = graph.get_outgoing(fid) {
            for e in &edges {
                if e.edge_type != EdgeType::Contains {
                    continue;
                }
                if e.target == node_id {
                    return Some(path.clone());
                }
                if let Ok(inner) = graph.get_outgoing(e.target) {
                    for ie in &inner {
                        if ie.edge_type != EdgeType::Contains {
                            continue;
                        }
                        if ie.target == node_id {
                            return Some(path.clone());
                        }
                        if let Ok(deep) = graph.get_outgoing(ie.target) {
                            for de in &deep {
                                if de.edge_type == EdgeType::Contains && de.target == node_id {
                                    return Some(path.clone());
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

/// Trace incoming Contains edges to find the enclosing Module name.
pub fn find_containing_module(
    graph: &dyn Graph,
    symbols: &SymbolTable,
    node_id: u64,
) -> String {
    if let Ok(incoming) = graph.get_incoming(node_id) {
        for edge in &incoming {
            if edge.edge_type == EdgeType::Contains {
                if let Ok(Some(parent)) = graph.get_node(edge.source) {
                    if let NodeKind::Module { name } = &parent.kind {
                        return symbols.resolve(*name).unwrap_or("?").to_string();
                    }
                    return find_containing_module(graph, symbols, edge.source);
                }
            }
        }
    }
    "?".to_string()
}

// ---------------------------------------------------------------------------
// Edge classification
// ---------------------------------------------------------------------------

/// Edges that represent a reference/usage relationship.
pub fn is_ref_edge(et: EdgeType) -> bool {
    matches!(
        et,
        EdgeType::References
            | EdgeType::Drives
            | EdgeType::Extends
            | EdgeType::Calls
            | EdgeType::ConfigSets
            | EdgeType::ConfigGets
            | EdgeType::Instantiates
            | EdgeType::Connects
            | EdgeType::FactoryRegisters
            | EdgeType::FactoryOverrides
            | EdgeType::TLMBinds
    )
}

/// Edges that should be followed in impact analysis (BFS).
pub fn is_impact_edge(et: EdgeType) -> bool {
    matches!(
        et,
        EdgeType::References
            | EdgeType::Drives
            | EdgeType::Calls
            | EdgeType::Connects
            | EdgeType::Instantiates
            | EdgeType::Extends
            | EdgeType::Overrides
            | EdgeType::FactoryRegisters
            | EdgeType::FactoryOverrides
            | EdgeType::TLMBinds
            | EdgeType::ConfigSets
            | EdgeType::ConfigGets
            | EdgeType::Contains
    )
}

// ---------------------------------------------------------------------------
// Glob matching
// ---------------------------------------------------------------------------

/// Simple glob pattern matching (case-insensitive).
/// Supports `*` (any chars) and `?` (single char).
/// Falls back to case-insensitive substring match if no glob chars.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() || pattern == "*" {
        return true;
    }

    let has_glob = pattern.contains('*') || pattern.contains('?');

    if has_glob {
        // Convert glob to a simple regex-like match using iterators
        glob_match_inner(pattern, text)
    } else {
        // Plain case-insensitive substring
        text.to_lowercase().contains(&pattern.to_lowercase())
    }
}

fn glob_match_inner(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.to_lowercase().chars().collect();
    let txt: Vec<char> = text.to_lowercase().chars().collect();
    glob_match_recursive(&pat, &txt)
}

fn glob_match_recursive(pat: &[char], txt: &[char]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0;

    while ti < txt.len() {
        if pi < pat.len() && (pat[pi] == '?' || pat[pi] == txt[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pat.len() && pat[pi] == '*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if star_pi != usize::MAX {
            pi = star_pi + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }

    while pi < pat.len() && pat[pi] == '*' {
        pi += 1;
    }

    pi == pat.len()
}
