use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Extract a package declaration node.
    /// Returns the package_id if successful, or None if the package has no name.
    pub fn extract_package(
        &mut self,
        node: Node,
        source: &[u8],
        _file_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) -> Option<u64> {
        let pkg_name = node
            .child_by_field_name("name")
            .map(|n| self.text(n, source).to_string())
            .unwrap_or_default();

        if pkg_name.is_empty() {
            return None;
        }

        let name_sym = self.symbols.intern(&pkg_name);
        let pkg_id = self.next_id();
        nodes.push(self.make_node(node, pkg_id, NodeKind::Package { name: name_sym }, None));

        // Contains edge from source file to package
        edges.push(Edge {
            source: _file_id,
            target: pkg_id,
            edge_type: EdgeType::Contains,
        });

        // Extract package body items — import and export declarations
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "package_item" => {
                    self.extract_package_item(child, source, pkg_id, nodes, edges);
                }
                _ => {}
            }
        }

        Some(pkg_id)
    }

    /// Extract a single package body item.
    fn extract_package_item(
        &mut self,
        node: Node,
        source: &[u8],
        pkg_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "package_import_declaration" => {
                    self.extract_package_import(child, source, pkg_id, nodes, edges);
                }
                "package_export_declaration" => {
                    self.extract_package_export(child, source, pkg_id, nodes, edges);
                }
                _ => {}
            }
        }
    }

    /// Extract a `package_import_declaration` node.
    /// Creates a PackageImport node for each imported item.
    fn extract_package_import(
        &mut self,
        node: Node,
        source: &[u8],
        pkg_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "package_import_item" {
                let source_path = self.text(child, source).to_string();
                if !source_path.is_empty() {
                    let src_sym = self.symbols.intern(&source_path);
                    let import_id = self.next_id();
                    nodes.push(self.make_node(child, import_id, NodeKind::PackageImport { source: src_sym }, None));
                    edges.push(Edge {
                        source: pkg_id,
                        target: import_id,
                        edge_type: EdgeType::Contains,
                    });
                }
            }
        }
    }

    /// Extract a `package_export_declaration` node.
    /// Creates a PackageImport node for each exported item.
    fn extract_package_export(
        &mut self,
        node: Node,
        source: &[u8],
        pkg_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // Exports can be '*::*' (wildcard) or a list of package_import_item
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "package_import_item" => {
                    let source_path = self.text(child, source).to_string();
                    if !source_path.is_empty() {
                        let src_sym = self.symbols.intern(&source_path);
                        let export_id = self.next_id();
                        nodes.push(self.make_node(child, export_id, NodeKind::PackageImport { source: src_sym }, None));
                        edges.push(Edge {
                            source: pkg_id,
                            target: export_id,
                            edge_type: EdgeType::Contains,
                        });
                    }
                }
                _ => {
                    // Check for '*::*' wildcard export
                    let t = self.text(child, source);
                    if t == "*::*" {
                        let src_sym = self.symbols.intern("*::*");
                        let export_id = self.next_id();
                        nodes.push(self.make_node(child, export_id, NodeKind::PackageImport { source: src_sym }, None));
                        edges.push(Edge {
                            source: pkg_id,
                            target: export_id,
                            edge_type: EdgeType::Contains,
                        });
                    }
                }
            }
        }
    }
}
