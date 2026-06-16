use hdl_graph_core::*;
use crate::server::ProjectState;

pub fn run(state: &ProjectState, pattern: &str) -> String {
    let has_glob = pattern.contains('*') || pattern.contains('?');
    let pattern_lower = pattern.to_lowercase();
    let glob_re = if has_glob {
        let mut regex_str = String::from("(?i)");
        for ch in pattern.chars() {
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
    } else {
        None
    };

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for node in state.graph.all_nodes() {
        if seen.contains(&node.id) {
            continue;
        }
        let label = node_label(&node, &state.symbols);
        let label_lower = label.to_lowercase();
        let name_part = node_name_str(&state.symbols, &node.kind);
        let matches = if let Some(ref re) = glob_re {
            re.is_match(&label) || name_part.as_ref().is_some_and(|n| re.is_match(n))
        } else {
            label_lower.contains(&pattern_lower)
        };
        if matches {
            seen.insert(node.id);
            results.push(format!("  {} (id: {})", label, node.id));
        }
    }

    if results.is_empty() {
        format!("No symbols found matching '{}'", pattern)
    } else {
        let mut out = format!("Search results for '{}':\n", pattern);
        for r in &results {
            out.push_str(r);
            out.push('\n');
        }
        out
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

fn node_label(node: &GraphNode, symbols: &SymbolTable) -> String {
    let kind_str = node_kind_str(&node.kind);
    match node_name_str(symbols, &node.kind) {
        Some(name) => format!("{} {}", kind_str, name),
        None => kind_str.to_string(),
    }
}

fn node_kind_str(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Module { .. } => "module",
        NodeKind::ModulePort { .. } => "port",
        NodeKind::SignalDecl { .. } => "signal",
        NodeKind::ModuleInstance { .. } => "instance",
        NodeKind::Class { .. } => "class",
        NodeKind::Package { .. } => "package",
        NodeKind::Function { .. } => "function",
        NodeKind::Interface { .. } => "interface",
        NodeKind::Method { .. } => "method",
        NodeKind::Property { .. } => "property",
        NodeKind::Parameter { .. } => "parameter",
        NodeKind::AlwaysBlock { .. } => "always",
        NodeKind::Assignment => "assign",
        NodeKind::TLMPort { .. } => "tlm_port",
        NodeKind::FactoryReg { .. } => "factory_reg",
        NodeKind::FactoryCreate { .. } => "factory_create",
        NodeKind::FactoryOverride { .. } => "factory_override",
        NodeKind::ConfigDBSet { .. } => "config_db_set",
        NodeKind::ConfigDBGet { .. } => "config_db_get",
        _ => "symbol",
    }
}
