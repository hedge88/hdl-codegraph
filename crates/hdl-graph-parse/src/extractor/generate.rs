use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract a loop_generate_construct: for (init; cond; iter) generate_block
    pub fn extract_generate_for(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let gen_id = self.next_id();
        nodes.push(self.make_node(node, gen_id, NodeKind::GenerateBlock {
                kind: GenerateKind::For,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: gen_id,
            edge_type: EdgeType::Contains,
        });

        // Extract the generate_block body
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "generate_block" {
                self.extract_generate_block(child, source, gen_id, nodes, edges);
            }
        }
    }

    /// Extract a conditional_generate_construct: if_generate_construct | case_generate_construct
    pub fn extract_generate_if(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "if_generate_construct" => {
                    self.extract_if_generate(child, source, parent_id, nodes, edges)
                }
                "case_generate_construct" => {
                    self.extract_case_generate(child, source, parent_id, nodes, edges)
                }
                _ => {}
            }
        }
    }

    /// Extract a generate_region: generate ... endgenerate
    pub fn extract_generate_region(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // Create a single container node for the generate region
        let gen_id = self.next_id();
        nodes.push(self.make_node(node, gen_id, NodeKind::GenerateBlock {
                kind: GenerateKind::Loop,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: gen_id,
            edge_type: EdgeType::Contains,
        });

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_generate_item(child, source, gen_id, nodes, edges);
        }
    }

    fn extract_if_generate(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // if_generate_construct: if (expr) generate_block [else generate_block]
        // Conservative: create a GenerateBlock node for each branch
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "generate_block" {
                let branch_id = self.next_id();
                nodes.push(self.make_node(child, branch_id, NodeKind::GenerateBlock {
                        kind: GenerateKind::If,
                    }, None));
                edges.push(Edge {
                    source: parent_id,
                    target: branch_id,
                    edge_type: EdgeType::Contains,
                });

                self.extract_generate_block(child, source, branch_id, nodes, edges);
            }
        }
    }

    fn extract_case_generate(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // case_generate_construct: case (expr) case_generate_item+ endcase
        // Conservative: create a GenerateBlock node for each case_generate_item
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "case_generate_item" {
                let branch_id = self.next_id();
                nodes.push(self.make_node(child, branch_id, NodeKind::GenerateBlock {
                        kind: GenerateKind::Case,
                    }, None));
                edges.push(Edge {
                    source: parent_id,
                    target: branch_id,
                    edge_type: EdgeType::Contains,
                });

                let mut cc = child.walk();
                for cchild in child.children(&mut cc) {
                    if cchild.kind() == "generate_block" {
                        self.extract_generate_block(cchild, source, branch_id, nodes, edges);
                    }
                }
            }
        }
    }

    fn extract_generate_block(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // generate_block is either a single _generate_item (inlined) or a begin-end
        // Just extract all item children
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_generate_item(child, source, parent_id, nodes, edges);
        }
    }

    /// Recursively extract items that can appear inside a generate block.
    fn extract_generate_item(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        match node.kind() {
            // Nested generate constructs
            "loop_generate_construct" => {
                self.extract_generate_for(node, source, parent_id, nodes, edges);
            }
            "conditional_generate_construct" => {
                self.extract_generate_if(node, source, parent_id, nodes, edges);
            }
            "generate_region" => {
                self.extract_generate_region(node, source, parent_id, nodes, edges);
            }

            // Standard module body items
            "module_instantiation" | "udp_instantiation" | "gate_instantiation"
            | "interface_instantiation" => {
                self.extract_instance(node, source, parent_id, nodes, edges);
            }
            "net_declaration" | "data_declaration" => {
                self.extract_signal(node, source, parent_id, nodes, edges);
            }
            "always_construct" => {
                self.extract_always(node, source, parent_id, nodes, edges);
            }
            "continuous_assign" => {
                let assign_id = self.next_id();
                nodes.push(self.make_node(node, assign_id, NodeKind::Assignment, None));
                edges.push(Edge {
                    source: parent_id,
                    target: assign_id,
                    edge_type: EdgeType::Contains,
                });
            }

            // Assertion items inside generates
            "concurrent_assertion_item" => {
                self.extract_concurrent_assertion_item(node, source, parent_id, nodes, edges);
            }
            "covergroup_declaration" => {
                self.extract_covergroup_declaration(node, source, parent_id, nodes, edges);
            }
            "property_declaration" => {
                self.extract_property_declaration(node, source, parent_id, nodes, edges);
            }
            "sequence_declaration" => {
                self.extract_sequence_declaration(node, source, parent_id, nodes, edges);
            }

            // DPI/Bind/Config
            "dpi_import_export" => {
                self.extract_dpi_import(node, source, parent_id, nodes, edges);
            }
            "bind_directive" => {
                self.extract_bind_directive(node, source, parent_id, nodes, edges);
            }

            "function_declaration" | "task_declaration" => {
                // Will extract in Phase 2
            }

            // Keywords, punctuation, attribute instances — skip
            _ => {}
        }
    }
}
