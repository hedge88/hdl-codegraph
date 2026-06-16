use crate::edge::Edge;
use crate::error::CoreResult;
use crate::node::{GraphNode, NodeId};

pub trait Graph: Send + Sync {
    fn add_node(&mut self, node: GraphNode) -> CoreResult<NodeId>;
    fn get_node(&self, id: NodeId) -> CoreResult<Option<GraphNode>>;
    fn add_edge(&mut self, edge: Edge) -> CoreResult<()>;
    fn remove_node(&mut self, id: NodeId) -> CoreResult<()>;
    fn remove_edge(&mut self, source: NodeId, target: NodeId) -> CoreResult<()>;
    fn get_outgoing(&self, node_id: NodeId) -> CoreResult<Vec<Edge>>;
    fn get_incoming(&self, node_id: NodeId) -> CoreResult<Vec<Edge>>;
    fn node_count(&self) -> usize;
    fn edge_count(&self) -> usize;
    fn all_nodes(&self) -> Vec<GraphNode>;

    /// Look up nodes by human-readable name.
    /// Implementations with a name index provide O(1) lookup.
    fn get_by_name(&self, name: &str) -> CoreResult<Vec<GraphNode>>;
}
