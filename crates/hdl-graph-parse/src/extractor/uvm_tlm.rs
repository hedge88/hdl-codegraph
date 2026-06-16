use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Check if a data_declaration node declares a TLM port.
    /// Returns Some(TLMDirection) if it does, None otherwise.
    pub fn detect_tlm_port(&self, node: Node, source: &[u8]) -> Option<TLMDirection> {
        // Fast path: check the full text of the node for TLM keywords
        let node_text = self.text(node, source);
        let fast_result = Self::tlm_direction_from_type_name(node_text);
        if fast_result.is_some() {
            return fast_result;
        }

        // Slow path: walk the CST for type information
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "data_type_or_implicit" {
                let result = self.tlm_direction_from_data_type(child, source);
                if result.is_some() {
                    return result;
                }
            }
        }
        None
    }

    /// Walk data_type / class_type / simple_identifier to find a TLM type name.
    fn tlm_direction_from_data_type(&self, node: Node, source: &[u8]) -> Option<TLMDirection> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "data_type" | "implicit_data_type" => {
                    return self.tlm_direction_from_data_type(child, source);
                }
                "class_type" => {
                    let mut c2 = child.walk();
                    for gc in child.children(&mut c2) {
                        if gc.kind() == "ps_class_identifier" {
                            let type_name = self.text(gc, source);
                            let result = Self::tlm_direction_from_type_name(type_name);
                            if result.is_some() {
                                return result;
                            }
                        }
                        // Also check direct text fallback
                        let text = self.text(gc, source);
                        let result = Self::tlm_direction_from_type_name(text);
                        if result.is_some() {
                            return result;
                        }
                    }
                }
                "simple_identifier" => {
                    let text = self.text(child, source);
                    let result = Self::tlm_direction_from_type_name(text);
                    if result.is_some() {
                        return result;
                    }
                }
                _ => {}
            }
        }
        // Fallback: check the full text of the node
        let text = self.text(node, source);
        Self::tlm_direction_from_type_name(text)
    }

    /// Determine TLM direction from a type name string.
    fn tlm_direction_from_type_name(type_name: &str) -> Option<TLMDirection> {
        if type_name.contains("uvm_analysis_port") {
            Some(TLMDirection::AnalysisPort)
        } else if type_name.contains("uvm_analysis_imp") || type_name.contains("uvm_analysis_export") {
            Some(TLMDirection::AnalysisExport)
        } else if type_name.contains("uvm_blocking") {
            Some(TLMDirection::Blocking)
        } else if type_name.contains("uvm_nonblocking") {
            Some(TLMDirection::Nonblocking)
        } else if type_name.contains("uvm_tlm_fifo") {
            Some(TLMDirection::Fifo)
        } else {
            None
        }
    }

    /// Extract a TLM port declaration node.
    ///
    /// node must be a data_declaration whose type is a TLM port type.
    /// Creates a NodeKind::TLMPort for each variable name found.
    pub fn extract_tlm_port(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let dir = match self.detect_tlm_port(node, source) {
            Some(d) => d,
            None => return,
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "list_of_variable_decl_assignments" {
                let mut c2 = child.walk();
                for var_decl in child.children(&mut c2) {
                    if var_decl.kind() == "variable_decl_assignment" {
                        if let Some(name_node) = var_decl.child_by_field_name("name") {
                            let port_name = self.text(name_node, source);
                            let name_sym = self.symbols.intern(port_name);
                            let port_id = self.next_id();
                            nodes.push(self.make_node(var_decl, port_id, NodeKind::TLMPort {
                                    name: name_sym,
                                    direction: dir.clone(),
                                }, None));
                            edges.push(Edge {
                                source: parent_id,
                                target: port_id,
                                edge_type: EdgeType::Defines,
                            });
                        }
                    }
                }
            }
        }
    }

    /// Extract a TLM .connect() call from a method_call node.
    ///
    /// Creates a TLMBinding node under the parent scope.
    pub fn extract_tlm_connect(
        &mut self,
        _node: Node,
        _source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // Create a TLMBinding node to mark the connect call location
        let bind_id = self.next_id();
        nodes.push(self.make_node(_node, bind_id, NodeKind::TLMBinding, None));
        edges.push(Edge {
            source: parent_id,
            target: bind_id,
            edge_type: EdgeType::Contains,
        });
    }
}
