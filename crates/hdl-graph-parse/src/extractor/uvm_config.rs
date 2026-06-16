use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Check if a method_call is a `uvm_config_db#(T)::set(...)` invocation.
    fn is_uvm_config_db_set(&self, node: Node, source: &[u8]) -> Option<String> {
        self.extract_config_db_field(node, source, "set")
    }

    /// Check if a method_call is a `uvm_config_db#(T)::get(...)` invocation.
    fn is_uvm_config_db_get(&self, node: Node, source: &[u8]) -> Option<String> {
        self.extract_config_db_field(node, source, "get")
    }

    /// Common logic: check if a method_call node is a uvm_config_db::set/get
    /// call. Returns the field argument text if it matches.
    fn extract_config_db_field(
        &self,
        node: Node,
        source: &[u8],
        expected_name: &str,
    ) -> Option<String> {
        // Check that the method_call has a class_type root and the right method name
        let mut has_uvm_config_db = false;
        let mut method_name = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_type" => {
                    let text = self.text(child, source);
                    if text.starts_with("uvm_config_db") {
                        has_uvm_config_db = true;
                    }
                }
                "method_call_body" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method_name = self.text(name_node, source).to_string();
                    }
                }
                _ => {}
            }
        }

        if !has_uvm_config_db || method_name != expected_name {
            return None;
        }

        // Extract the field argument (3rd positional argument, index 2)
        self.get_nth_argument(node, source, 2)
    }

    /// Get the text of the n-th positional argument from a method_call node.
    fn get_nth_argument(
        &self,
        node: Node,
        source: &[u8],
        index: usize,
    ) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "method_call_body" {
                if let Some(args_node) = child.child_by_field_name("arguments") {
                    let mut ac = args_node.walk();
                    for arg_child in args_node.children(&mut ac) {
                        if arg_child.kind() == "list_of_arguments" {
                            let mut idx = 0usize;
                            let mut lac = arg_child.walk();
                            for expr in arg_child.children(&mut lac) {
                                if expr.kind() == "expression" {
                                    if idx == index {
                                        // Return the text, stripping quotes if it's a string literal
                                        let raw = self.text(expr, source);
                                        return Some(
                                            raw.trim_matches('"').to_string(),
                                        );
                                    }
                                    idx += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Extract a `uvm_config_db#(T)::set(...)` invocation.
    ///
    /// Creates a ConfigDBSet node under the parent scope.
    pub fn extract_config_db_set(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let field = match self.is_uvm_config_db_set(node, source) {
            Some(f) => f,
            None => return,
        };

        let field_sym = self.symbols.intern(&field);
        let cfg_id = self.next_id();
        nodes.push(self.make_node(node, cfg_id, NodeKind::ConfigDBSet { field: field_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: cfg_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Extract a `uvm_config_db#(T)::get(...)` invocation.
    ///
    /// Creates a ConfigDBGet node under the parent scope.
    pub fn extract_config_db_get(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let field = match self.is_uvm_config_db_get(node, source) {
            Some(f) => f,
            None => return,
        };

        let field_sym = self.symbols.intern(&field);
        let cfg_id = self.next_id();
        nodes.push(self.make_node(node, cfg_id, NodeKind::ConfigDBGet { field: field_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: cfg_id,
            edge_type: EdgeType::Contains,
        });
    }
}
