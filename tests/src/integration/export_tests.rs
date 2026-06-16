use crate::integration::common;

use common::*;
use hdl_graph_core::*;
use hdl_graph_query::{JsonExporter, MarkdownExporter, MarkdownMode};

fn build_export_state() -> (hdl_graph_storage::InMemoryGraph, SymbolTable, std::collections::HashMap<String, u64>) {
    let dir = fixture_dir("verilog_basic");
    let state = index_project(&dir);
    (state.graph, state.symbols, state.file_map)
}

#[test]
fn test_json_export_roundtrip() {
    let (graph, symbols, file_map) = build_export_state();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().with_extension("json");

    JsonExporter::export(&graph, &symbols, &file_map, &path)
        .expect("JSON export failed");

    // Read and parse the exported JSON
    let content = std::fs::read_to_string(&path).expect("Failed to read JSON export");
    let json: serde_json::Value =
        serde_json::from_str(&content).expect("Failed to parse JSON export");

    // Verify structure
    assert!(json.get("metadata").is_some(), "Expected 'metadata' field");
    assert!(json.get("nodes").is_some(), "Expected 'nodes' field");
    assert!(json.get("edges").is_some(), "Expected 'edges' field");
    assert!(json.get("files").is_some(), "Expected 'files' field");

    // Verify counts
    let nodes = json["nodes"].as_array().expect("nodes should be array");
    let edges = json["edges"].as_array().expect("edges should be array");
    let files = json["files"].as_array().expect("files should be array");

    assert!(
        nodes.len() > 5,
        "Expected many nodes in JSON export, got {}",
        nodes.len()
    );
    assert!(!edges.is_empty(), "Expected edges in JSON export");
    assert!(
        files.len() >= 4,
        "Expected at least 4 files in JSON export, got {}",
        files.len()
    );

    // Verify node structure
    let first_node = &nodes[0];
    assert!(first_node.get("id").is_some(), "Node should have 'id' field");
    assert!(first_node.get("kind").is_some(), "Node should have 'kind' field");

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_json_export_schema() {
    let (graph, symbols, file_map) = build_export_state();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().with_extension("json");

    JsonExporter::export(&graph, &symbols, &file_map, &path)
        .expect("JSON export failed");

    let content = std::fs::read_to_string(&path).expect("Failed to read JSON");
    let json: serde_json::Value = serde_json::from_str(&content).expect("Failed to parse JSON");

    // Verify metadata
    let meta = &json["metadata"];
    assert!(meta.get("tool").is_some(), "Metadata should have 'tool' field");
    assert!(meta.get("version").is_some(), "Metadata should have 'version' field");

    // Verify edge structure
    let edges = json["edges"].as_array().unwrap();
    if !edges.is_empty() {
        let first_edge = &edges[0];
        assert!(first_edge.get("source").is_some(), "Edge should have 'source' field");
        assert!(first_edge.get("target").is_some(), "Edge should have 'target' field");
        assert!(first_edge.get("type").is_some(), "Edge should have 'type' field");
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_markdown_export_single() {
    let (graph, symbols, file_map) = build_export_state();

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().with_extension("md");

    MarkdownExporter::export(&graph, &symbols, &file_map, &path, MarkdownMode::Single)
        .expect("Markdown export failed");

    let content = std::fs::read_to_string(&path).expect("Failed to read Markdown");

    // Should contain key sections
    assert!(
        content.contains("Module") || content.contains("module") || content.contains("#"),
        "Expected module section in Markdown export"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_markdown_export_per_module() {
    let (graph, symbols, file_map) = build_export_state();

    let tmp_dir = tempfile::tempdir().unwrap();
    let out_dir = tmp_dir.path().join("docs");

    MarkdownExporter::export(&graph, &symbols, &file_map, &out_dir, MarkdownMode::PerModule)
        .expect("Per-module Markdown export failed");

    // Should create directory with index.md
    let index_path = out_dir.join("index.md");
    assert!(index_path.exists(), "Expected index.md in per-module export");

    let index_content = std::fs::read_to_string(&index_path).expect("Failed to read index.md");
    assert!(!index_content.is_empty(), "index.md should not be empty");
}
