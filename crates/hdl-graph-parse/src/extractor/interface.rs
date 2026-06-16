use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract an interface declaration node.
    /// Returns the interface_id if successful, or None if the interface has no name.
    pub fn extract_interface(
        &mut self,
        node: Node,
        source: &[u8],
        scope_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) -> Option<u64> {
        // Try to get the name — may be a direct field or inside a header node
        let iface_name = if let Some(name_node) = node.child_by_field_name("name") {
            self.text(name_node, source).to_string()
        } else {
            // Look inside interface_nonansi_header or interface_ansi_header
            let mut found = String::new();
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "interface_nonansi_header" | "interface_ansi_header" => {
                        if let Some(name_node) = child.child_by_field_name("name") {
                            found = self.text(name_node, source).to_string();
                        }
                    }
                    _ => {}
                }
            }
            found
        };

        if iface_name.is_empty() {
            return None;
        }

        let name_sym = self.symbols.intern(&iface_name);
        let iface_id = self.next_id();
        nodes.push(self.make_node(node, iface_id, NodeKind::Interface { name: name_sym }, None));

        // Contains edge from containing scope to interface
        edges.push(Edge {
            source: scope_id,
            target: iface_id,
            edge_type: EdgeType::Contains,
        });

        // Extract body items — modport declarations and clocking blocks
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "interface_item" => {
                    self.extract_interface_item(child, source, iface_id, nodes, edges);
                }
                "modport_declaration" => {
                    self.extract_modport(child, source, iface_id, nodes, edges);
                }
                "clocking_declaration" => {
                    self.extract_clocking(child, source, iface_id, nodes, edges);
                }
                _ => {}
            }
        }

        Some(iface_id)
    }

    /// Extract body items from a named `interface_item` wrapper
    /// (used by the non-ANSI interface variant).
    fn extract_interface_item(
        &mut self,
        node: Node,
        source: &[u8],
        iface_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "modport_declaration" => {
                    self.extract_modport(child, source, iface_id, nodes, edges);
                }
                "clocking_declaration" => {
                    self.extract_clocking(child, source, iface_id, nodes, edges);
                }
                _ => {}
            }
        }
    }

    /// Extract a `modport_declaration` node, creating Modport nodes for
    /// each named modport item within it.
    fn extract_modport(
        &mut self,
        node: Node,
        source: &[u8],
        iface_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "modport_item" {
                let modport_name = self.text(child, source);

                // Trim trailing parenthesised port list
                let clean_name = modport_name
                    .split('(')
                    .next()
                    .unwrap_or(&modport_name)
                    .trim()
                    .to_string();

                if !clean_name.is_empty() {
                    let name_sym = self.symbols.intern(&clean_name);
                    let mp_id = self.next_id();
                    nodes.push(self.make_node(child, mp_id, NodeKind::Modport { name: name_sym }, None));
                    edges.push(Edge {
                        source: iface_id,
                        target: mp_id,
                        edge_type: EdgeType::Contains,
                    });
                }
            }
        }
    }

    /// Extract a clocking declaration node.
    /// Clocking blocks are noted in the graph but no dedicated node kind
    /// is defined; we create a simple placeholder node.
    fn extract_clocking(
        &mut self,
        node: Node,
        source: &[u8],
        iface_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut clocking_name = String::new();
        if let Some(name_node) = node.child_by_field_name("name") {
            clocking_name = self.text(name_node, source).to_string();
        }

        if !clocking_name.is_empty() {
            let name_sym = self.symbols.intern(&clocking_name);
            let ck_id = self.next_id();
            // Use SignalDecl as a lightweight placeholder for clocking blocks
            // since there's no dedicated Clocking variant.
            nodes.push(self.make_node(node, ck_id, NodeKind::SignalDecl {
                    name: name_sym,
                    kind: SignalKind::Logic,
                }, None));
            edges.push(Edge {
                source: iface_id,
                target: ck_id,
                edge_type: EdgeType::Contains,
            });
        }
    }
}
