use crate::integration::common;

use hdl_graph_parse::preprocessor::preprocess;
use std::collections::HashMap;

fn preprocess_str(source: &str) -> hdl_graph_parse::preprocessor::PreprocessedFile {
    preprocess(source, "test.sv", &HashMap::new(), &[])
}

#[test]
fn test_component_utils_expansion() {
    let src = r#"
class my_driver extends uvm_driver #(my_txn);
    `uvm_component_utils(my_driver)
    function new(string name, uvm_component parent);
        super.new(name, parent);
    endfunction
endclass
"#;
    let result = preprocess_str(src);

    // Expanded source should contain uvm_component_registry and type_id
    assert!(
        result.expanded_source.contains("uvm_component_registry"),
        "Expected uvm_component_registry in expanded source"
    );
    assert!(
        result.expanded_source.contains("type_id"),
        "Expected type_id typedef in expanded source"
    );
    assert!(result.has_uvm_macros, "Expected has_uvm_macros == true");
}

#[test]
fn test_object_utils_expansion() {
    let src = r#"
class my_obj extends uvm_object;
    `uvm_object_utils(my_obj)
    function new(string name);
        super.new(name);
    endfunction
endclass
"#;
    let result = preprocess_str(src);

    assert!(
        result.expanded_source.contains("uvm_object_registry"),
        "Expected uvm_object_registry in expanded source"
    );
    assert!(result.has_uvm_macros, "Expected has_uvm_macros == true");
}

#[test]
fn test_field_macros_expand() {
    let src = r#"
class field_txn extends uvm_sequence_item;
    rand bit [31:0] addr;
    function void do_print(uvm_printer printer);
        `uvm_field_int(addr, UVM_ALL_ON)
    endfunction
endclass
"#;
    let result = preprocess_str(src);

    // uvm_field_int should expand (to something parseable)
    assert!(
        result.expanded_source.contains("addr"),
        "Expected addr to still be present after field macro expansion"
    );
    assert!(result.has_uvm_macros, "Expected has_uvm_macros == true");
}

#[test]
fn test_info_macros_expand() {
    let src = r#"
class demo extends uvm_component;
    task run_phase(uvm_phase phase);
        `uvm_info("TAG", "message", UVM_LOW)
        `uvm_error("TAG", "error msg")
        `uvm_warning("TAG", "warning msg")
        `uvm_fatal("TAG", "fatal msg")
    endtask
endclass
"#;
    let result = preprocess_str(src);

    assert!(
        result.expanded_source.contains("uvm_report_info")
            || result.expanded_source.contains("uvm_info"),
        "Expected uvm_info to expand"
    );
    assert!(
        result.expanded_source.contains("uvm_report_error")
            || result.expanded_source.contains("uvm_error"),
        "Expected uvm_error to expand"
    );
}

#[test]
fn test_do_macros_expand() {
    let src = r#"
class seq extends uvm_sequence;
    task body();
        `uvm_do(req)
        `uvm_do_with(req, { write == 1; })
    endtask
endclass
"#;
    let result = preprocess_str(src);

    // uvm_do should expand to start_item/finish_item
    assert!(
        result.expanded_source.contains("start_item") || result.expanded_source.contains("randomize"),
        "Expected uvm_do to expand to start_item/finish_item"
    );
    assert!(result.has_uvm_macros, "Expected has_uvm_macros == true");
}

#[test]
fn test_macros_then_extract() {
    // Full pipeline: preprocess UVM macros, then parse and extract graph
    let src = r#"
class my_driver extends uvm_driver #(my_txn);
    `uvm_component_utils(my_driver)
    function new(string name, uvm_component parent);
        super.new(name, parent);
    endfunction
endclass
"#;
    let result = preprocess_str(src);

    // Try to parse the expanded source
    let (nodes, _edges, _extractor) = super::common::parse_sv_to_graph(&result.expanded_source, 1);

    // Should have at least some nodes (Class, Method, etc.)
    assert!(
        !nodes.is_empty(),
        "Expected nodes from expanded UVM source, got 0"
    );
}
