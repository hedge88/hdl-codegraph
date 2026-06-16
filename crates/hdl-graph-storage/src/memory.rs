use std::collections::HashMap;
use hdl_graph_core::*;

#[derive(Default)]
pub struct InMemoryGraph {
    nodes: HashMap<u64, GraphNode>,
    by_source: HashMap<u64, Vec<Edge>>,
    by_target: HashMap<u64, Vec<Edge>>,
    /// Index from InternedString.0 -> node IDs for O(1) name-based lookup.
    by_interned_name: HashMap<u64, Vec<u64>>,
    /// SymbolTable for resolving string names to InternedString ids.
    /// When names are interned through this table (via `symbols_mut()` or
    /// `add_node_with_name`), `get_by_name` can perform O(1) lookup.
    symbols: SymbolTable,
    next_id: u64,
}

impl InMemoryGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Access the SymbolTable for resolving InternedStrings.
    pub fn symbols(&self) -> &SymbolTable {
        &self.symbols
    }

    /// Mutably access the SymbolTable (e.g., to intern names before creating nodes).
    pub fn symbols_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbols
    }

    /// Add a node and index its human-readable name for O(1) lookup via `get_by_name`.
    /// The name is interned in this graph's internal SymbolTable.
    ///
    /// Use this instead of `add_node` when you want the node to be discoverable
    /// by name through the Graph trait's `get_by_name` method.
    pub fn add_node_with_name(&mut self, node: GraphNode, name: &str) -> CoreResult<u64> {
        let interned = self.symbols.intern(name);
        let interned_id = interned.0;
        let id = self.add_node(node)?;
        // Also ensure the graph's own interned id is in the index.
        // add_node already indexed by the node's own InternedString id,
        // but we also index by the graph's SymbolTable id so get_by_name works.
        let ids = self.by_interned_name.entry(interned_id).or_default();
        if !ids.contains(&id) {
            ids.push(id);
        }
        Ok(id)
    }

    /// O(1) lookup of node IDs by raw InternedString id.
    pub fn get_by_interned_name(&self, interned_id: u64) -> Vec<u64> {
        self.by_interned_name.get(&interned_id).cloned().unwrap_or_default()
    }
}

impl Graph for InMemoryGraph {
    fn add_node(&mut self, node: GraphNode) -> CoreResult<u64> {
        let id = if node.id == 0 { self.next_id } else { node.id };
        let mut node = node;
        node.id = id;
        if node.id >= self.next_id {
            self.next_id = node.id + 1;
        }
        // Index by interned name if the node kind has one
        if let Some(name_id) = extract_name_id(&node.kind) {
            self.by_interned_name.entry(name_id).or_default().push(id);
        }
        self.nodes.insert(id, node);
        self.by_source.entry(id).or_default();
        self.by_target.entry(id).or_default();
        Ok(id)
    }

    fn get_node(&self, id: u64) -> CoreResult<Option<GraphNode>> {
        Ok(self.nodes.get(&id).cloned())
    }

    fn add_edge(&mut self, edge: Edge) -> CoreResult<()> {
        self.by_source.entry(edge.source).or_default().push(edge.clone());
        self.by_target.entry(edge.target).or_default().push(edge);
        Ok(())
    }

    fn remove_node(&mut self, id: u64) -> CoreResult<()> {
        // Remove from interned name index
        if let Some(node) = self.nodes.get(&id) {
            if let Some(name_id) = extract_name_id(&node.kind) {
                if let Some(ids) = self.by_interned_name.get_mut(&name_id) {
                    ids.retain(|&nid| nid != id);
                    if ids.is_empty() {
                        self.by_interned_name.remove(&name_id);
                    }
                }
            }
        }
        // Remove outgoing edges from their targets' by_target lists
        if let Some(edges) = self.by_source.remove(&id) {
            for edge in &edges {
                if let Some(target_edges) = self.by_target.get_mut(&edge.target) {
                    target_edges.retain(|e| e.source != id);
                }
            }
        }
        // Remove incoming edges from their sources' by_source lists
        if let Some(edges) = self.by_target.remove(&id) {
            for edge in &edges {
                if let Some(source_edges) = self.by_source.get_mut(&edge.source) {
                    source_edges.retain(|e| e.target != id);
                }
            }
        }
        // Remove the node itself
        self.nodes.remove(&id);
        Ok(())
    }

