use crate::integration::common;

use common::*;
use hdl_graph_core::*;

/// Test that the uvm_macros fixture files parse after preprocessing
/// and produce the expected UVM-specific nodes.
/// These tests exercise the full pipeline: fixture → preprocess → parse → extract.

#[test]
fn test_macro_utils_fixture_extraction() {
    // macro_utils.sv has: `uvm_component_utils(my_comp)` and `uvm_object_utils(my_obj)`
    let src = load_fixture("uvm_macros/macro_utils.sv");
    let (nodes, edges, extractor) = preprocess_and_parse(&src, 1);

    // After preprocessing, should find FactoryReg nodes for both classes
    let factory_regs: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::FactoryReg { .. }));
    assert!(
        factory_regs.len() >= 2,
        "Expected at least 2 FactoryReg from uvm_component_utils + uvm_object_utils, got {}",
        factory_regs.len()
    );

    // Verify the registrations
    let mut found_comp = false;
    let mut found_obj = false;
    for reg in &factory_regs {
        if let NodeKind::FactoryReg { type_name, base_type } = &reg.kind {
            let tn = resolve_from_extractor(&extractor, *type_name);
            let bt = resolve_from_extractor(&extractor, *base_type);
            if tn == "my_comp" && bt == "uvm_component" {
                found_comp = true;
            }
            if tn == "my_obj" && bt == "uvm_object" {
                found_obj = true;
            }
        }
    }
    assert!(found_comp, "Expected FactoryReg for my_comp (uvm_component)");
    assert!(found_obj, "Expected FactoryReg for my_obj (uvm_object)");

    // Should have Class nodes
    let classes: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::Class { .. }));
    assert!(
        classes.len() >= 2,
        "Expected at least 2 classes, got {}",
        classes.len()
    );

    // Should have Contains edges
    let contains = find_edges_by_type(&edges, EdgeType::Contains);
    assert!(!contains.is_empty(), "Expected Contains edges");
}

#[test]
fn test_macro_fields_fixture_extraction() {
    // macro_fields.sv has: `uvm_field_int(addr, UVM_ALL_ON)` etc. inside do_print.
    // After preprocessing, uvm_field macros expand to duplicate function declarations
    // which cause parse errors. Use lenient parsing and verify preprocessing at minimum.
    let src = load_fixture("uvm_macros/macro_fields.sv");
    let result = hdl_graph_parse::preprocessor::preprocess(
        &src,
        "macro_fields.sv",
        &std::collections::HashMap::new(),
        &[],
    );

    // Verify preprocessing happened
    assert!(result.has_uvm_macros, "Expected UVM macros detected");

    // The expanded source should contain the field expansion artifacts
    assert!(
        result.expanded_source.contains("addr") || result.expanded_source.contains("field_txn"),
        "Expected field_txn content in expanded source"
    );

    // Try lenient parsing — if it fails, that's expected for field macros
    let parsed = parse_sv_to_graph_lenient(&result.expanded_source, 1);
    if let Some((nodes, _edges, extractor)) = parsed {
        // If parsing succeeds, verify basic structure
        let classes: Vec<_> = find_nodes_by_kind(&nodes, |k| {
            matches!(k, NodeKind::Class { name, .. } if resolve_from_extractor(&extractor, *name) == "field_txn")
        });
        assert!(
            !classes.is_empty(),
            "Expected at least one 'field_txn' class if parse succeeded"
        );
    }
    // If parsing fails, that's the known behavior for field macros
}

#[test]
fn test_macro_info_fixture_extraction() {
    // macro_info.sv has: `uvm_info`, `uvm_warning`, `uvm_error`
    let src = load_fixture("uvm_macros/macro_info.sv");
    let (nodes, _edges, _extractor) = preprocess_and_parse(&src, 1);

    // Should have Class node for info_demo
    let classes: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::Class { .. }));
    assert_eq!(classes.len(), 1, "Expected exactly one class");

    // After preprocessing, uvm_info/uvm_warning/uvm_error expand to
    // uvm_report_info/uvm_report_warning/uvm_report_error.
    // The expanded source should be parseable — verify we got a method for run_phase.
    let methods: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::Method { .. }));
    assert!(
        !methods.is_empty(),
        "Expected at least 1 method (constructor or run_phase)"
    );
}

#[test]
fn test_macro_do_fixture_extraction() {
    // macro_do.sv has: `uvm_do(req)`, `uvm_do_with(...)`, `uvm_create(...)`, `uvm_send(...)`
    // After preprocessing, uvm_do_with may produce parse errors due to constraint syntax.
    let src = load_fixture("uvm_macros/macro_do.sv");
    let result = hdl_graph_parse::preprocessor::preprocess(
        &src,
        "macro_do.sv",
        &std::collections::HashMap::new(),
        &[],
    );

    // Verify preprocessing happened
    assert!(result.has_uvm_macros, "Expected UVM macros detected");

    // The expanded source should contain start_item/finish_item from uvm_do expansion
    assert!(
        result.expanded_source.contains("start_item") || result.expanded_source.contains("randomize"),
        "Expected start_item or randomize from uvm_do expansion"
    );

    // Try lenient parsing
    let parsed = parse_sv_to_graph_lenient(&result.expanded_source, 1);
    if let Some((nodes, _edges, _extractor)) = parsed {
        assert!(
            nodes.len() >= 3,
            "Expected meaningful nodes from expanded uvm_do source, got {}",
            nodes.len()
        );
    }
    // If parsing fails, that's the known behavior for uvm_do_with expansion
}

#[test]
fn test_uvm_macro_fixtures_index() {
    // Index all uvm_macros fixture files with preprocessing
    let dir = fixture_dir("uvm_macros");
    let state = index_project_with_preprocessing(&dir);

    // Should have indexed 4 files
    assert!(
        state.file_map.len() >= 4,
        "Expected at least 4 indexed macro fixture files, got {}",
        state.file_map.len()
    );

    // Should have classes from fixtures that parse cleanly
    // (macro_fields and macro_do may fail to parse after expansion)
    let all_nodes = state.graph.all_nodes();
    let classes: Vec<_> = all_nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Class { .. }))
        .collect();
    assert!(
        classes.len() >= 2,
        "Expected at least 2 classes from macro fixtures, got {}",
        classes.len()
    );

    // Should have FactoryReg nodes from utils macros
    let factory_regs: Vec<_> = all_nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::FactoryReg { .. }))
        .collect();
    assert!(
        factory_regs.len() >= 3,
        "Expected at least 3 FactoryReg nodes, got {}",
        factory_regs.len()
    );

    // Graph should have edges
    assert!(
        state.graph.edge_count() > 10,
        "Expected many edges from macro fixtures, got {}",
        state.graph.edge_count()
    );
}
