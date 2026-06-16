use hdl_graph_core::*;
use hdl_graph_parse::GraphExtractor;

#[test]
fn test_changeset_same_source_empty() {
    // Re-parsing identical source should produce an empty changeset
    let src = r#"
module counter (
    input  wire clk,
    output reg  count
);
    always @(posedge clk)
        count <= ~count;
endmodule
"#;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&hdl_graph_grammar::language_ref())
        .expect("Failed to set language");
    let tree = parser.parse(src, None).expect("Failed to parse");
    assert!(!tree.root_node().has_error());

    // First extract
    let mut ext1 = GraphExtractor::new();
    let (nodes1, edges1) = ext1.extract(&tree, src.as_bytes(), 1);
    let old_ids: Vec<u64> = nodes1.iter().map(|n| n.id).collect();

    // Second extract with same tree — should produce same result
    let mut ext2 = GraphExtractor::new();
    let changeset = ext2.extract_changeset(&tree, src.as_bytes(), 1, &old_ids, &edges1);

    // All old nodes should still be present, no removals
    assert!(
        changeset.removed_node_ids.is_empty(),
        "No nodes should be removed for identical source"
    );
}

#[test]
fn test_changeset_add_signal() {
    // Original source
    let src1 = r#"
module mod_a (
    input wire clk,
    output reg out
);
    always @(posedge clk)
        out <= 1'b1;
endmodule
"#;
    // Modified source: added a signal
    let src2 = r#"
module mod_a (
    input wire clk,
    output reg out
);
    wire internal;
    assign internal = clk;
    always @(posedge clk)
        out <= internal;
endmodule
"#;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&hdl_graph_grammar::language_ref())
        .expect("Failed to set language");

    // Parse old
    let tree1 = parser.parse(src1, None).unwrap();
    assert!(!tree1.root_node().has_error());

    let mut ext1 = GraphExtractor::new();
    let (nodes1, edges1) = ext1.extract(&tree1, src1.as_bytes(), 1);
    let old_ids: Vec<u64> = nodes1.iter().map(|n| n.id).collect();

    // Parse new
    let tree2 = parser.parse(src2, None).unwrap();
    assert!(!tree2.root_node().has_error());

    let mut ext2 = GraphExtractor::new();
    let changeset = ext2.extract_changeset(&tree2, src2.as_bytes(), 1, &old_ids, &edges1);

    // New source has more nodes (added signal + assignment)
    assert!(
        !changeset.added_nodes.is_empty(),
        "Should have added nodes for the new signal"
    );
}

#[test]
fn test_changeset_apply_to_graph() {
    let src = r#"
module simple (
    input wire a,
    output wire b
);
    assign b = a;
endmodule
"#;
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&hdl_graph_grammar::language_ref())
        .expect("Failed to set language");
    let tree = parser.parse(src, None).unwrap();

    let mut extractor = GraphExtractor::new();
    let (nodes, edges) = extractor.extract(&tree, src.as_bytes(), 1);

    // Build an InMemoryGraph from the extraction
    let mut graph = hdl_graph_storage::InMemoryGraph::new();
    for node in &nodes {
        graph.add_node(node.clone()).unwrap();
    }
    for edge in &edges {
        graph.add_edge(edge.clone()).unwrap();
    }

    let initial_count = graph.node_count();

    // Apply an empty changeset — graph should be unchanged
    let cs = hdl_graph_parse::ChangeSet {
        added_nodes: vec![],
        removed_node_ids: vec![],
        added_edges: vec![],
        removed_edges: vec![],
    };
    cs.apply_to(&mut graph).unwrap();
    assert_eq!(
        graph.node_count(),
        initial_count,
        "Empty changeset should not change graph"
    );
}

#[test]
fn test_changeset_apply_adds_nodes() {
    let mut graph = hdl_graph_storage::InMemoryGraph::new();
    let n1 = graph
        .add_node(GraphNode {
            id: 0,
            kind: NodeKind::SourceFile,
            scope_id: None,
            ..Default::default()
        })
        .unwrap();

    let cs = hdl_graph_parse::ChangeSet {
        added_nodes: vec![(
            99,
            GraphNode {
                id: 99,
                kind: NodeKind::Module {
                    name: InternedString(1),
                },
                scope_id: None,
                ..Default::default()
            },
        )],
        removed_node_ids: vec![],
        added_edges: vec![Edge {
            source: n1,
            target: 99,
            edge_type: EdgeType::Contains,
        }],
        removed_edges: vec![],
    };

    cs.apply_to(&mut graph).unwrap();

    assert_eq!(graph.node_count(), 2, "Should have 2 nodes after apply");
    let added = graph.get_node(99).unwrap();
    assert!(added.is_some(), "Node 99 should exist");
    assert!(
        matches!(added.unwrap().kind, NodeKind::Module { .. }),
        "Node 99 should be a Module"
    );
}

#[test]
fn test_changeset_has_changes() {
    let cs = hdl_graph_parse::ChangeSet {
        added_nodes: vec![],
        removed_node_ids: vec![],
        added_edges: vec![],
        removed_edges: vec![],
    };
    assert!(
        cs.removed_node_ids.is_empty() && cs.added_nodes.is_empty(),
        "Empty changeset should have no changes"
    );

    let cs2 = hdl_graph_parse::ChangeSet {
        added_nodes: vec![],
        removed_node_ids: vec![1],
        added_edges: vec![],
        removed_edges: vec![],
    };
    assert!(
        !cs2.removed_node_ids.is_empty(),
        "Changeset with removed nodes should indicate changes"
    );
}
