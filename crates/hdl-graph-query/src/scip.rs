use std::path::Path;
use std::collections::HashMap;
use hdl_graph_core::*;
use hdl_graph_storage::InMemoryGraph;

/// Exports an HDL Code Graph to SCIP-compatible JSON format.
pub struct ScipExporter;

impl ScipExporter {
    /// Export the graph to JSON-based SCIP format at the given output path.
    pub fn export(
        graph: &InMemoryGraph,
        symbols: &SymbolTable,
        file_map: &HashMap<String, u64>,
        output: &Path,
    ) -> anyhow::Result<()> {
        let mut documents: Vec<serde_json::Value> = Vec::new();

        for (file_path, file_id) in file_map {
            let mut occurrences: Vec<serde_json::Value> = Vec::new();

            if let Ok(edges) = graph.get_outgoing(*file_id) {
                for edge in edges {
                    if edge.edge_type == EdgeType::Contains {
                        if let Ok(Some(node)) = graph.get_node(edge.target) {
                            if let Some(occ) = Self::node_to_occurrence(&node, symbols) {
                                occurrences.push(occ);
                            }
                            // Also check children
                            if let Ok(kids) = graph.get_outgoing(node.id) {
                                for ke in kids {
                                    if ke.edge_type == EdgeType::Defines || ke.edge_type == EdgeType::Contains {
                                        if let Ok(Some(kid)) = graph.get_node(ke.target) {
                                            if let Some(occ) = Self::node_to_occurrence(&kid, symbols) {
                                                occurrences.push(occ);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !occurrences.is_empty() {
                documents.push(serde_json::json!({
                    "relativePath": file_path,
                    "language": "SystemVerilog",
                    "occurrences": occurrences,
                }));
            }
        }

        let index = serde_json::json!({
            "metadata": {
                "version": 0,
                "project_root": "file://.",
                "tool_info": {
                    "name": "hdl-graph",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            },
            "documents": documents,
        });

        let json_str = serde_json::to_string_pretty(&index)?;
        std::fs::write(output, json_str)?;

        Ok(())
    }

    fn node_to_occurrence(node: &GraphNode, symbols: &SymbolTable) -> Option<serde_json::Value> {
        let (symbol_name, kind) = match &node.kind {
            NodeKind::Module { name } => {
                (format!("hdl module {}", symbols.resolve(*name)?), "Module")
            }
            NodeKind::ModuleInstance { name, module_type } => {
                let n = symbols.resolve(*name)?;
                let t = symbols.resolve(*module_type)?;
                (format!("hdl instance {} {}", n, t), "Instance")
            }
            NodeKind::Class { name, .. } => {
                (format!("hdl class {}", symbols.resolve(*name)?), "Class")
            }
            NodeKind::Package { name } => {
                (format!("hdl package {}", symbols.resolve(*name)?), "Package")
            }
            NodeKind::Interface { name } => {
                (format!("hdl interface {}", symbols.resolve(*name)?), "Interface")
            }
            NodeKind::Function { name, .. } => {
                (format!("hdl function {}", symbols.resolve(*name)?), "Function")
            }
            NodeKind::SignalDecl { name, .. } => {
                (format!("hdl signal {}", symbols.resolve(*name)?), "Variable")
            }
            NodeKind::ModulePort { name, .. } => {
                (format!("hdl port {}", symbols.resolve(*name)?), "Parameter")
            }
            _ => return None,
        };

        Some(serde_json::json!({
            "symbol": symbol_name,
            "kind": kind,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hdl_graph_storage::InMemoryGraph;

    #[test]
    fn test_scip_export_no_crash() {
        let graph = InMemoryGraph::new();
        let symbols = SymbolTable::new();
        let file_map = HashMap::new();
        let tmp = std::env::temp_dir().join("test_scip.json");

        let result = ScipExporter::export(&graph, &symbols, &file_map, &tmp);
        assert!(result.is_ok());
        let _ = std::fs::remove_file(&tmp);
    }
}