    fn remove_edge(&mut self, source: u64, target: u64) -> CoreResult<()> {
        if let Some(edges) = self.by_source.get_mut(&source) {
            edges.retain(|e| e.source != source || e.target != target);
        }
        if let Some(edges) = self.by_target.get_mut(&target) {
            edges.retain(|e| e.source != source || e.target != target);
        }
        Ok(())
    }

    fn get_outgoing(&self, node_id: u64) -> CoreResult<Vec<Edge>> {
        Ok(self.by_source.get(&node_id).cloned().unwrap_or_default())
    }

    fn get_incoming(&self, node_id: u64) -> CoreResult<Vec<Edge>> {
        Ok(self.by_target.get(&node_id).cloned().unwrap_or_default())
    }

    fn node_count(&self) -> usize { self.nodes.len() }
    fn edge_count(&self) -> usize { self.by_source.values().map(|v| v.len()).sum() }
    fn all_nodes(&self) -> Vec<GraphNode> { self.nodes.values().cloned().collect() }

    fn get_by_name(&self, name: &str) -> CoreResult<Vec<GraphNode>> {
        // Look up the name's interned id in the graph's SymbolTable,
        // then use the by_interned_name index for O(1) retrieval.
        if let Some(interned_id) = self.symbols.resolve_id(name) {
            let ids = self.get_by_interned_name(interned_id);
            Ok(ids.iter().filter_map(|&id| self.nodes.get(&id).cloned()).collect())
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(kind: NodeKind) -> GraphNode {
        GraphNode { id: 0, kind, ..Default::default() }
    }

    #[test]
    fn test_add_node() {
        let mut g = InMemoryGraph::new();
        let id = g.add_node(make_node(NodeKind::SourceFile)).unwrap();
        assert_eq!(g.node_count(), 1);
        assert!(g.get_node(id).unwrap().is_some());
    }

    #[test]
    fn test_add_edge() {
        let mut g = InMemoryGraph::new();
        let a = g.add_node(make_node(NodeKind::SourceFile)).unwrap();
        let b = g.add_node(make_node(NodeKind::Module { name: InternedString(1) })).unwrap();
        g.add_edge(Edge { source: a, target: b, edge_type: EdgeType::Contains }).unwrap();
        assert_eq!(g.edge_count(), 1);
        let out = g.get_outgoing(a).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].target, b);
    }

    #[test]
    fn test_incoming() {
        let mut g = InMemoryGraph::new();
        let a = g.add_node(make_node(NodeKind::SourceFile)).unwrap();
        let b = g.add_node(make_node(NodeKind::Module { name: InternedString(1) })).unwrap();
        g.add_edge(Edge { source: a, target: b, edge_type: EdgeType::Contains }).unwrap();
        let inc = g.get_incoming(b).unwrap();
        assert_eq!(inc.len(), 1);
        assert_eq!(inc[0].source, a);
    }

