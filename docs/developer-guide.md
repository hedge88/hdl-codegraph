# HDL Code Graph — Developer Guide

## Workspace Structure

```
hdl-codegraph/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── hdl-graph-core/                 # Core types (NodeKind, Edge, SymbolTable)
│   ├── hdl-graph-grammar/              # gmlarumbe tree-sitter grammar (61MB parser.c)
│   ├── hdl-graph-parse/                # CST traversal + UVM preprocessor
│   │   ├── scanner.rs                  # File handling
│   │   ├── extractor/mod.rs            # Main CST→graph dispatcher
│   │   ├── extractor/class.rs          # Class/inheritance extraction
│   │   ├── extractor/package.rs        # Package/import extraction
│   │   ├── extractor/interface.rs      # Interface/modport extraction
│   │   ├── extractor/generate.rs       # Generate block extraction
│   │   ├── extractor/assertion.rs      # SVA coverage extraction
│   │   ├── extractor/dpi.rs            # DPI-C/bind/config extraction
│   │   ├── extractor/incremental.rs    # Change set computation
│   │   ├── extractor/uvm_factory.rs    # Factory tracking
│   │   ├── extractor/uvm_tlm.rs        # TLM connections
│   │   ├── extractor/uvm_config.rs     # Config DB tracking
│   │   └── preprocessor/               # UVM macro expansion
│   ├── hdl-graph-storage/             # InMemory + RocksDB
│   ├── hdl-graph-query/               # SCIP export
│   ├── hdl-graph-lsp/                 # LSP server
│   ├── hdl-graph-web/                  # Web API (future)
│   ├── hdl-graph-types/               # Protobuf schemas (future)
│   └── hdl-graph-cli/                 # CLI binary
├── vscode-extension/                   # VS Code extension
├── tests/                              # Integration test corpora
├── fuzz/                               # Fuzz targets
├── docs/                               # Documentation
└── ci/                                 # CI/CD scripts
```

## Adding a New Construct

1. **Find the grammar node type** — Check `grammar.json` or `node-types.json` in hdl-graph-grammar
2. **Add a NodeKind variant** — Add to `NodeKind` enum in `hdl-graph-core/src/node.rs`
3. **Add extraction logic** — Add a method in the appropriate module under `extractor/`
4. **Add match arm** — Wire it up in `extractor/mod.rs` in `extract()` or `extract_module()`
5. **Add test** — Add a test in `hdl-graph-parse/src/lib.rs`
6. **Verify** — `cargo test -p hdl-graph-parse`

Example — adding property support:
```rust
// 1. In node.rs:
NodeKind::PropertyDecl { name: InternedString }

// 2. In extractor/assertion.rs:
pub fn extract_property_declaration(&mut self, node: Node, source: &[u8], ...) {
    let name = node.child_by_field_name("name")
        .map(|n| self.text(n, source).to_string());
    // ... add node
}

// 3. In mod.rs match arms:
"property_declaration" => {
    self.extract_property_declaration(child, source, module_id, nodes, edges);
}

// 4. In lib.rs test:
#[test] fn test_property_decl() { ... }
```

## Building

```bash
cargo build                     # Debug build
cargo build --release           # Release build
cargo test --workspace          # All tests
make fetch-grammar              # Update tree-sitter grammar
```

## LSP Protocol Support

| Method | Status |
|--------|--------|
| initialize | ✅ |
| initialized | ✅ |
| shutdown | ✅ |
| textDocument/definition | ✅ |
| textDocument/references | ✅ (basic) |
| textDocument/hover | ✅ (basic) |
| textDocument/semanticTokens | 🔄 In progress |
| textDocument/completion | 🔄 In progress |
| textDocument/documentSymbol | ⏳ Planned |
| workspace/symbol | ⏳ Planned |

## Performance Targets

| Operation | Target | Current |
|-----------|--------|---------|
| Parse single file | < 10ms | ✅ |
| Index 1000 files | < 60s | 🟢 |
| Definition lookup | < 5ms | ✅ |
| Find references | < 50ms | 🟡 |
| Incremental update | < 50ms | ✅ |
