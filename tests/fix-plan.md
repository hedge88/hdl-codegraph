# Bug Fix Plan — hdl-graph Extractor

Based on deep verification against darkriscv, ibex, axi, sv-tests.

## Bug 1: Port direction fallback to Inout (CRITICAL)
- File: `crates/hdl-graph-parse/src/extractor/mod.rs` line 321
- Root cause: Verilog `input wire data` parsed by tree-sitter as `port_declaration` (not `ansi_port_declaration`), the non-ANSI path checks for bare `"input"` child nodes, but tree-sitter may wrap them in `port_direction` or `net_type` nodes
- Fix: Add fallback logic to scan ALL children for direction keywords, not just exact child kind matches

## Bug 2: `always @(posedge)` classified as Combinational (HIGH)
- File: `crates/hdl-graph-parse/src/extractor/mod.rs` line 572
- Root cause: Only checks `always_keyword` child for "always_ff"/"always_latch". Verilog `always` blocks have the keyword as "always" but the sensitivity list contains `posedge`/`negedge` — these should be Sequential
- Fix: When `always_keyword` is "always", check sensitivity list for `posedge`/`negedge` to determine Sequential vs Combinational

## Bug 3: No Instantiates edges (HIGH)
- File: `crates/hdl-graph-parse/src/extractor/mod.rs` line 648
- Root cause: `extract_hierarchical_instance` creates `Contains` edge but no `Instantiates` edge from instance to module definition
- Fix: Add `Instantiates` edge creation (needs deferred resolution since target module may not be indexed yet)

## Bug 4: Package-embedded classes not extracted (MEDIUM)
- File: `crates/hdl-graph-parse/src/extractor/package.rs` line 63
- Root cause: `extract_package_item` only handles `package_import_declaration` and `package_export_declaration`, ignores `class_declaration`
- Fix: Add `class_declaration` handling in `extract_package_item`, delegate to `extract_class`

## Bug 5: Macro function-argument expansion (sv-tests 22 files)
- File: `crates/hdl-graph-parse/src/preprocessor/sv_preprocessor.rs` line 271
- Root cause: `expand_inline_macros` finds macro args `(x,y)` but doesn't substitute them into the value. Formal params like `(x,y)` in define values leak into output
- Fix: Parse formal params from `define`, parse actual args from invocation, substitute into value body
