use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract a class declaration node.
    /// Returns the class_id if successful, or None if the class has no name.
    pub fn extract_class(
        &mut self,
        node: Node,
        source: &[u8],
        scope_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) -> Option<u64> {
        let class_name = node
            .child_by_field_name("name")
            .map(|n| self.text(n, source).to_string())
            .unwrap_or_default();

        if class_name.is_empty() {
            return None;
        }

        // Find parent class name from extends clause
        let parent_name = self.find_parent_class(node, source);

        let name_sym = self.symbols.intern(&class_name);
        let parent_sym = parent_name.as_ref().map(|p| self.symbols.intern(p));

        let class_id = self.next_id();
        nodes.push(self.make_node(node, class_id, NodeKind::Class {
                name: name_sym,
                parent: parent_sym,
            }, None));

        // Contains edge from containing scope (module, package, or outer class)
        edges.push(Edge {
            source: scope_id,
            target: class_id,
            edge_type: EdgeType::Contains,
        });

        // Extract class body items (methods, properties, nested classes)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_item" => {
                    self.extract_class_item(child, source, class_id, nodes, edges);
                }
                _ => {}
            }
        }

        Some(class_id)
    }

    /// Walk direct children of class_declaration to find the parent class
    /// specified in the extends clause.
    fn find_parent_class(&self, node: Node, source: &[u8]) -> Option<String> {
        let mut cursor = node.walk();
        let mut found_extends = false;
        for child in node.children(&mut cursor) {
            let t = self.text(child, source);
            if t == "extends" {
                found_extends = true;
            } else if found_extends && child.kind() == "class_type" {
                return Some(self.text(child, source).to_string());
            }
        }
        None
    }

    /// Extract items within a class body (property, method, nested class).
    fn extract_class_item(
        &mut self,
        node: Node,
        source: &[u8],
        class_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_property" => {
                    self.extract_class_property(child, source, class_id, nodes, edges);
                }
                "class_method" => {
                    self.extract_class_method(child, source, class_id, nodes, edges);
                }
                "class_declaration" => {
                    // Nested class
                    self.extract_class(child, source, class_id, nodes, edges);
                }
                _ => {}
            }
        }
    }

    /// Extract a class property (data member) from a class_property node.
    /// Creates NodeKind::Property nodes for each variable name found.
    /// Also checks for UVM factory registration typedefs.
    fn extract_class_property(
        &mut self,
        node: Node,
        source: &[u8],
        class_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "data_declaration" {
                // Standard property extraction
                for name in &self.collect_signal_names(child, source) {
                    let name_sym = self.symbols.intern(name);
                    let prop_id = self.next_id();
                    nodes.push(self.make_node(child, prop_id, NodeKind::Property { name: name_sym }, None));
                    edges.push(Edge {
                        source: class_id,
                        target: prop_id,
                        edge_type: EdgeType::Defines,
                    });
                }
                // UVM factory registration (e.g., typedef uvm_component_registry #(T, "T") type_id)
                self.extract_factory_registration(child, source, class_id, nodes, edges);
            }
        }
    }

    /// Extract a class method from a class_method node.
    /// Detects the `virtual` qualifier and the method name from the
    /// contained task_declaration, function_declaration, or class_constructor_declaration.
    fn extract_class_method(
        &mut self,
        node: Node,
        source: &[u8],
        class_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut is_virtual = false;
        let mut method_name = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "method_qualifier" => {
                    // Check if the qualifier contains the "virtual" keyword
                    let mut mc = child.walk();
                    for mchild in child.children(&mut mc) {
                        if self.text(mchild, source) == "virtual" {
                            is_virtual = true;
                        }
                    }
                }
                "function_declaration" | "task_declaration" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method_name = self.text(name_node, source).to_string();
                    }
                }
                "class_constructor_declaration" => {
                    method_name = "new".to_string();
                }
                _ => {
                    // Check for "virtual" keyword as a direct text child
                    if self.text(child, source) == "virtual" {
                        is_virtual = true;
                    }
                }
            }
        }

        if !method_name.is_empty() {
            let name_sym = self.symbols.intern(&method_name);
            let method_id = self.next_id();
            nodes.push(self.make_node(node, method_id, NodeKind::Method {
                    name: name_sym,
                    is_virtual,
                }, None));
            edges.push(Edge {
                source: class_id,
                target: method_id,
                edge_type: EdgeType::Defines,
            });
            // Scan method body for factory create/override calls
            self.extract_method_calls_in_subtree(node, source, method_id, nodes, edges);
        }
    }
}
