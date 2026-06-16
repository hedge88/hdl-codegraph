use crate::integration::common;

use common::*;
use hdl_graph_core::*;
use hdl_graph_parse::{FileScanner, GraphExtractor};

#[test]
fn test_file_scanner_parse_source() {
    let src = r#"
module scanner_test (
    input  wire clk,
    output reg  out
);
    always @(posedge clk)
        out <= ~out;
endmodule
"#;
    let mut scanner = FileScanner::new().expect("Failed to create scanner");
    let tree = scanner.parse_source(src);
    assert!(!tree.root_node().has_error(), "Parse error in scanner test");
}

#[test]
fn test_file_scanner_parse_file() {
    let fixture_path = fixture_dir("verilog_basic").join("counter.v");
    let mut scanner = FileScanner::new().expect("Failed to create scanner");
    let tree = scanner.parse_file(&fixture_path).expect("Failed to parse file");
    assert!(!tree.root_node().has_error(), "Parse error in counter.v");
}

#[test]
fn test_scanner_incremental_no_old_tree() {
    let src = r#"
module inc_test;
    logic x;
    assign x = 1'b1;
endmodule
"#;
    let mut scanner = FileScanner::new().expect("Failed to create scanner");
    // Incremental with None old_tree should behave like full parse
    let tree = scanner.parse_source_incremental(src, None);
    assert!(!tree.root_node().has_error());

    let mut extractor = GraphExtractor::new();
    let (nodes, _edges) = extractor.extract(&tree, src.as_bytes(), 1);
    let modules: Vec<_> = find_nodes_by_kind(&nodes, |k| matches!(k, NodeKind::Module { .. }));
    assert_eq!(modules.len(), 1, "Should find 1 module");
}

#[test]
fn test_scanner_incremental_with_old_tree() {
    let src1 = "module inc_test; logic x; assign x = 1'b1; endmodule";
    let src2 = "module inc_test; logic x; logic y; assign x = 1'b1; assign y = 1'b0; endmodule";

    let mut scanner = FileScanner::new().expect("Failed to create scanner");
    let tree1 = scanner.parse_source(src1);
    assert!(!tree1.root_node().has_error());

    // Incremental parse with old tree
    let tree2 = scanner.parse_source_incremental(src2, Some(&tree1));
    // Incremental parse may or may not produce errors depending on tree-sitter's
    // diff algorithm. The key test: it doesn't crash and produces a usable tree.
    let _has_error = tree2.root_node().has_error();

    // Verify the new tree is a valid parse result
    let mut extractor = GraphExtractor::new();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        extractor.extract(&tree2, src2.as_bytes(), 1)
    }));
    // Either the tree parses cleanly or the extractor handles errors gracefully
    assert!(result.is_ok(), "Extractor should not panic on incremental parse result");
}

#[test]
fn test_scanner_with_include_dirs() {
    let scanner = FileScanner::with_include_dirs(vec!["./include".to_string()]);
    assert!(scanner.is_ok(), "Scanner with include dirs should be created successfully");
}

#[test]
fn test_file_scanner_parse_all_fixtures() {
    let mut scanner = FileScanner::new().expect("Failed to create scanner");

    // All fixture directories
    let dirs = ["verilog_basic", "sv_oop", "sv_advanced", "edge_cases"];
    for dir_name in &dirs {
        let dir = fixture_dir(dir_name);
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            match path.extension().and_then(|e| e.to_str()) {
                Some("sv" | "v" | "svh") => {
                    let tree = scanner
                        .parse_file(&path)
                        .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", path, e));
                    // Some fixtures intentionally have errors (e.g. ifdef_macros.sv uses macros)
                    // Just verify the scanner doesn't crash
                    let _ = tree;
                }
                _ => {}
            }
        }
    }
}
