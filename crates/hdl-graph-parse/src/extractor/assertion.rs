use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract a concurrent_assertion_item: [block_identifier:] assert/assume/cover property(...)
    pub fn extract_concurrent_assertion_item(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let assert_id = self.next_id();
        nodes.push(self.make_node(node, assert_id, NodeKind::AssertProperty, None));
        edges.push(Edge {
            source: parent_id,
            target: assert_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Extract a covergroup_declaration: covergroup name ... endgroup
    pub fn extract_covergroup_declaration(
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
        let cg_id = self.next_id();
        nodes.push(self.make_node(node, cg_id, NodeKind::CoverGroup { name: name_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: cg_id,
            edge_type: EdgeType::Contains,
        });

        // Extract coverpoints
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "cover_point" {
                let cp_name = child
                    .child_by_field_name("name")
                    .map(|n| self.text(n, source).to_string())
                    .unwrap_or_default();

                if !cp_name.is_empty() {
                    let cp_sym = self.symbols.intern(&cp_name);
                    let cp_id = self.next_id();
                    nodes.push(self.make_node(child, cp_id, NodeKind::CoverPoint { name: cp_sym }, None));
                    edges.push(Edge {
                        source: cg_id,
                        target: cp_id,
                        edge_type: EdgeType::Contains,
                    });
                }
            }
        }
    }

    /// Extract a property_declaration: property name ... endproperty
    pub fn extract_property_declaration(
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
        let prop_id = self.next_id();
        nodes.push(self.make_node(node, prop_id, NodeKind::PropertyDecl { name: name_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: prop_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Extract a sequence_declaration: sequence name ... endsequence
    pub fn extract_sequence_declaration(
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
        let seq_id = self.next_id();
        nodes.push(self.make_node(node, seq_id, NodeKind::SequenceDecl { name: name_sym }, None));
        edges.push(Edge {
            source: parent_id,
            target: seq_id,
            edge_type: EdgeType::Contains,
        });
    }
}