    #[test]
    fn test_interned_name_index() {
        let mut g = InMemoryGraph::new();
        let id = g.add_node(make_node(NodeKind::Module { name: InternedString(42) })).unwrap();

        // Lookup by raw interned id
        let ids = g.get_by_interned_name(42);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], id);

        // No match for different id
        assert!(g.get_by_interned_name(99).is_empty());
    }

    #[test]
    fn test_interned_name_index_multiple_same_name() {
        let mut g = InMemoryGraph::new();
        let id1 = g.add_node(make_node(
            NodeKind::SignalDecl { name: InternedString(7), kind: SignalKind::Wire },
        )).unwrap();
        let id2 = g.add_node(make_node(
            NodeKind::SignalDecl { name: InternedString(7), kind: SignalKind::Reg },
        )).unwrap();

        let ids = g.get_by_interned_name(7);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_interned_name_index_remove_node() {
        let mut g = InMemoryGraph::new();
        let id = g.add_node(make_node(NodeKind::Module { name: InternedString(10) })).unwrap();

        assert_eq!(g.get_by_interned_name(10).len(), 1);
        g.remove_node(id).unwrap();
        assert!(g.get_by_interned_name(10).is_empty());
    }

    #[test]
    fn test_interned_name_index_unnamed_nodes() {
        let mut g = InMemoryGraph::new();
        g.add_node(make_node(NodeKind::SourceFile)).unwrap();
        g.add_node(make_node(NodeKind::Assignment)).unwrap();

        // SourceFile and Assignment have no name field
        assert!(g.get_by_interned_name(0).is_empty());
    }

    #[test]
    fn test_add_node_with_name() {
        let mut g = InMemoryGraph::new();
        let id = g.add_node_with_name(
            make_node(NodeKind::Module { name: InternedString(0) }),
            "my_module",
        ).unwrap();

        // get_by_name finds it
        let nodes = g.get_by_name("my_module").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, id);

        // Different name returns empty
        assert!(g.get_by_name("other").unwrap().is_empty());
    }

    #[test]
    fn test_get_by_name_via_symbols_mut() {
        let mut g = InMemoryGraph::new();
        // Intern name through the graph's SymbolTable
        let name = g.symbols_mut().intern("counter");
        let id = g.add_node(make_node(
            NodeKind::SignalDecl { name, kind: SignalKind::Logic },
        )).unwrap();

        let nodes = g.get_by_name("counter").unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, id);
    }

    #[test]
    fn test_graph_node_name_str() {
        let mut symbols = SymbolTable::new();
        let name = symbols.intern("clk");
        let node = GraphNode {
            id: 1,
            kind: NodeKind::SignalDecl { name, kind: SignalKind::Wire },
            ..Default::default()
        };
        assert_eq!(node.name_str(&symbols), Some("clk".to_string()));
        assert_eq!(node.name_interned_id(), Some(name.0));

        let node2 = GraphNode { id: 2, kind: NodeKind::SourceFile, ..Default::default() };
        assert_eq!(node2.name_str(&symbols), None);
        assert_eq!(node2.name_interned_id(), None);
    }

    #[test]
    fn test_extract_name_id_covers_all_named_variants() {
        let named = vec![
            NodeKind::Module { name: InternedString(1) },
            NodeKind::SignalDecl { name: InternedString(1), kind: SignalKind::Wire },
            NodeKind::ModulePort { name: InternedString(1), direction: PortDirection::Input },
            NodeKind::ModuleInstance { name: InternedString(1), module_type: InternedString(2) },
            NodeKind::Function { name: InternedString(1), is_task: false },
            NodeKind::VariableRef { name: InternedString(1) },
            NodeKind::CallSite { target: InternedString(1) },
            NodeKind::Class { name: InternedString(1), parent: None },
            NodeKind::Method { name: InternedString(1), is_virtual: false },
            NodeKind::Property { name: InternedString(1) },
            NodeKind::Package { name: InternedString(1) },
            NodeKind::Interface { name: InternedString(1) },
            NodeKind::Modport { name: InternedString(1) },
            NodeKind::TLMPort { name: InternedString(1), direction: TLMDirection::Blocking },
            NodeKind::SequenceDecl { name: InternedString(1) },
            NodeKind::PropertyDecl { name: InternedString(1) },
            NodeKind::CoverGroup { name: InternedString(1) },
            NodeKind::CoverPoint { name: InternedString(1) },
            NodeKind::DPIImport { function_name: InternedString(1) },
            NodeKind::ConfigDBSet { field: InternedString(1) },
            NodeKind::ConfigDBGet { field: InternedString(1) },
            NodeKind::FactoryReg { type_name: InternedString(1), base_type: InternedString(2) },
            NodeKind::FactoryCreate { type_name: InternedString(1) },
            NodeKind::FactoryOverride { original_type: InternedString(1), override_type: InternedString(2) },
            NodeKind::ConfigBlock { name: InternedString(1) },
        ];
        for kind in &named {
            assert!(extract_name_id(kind).is_some(), "Expected name for {:?}", kind);
        }

        let unnamed = vec![
            NodeKind::SourceFile,
            NodeKind::GenerateBlock { kind: GenerateKind::If },
            NodeKind::AlwaysBlock { kind: AlwaysKind::Combinational },
            NodeKind::Assignment,
            NodeKind::BeginBlock { label: None },
            NodeKind::PackageImport { source: InternedString(1) },
            NodeKind::TLMBinding,
            NodeKind::AssertProperty,
            NodeKind::BindDirective { module_type: InternedString(1) },
        ];
        for kind in &unnamed {
            assert!(extract_name_id(kind).is_none(), "Expected no name for {:?}", kind);
        }
    }
}
