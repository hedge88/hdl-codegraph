use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract a dpi_import_export: import/export "DPI-C" function/task name ...
    pub fn extract_dpi_import(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut func_name = String::new();

        // Try direct field name access (works for export case)
        if let Some(name_node) = node.child_by_field_name("name") {
            func_name = self.text(name_node, source).to_string();
        }

        // Try the proto children (for import case)
        if func_name.is_empty() {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "dpi_function_proto" | "dpi_task_proto" => {
                        // The name is inside function_prototype / task_prototype
                        let mut pc = child.walk();
                        for proto_child in child.children(&mut pc) {
                            if let Some(name_node) = proto_child.child_by_field_name("name") {
                                func_name = self.text(name_node, source).to_string();
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if func_name.is_empty() {
            return;
        }

        let name_sym = self.symbols.intern(&func_name);
        let dpi_id = self.next_id();
        nodes.push(self.make_node(node, dpi_id, NodeKind::DPIImport {
                function_name: name_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: dpi_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Extract a bind_directive: bind target_scope[:target_instance] bind_instantiation
    pub fn extract_bind_directive(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut module_type = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "module_instantiation" | "interface_instantiation" | "program_instantiation" => {
                    if let Some(type_node) = child.child_by_field_name("instance_type") {
                        module_type = self.text(type_node, source).to_string();
                    }
                }
                _ => {}
            }
        }

        if module_type.is_empty() {
            return;
        }

        let type_sym = self.symbols.intern(&module_type);
        let bind_id = self.next_id();
        nodes.push(self.make_node(node, bind_id, NodeKind::BindDirective {
                module_type: type_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: bind_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Extract a config_declaration: config name ... endconfig
    pub fn extract_config_declaration(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let name = node
            .child_by_field_name("name")
            .map(|n| self.text(n, source).to_string())
            .unwrap_or_default();

        if name.is_empty() {
            return;
        }

        let name_sym = self.symbols.intern(&name);
        let config_id = self.next_id();
        nodes.push(self.make_node(node, config_id, NodeKind::ConfigBlock { name: name_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: config_id,
            edge_type: EdgeType::Contains,
        });
    }
}
