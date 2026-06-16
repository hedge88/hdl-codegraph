use tree_sitter::Tree;
use hdl_graph_core::*;

impl super::GraphExtractor {
    /// Compute the difference between old and new parse results for a file.
    /// Returns a changeset with added/removed nodes and edges.
    pub fn extract_changeset(
        &mut self,
        _tree: &Tree,
        source: &[u8],
        file_id: u64,
        old_node_ids: &[u64],
        old_edges: &[Edge],
    ) -> super::ChangeSet {
        // Full re-extract: parse from scratch and diff
        let _start_id = self.next_id;
        let (new_nodes, new_edges) = self.extract(&_tree, source, file_id);

        // Collect added nodes (includes all new nodes)
        let added_nodes: Vec<(u64, GraphNode)> = new_nodes.iter().map(|n| (n.id, n.clone())).collect();

        // Old nodes to remove
        let new_ids: std::collections::HashSet<u64> = new_nodes.iter().map(|n| n.id).collect();
        let removed_node_ids: Vec<u64> = old_node_ids.iter().filter(|id| !new_ids.contains(id)).copied().collect();

        // Edges to remove
        let old_edge_set: std::collections::HashSet<(u64, u64)> =
            old_edges.iter().map(|e| (e.source, e.target)).collect();
        let new_edge_set: std::collections::HashSet<(u64, u64)> =
            new_edges.iter().map(|e| (e.source, e.target)).collect();
        let removed_edges: Vec<(u64, u64)> = old_edge_set.difference(&new_edge_set).copied().collect();

        super::ChangeSet {
            added_nodes,
            removed_node_ids,
            added_edges: new_edges,
            removed_edges,
        }
    }
}
