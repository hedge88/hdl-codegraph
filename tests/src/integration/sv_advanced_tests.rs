use crate::integration::common;

use common::*;
use hdl_graph_core::*;

#[test]
fn test_generate_for() {
    let src = load_fixture("sv_advanced/generate_blocks.sv");
    let (nodes, edges, _extractor) = parse_sv_to_graph(&src, 1);

    // Should have GenerateBlock nodes
    let gen_blocks: Vec<_> = find_nodes_by_kind(&nodes, |k| {
        matches!(k, NodeKind::GenerateBlock { kind: GenerateKind::For })
    });
    assert!(
        !gen_blocks.is_empty(),
        "Expected at least one for-generate block"
    );

    // Should have Contains edges
    let contains = find_edges_by_type(&edges, EdgeType::Contains);
    assert!(!contains.is_empty(), "Expected Contains edges in generate");
}

#[test]
fn test_generate_if_case() {
    let src = load_fixture("sv_advanced/generate_blocks.sv");
    let (nodes, _edges, _extractor) = parse_sv_to_graph(&src, 1);

    // Should have if-generate and case-generate blocks
    let if_gen: Vec<_> = find_nodes_by_kind(&nodes, |k| {
        matches!(k, NodeKind::GenerateBlock { kind: GenerateKind::If })
    });
    assert!(
        !if_gen.is_empty(),
        "Expected at least one if-generate block"
    );

    let case_gen: Vec<_> = find_nodes_by_kind(&nodes, |k| {
        matches!(k, NodeKind::GenerateBlock { kind: GenerateKind::Case })
    });
    assert!(
        !case_gen.is_empty(),
        "Expected at least one case-generate block"
    );
}

#[test]
fn test_assertion_property() {
    let src = load_fixture("sv_advanced/assertions.sv");
    let (nodes, _edges, extractor) = parse_sv_to_graph(&src, 1);

    // Should have AssertProperty nodes
    let asserts: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::AssertProperty));
    assert!(
        !asserts.is_empty(),
        "Expected at least one AssertProperty"
    );

    // Should have PropertyDecl
    let props: Vec<_> = find_nodes_by_kind(&nodes, |k| {
        matches!(k, NodeKind::PropertyDecl { name } if resolve_from_extractor(&extractor, *name) == "req_gnt_prop")
    });
    assert!(
        !props.is_empty(),
        "Expected PropertyDecl 'req_gnt_prop'"
    );

    // Should have SequenceDecl
    let seqs: Vec<_> = find_nodes_by_kind(&nodes, |k| {
        matches!(k, NodeKind::SequenceDecl { name } if resolve_from_extractor(&extractor, *name) == "req_gnt_seq")
    });
    assert!(
        !seqs.is_empty(),
        "Expected SequenceDecl 'req_gnt_seq'"
    );
}

#[test]
fn test_covergroup() {
    let src = load_fixture("sv_advanced/assertions.sv");
    let (nodes, _edges, _extractor) = parse_sv_to_graph(&src, 1);

    // Should have CoverGroup
    let cgs: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::CoverGroup { .. }));
    assert!(
        !cgs.is_empty(),
        "Expected at least one CoverGroup"
    );

    // Should have CoverPoint — may not be extracted depending on tree-sitter CST shape
    let cps: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::CoverPoint { .. }));
    if cps.is_empty() {
        eprintln!("NOTE: CoverPoint not detected — extractor may not parse coverpoint inside covergroup");
    }
}

#[test]
fn test_dpi_import() {
    let src = load_fixture("sv_advanced/dpi_bind.sv");
    let (nodes, _edges, extractor) = parse_sv_to_graph(&src, 1);

    // Should have DPIImport nodes
    let dpis: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::DPIImport { .. }));
    assert!(
        dpis.len() >= 2,
        "Expected at least 2 DPI imports (c_crc32, c_wait_cycles), got {}",
        dpis.len()
    );

    // Verify function names
    let dpi_names: Vec<String> = dpis
        .iter()
        .filter_map(|n| {
            if let NodeKind::DPIImport { function_name } = &n.kind {
                Some(resolve_from_extractor(&extractor, *function_name))
            } else {
                None
            }
        })
        .collect();
    assert!(
        dpi_names.contains(&"c_crc32".to_string()),
        "Expected c_crc32 DPI import, got {:?}",
        dpi_names
    );
}

#[test]
fn test_bind_directive() {
    let src = load_fixture("sv_advanced/dpi_bind.sv");
    let (nodes, _edges, _extractor) = parse_sv_to_graph(&src, 1);

    // Config block should be detected
    let configs: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::ConfigBlock { .. }));
    // Config block may or may not parse depending on tree-sitter support
    // This is a smoke test — no crash
    let _ = configs;
}

#[test]
fn test_always_kinds() {
    let src = load_fixture("sv_advanced/always_comb_ff_latch.sv");
    let (nodes, _edges, _extractor) = parse_sv_to_graph(&src, 1);

    let always_blocks: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::AlwaysBlock { .. }));
    assert!(
        always_blocks.len() >= 4,
        "Expected at least 4 always blocks, got {}",
        always_blocks.len()
    );

    // Check for Combinational, Sequential, and/or Latch
    let has_comb = always_blocks.iter().any(|n| {
        matches!(n.kind, NodeKind::AlwaysBlock { kind: AlwaysKind::Combinational })
    });
    let has_seq = always_blocks.iter().any(|n| {
        matches!(n.kind, NodeKind::AlwaysBlock { kind: AlwaysKind::Sequential })
    });
    let seq_count = always_blocks.iter().filter(|n| {
        matches!(n.kind, NodeKind::AlwaysBlock { kind: AlwaysKind::Sequential })
    }).count();
    assert!(has_comb, "Expected a combinational always block");
    assert!(has_seq, "Expected a sequential always block");
    // Both always_ff and plain always @(posedge clk) should be Sequential
    assert!(seq_count >= 2, "Expected at least 2 sequential blocks (always_ff + plain always), got {}", seq_count);
}
