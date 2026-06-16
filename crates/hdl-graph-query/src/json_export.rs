use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;
use serde_json::{json, Value};

pub struct JsonExporter;

impl JsonExporter {
    pub fn export(
        graph: &InMemoryGraph,
        symbols: &SymbolTable,
        file_map: &HashMap<String, u64>,
        output: &Path,
    ) -> Result<()> {
        let mut nodes = Vec::new();
        for node in graph.all_nodes() {
            let name = node_name_str(symbols, &node.kind);
            let kind = node_kind_str(&node.kind);
            let file = find_file_for_node(graph, file_map, node.id);
            let mut entry = json!({
                "id": node.id,
                "kind": kind,
            });
            if let Some(n) = name {
                entry["name"] = Value::String(n);
            }
            if let Some(f) = file {
                entry["file"] = Value::String(f);
            }
            if let Some(scope) = node.scope_id {
                entry["scope_id"] = json!(scope);
            }
            nodes.push(entry);
        }

        let mut edges = Vec::new();
        for node in graph.all_nodes() {
            if let Ok(outgoing) = graph.get_outgoing(node.id) {
                for edge in &outgoing {
                    edges.push(json!({
                        "source": edge.source,
                        "target": edge.target,
                        "type": edge.edge_type.name(),
                    }));
                }
            }
        }

        let files: Vec<Value> = file_map.keys().map(|k| Value::String(k.clone())).collect();

        let result = json!({
            "metadata": {
                "tool": "hdl-graph",
                "version": env!("CARGO_PKG_VERSION"),
                "exported_at": chrono_now(),
            },
            "nodes": nodes,
            "edges": edges,
            "files": files,
        });

        let json_str = serde_json::to_string_pretty(&result)?;
        std::fs::write(output, json_str)?;
        Ok(())
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

fn node_kind_str(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::SourceFile => "SourceFile",
        NodeKind::Module { .. } => "Module",
        NodeKind::ModulePort { .. } => "ModulePort",
        NodeKind::ModuleInstance { .. } => "ModuleInstance",
        NodeKind::PortConnection { .. } => "PortConnection",
        NodeKind::GenerateBlock { .. } => "GenerateBlock",
        NodeKind::AlwaysBlock { .. } => "AlwaysBlock",
        NodeKind::SignalDecl { .. } => "SignalDecl",
        NodeKind::Assignment => "Assignment",
        NodeKind::Function { .. } => "Function",
        NodeKind::BeginBlock { .. } => "BeginBlock",
        NodeKind::VariableRef { .. } => "VariableRef",
        NodeKind::CallSite { .. } => "CallSite",
        NodeKind::Class { .. } => "Class",
        NodeKind::Method { .. } => "Method",
        NodeKind::Property { .. } => "Property",
        NodeKind::Package { .. } => "Package",
        NodeKind::PackageImport { .. } => "PackageImport",
        NodeKind::Interface { .. } => "Interface",
        NodeKind::Modport { .. } => "Modport",
        NodeKind::FactoryReg { .. } => "FactoryReg",
        NodeKind::FactoryCreate { .. } => "FactoryCreate",
        NodeKind::FactoryOverride { .. } => "FactoryOverride",
        NodeKind::TLMPort { .. } => "TLMPort",
        NodeKind::TLMBinding => "TLMBinding",
        NodeKind::ConfigDBSet { .. } => "ConfigDBSet",
        NodeKind::ConfigDBGet { .. } => "ConfigDBGet",
        NodeKind::AssertProperty => "AssertProperty",
        NodeKind::SequenceDecl { .. } => "SequenceDecl",
        NodeKind::PropertyDecl { .. } => "PropertyDecl",
        NodeKind::CoverGroup { .. } => "CoverGroup",
        NodeKind::CoverPoint { .. } => "CoverPoint",
        NodeKind::DPIImport { .. } => "DPIImport",
        NodeKind::BindDirective { .. } => "BindDirective",
        NodeKind::ConfigBlock { .. } => "ConfigBlock",
        NodeKind::Parameter { .. } => "Parameter",
    }
}

fn find_file_for_node(
    graph: &InMemoryGraph,
    file_map: &HashMap<String, u64>,
    node_id: u64,
) -> Option<String> {
    for (path, &fid) in file_map {
        if fid == node_id {
            return Some(path.clone());
        }
        // Check if node is contained in this file (1-2 hops)
        if let Ok(edges) = graph.get_outgoing(fid) {
            for e in &edges {
                if e.edge_type == EdgeType::Contains {
                    if e.target == node_id {
                        return Some(path.clone());
                    }
                    if let Ok(inner) = graph.get_outgoing(e.target) {
                        for ie in &inner {
                            if ie.edge_type == EdgeType::Contains && ie.target == node_id {
                                return Some(path.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}", d.as_secs()))
        .unwrap_or_else(|_| "unknown".to_string())
}
