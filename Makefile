.PHONY: all build test clean fetch-grammar init index watch help

all: build

help:
	@echo "HDL Code Graph — Make targets:"
	@echo ""
	@echo "  build            Build all crates (debug)"
	@echo "  release          Build all crates (release)"
	@echo "  test             Run all tests"
	@echo "  clean            Clean build artifacts"
	@echo "  fetch-grammar    Download tree-sitter-systemverilog grammar from npm"
	@echo ""
	@echo "  init  DIR=dir    Initialize a new .hdl-graph project in DIR"
	@echo "  index DIR=dir    Parse HDL sources and build the code graph"
	@echo "  watch DIR=dir    Watch DIR for changes and incrementally re-index"
	@echo "  stats DIR=dir    Display graph statistics for project in DIR"
	@echo ""
	@echo "  release-macos-x64    Cross-compile for macOS x86_64"
	@echo "  release-macos-arm64  Cross-compile for macOS ARM64"
	@echo "  release-linux        Cross-compile for Linux x86_64"

build:
	cargo build --workspace

test:
	cargo test --workspace

clean:
	cargo clean

fetch-grammar:
	cd crates/hdl-graph-grammar && \
	  npm pack tree-sitter-systemverilog@0.3.1 2>/dev/null && \
	  tar -xzf tree-sitter-systemverilog-0.3.1.tgz && \
	  cp package/src/parser.c grammar/src/parser.c && \
	  cp package/src/tree_sitter/parser.h grammar/src/tree_sitter/parser.h && \
	  cp package/src/tree_sitter/alloc.h grammar/src/tree_sitter/ 2>/dev/null; \
	  cp package/src/tree_sitter/array.h grammar/src/tree_sitter/ 2>/dev/null; \
	  cp package/queries/highlights.scm grammar/queries/; \
	  cp package/queries/locals.scm grammar/queries/ 2>/dev/null; true && \
	  rm -rf package tree-sitter-systemverilog-0.3.1.tgz && \
	  echo "Grammar updated from npm"

init:
	cargo run -p hdl-graph-cli -- init $(DIR)

index:
	cargo run -p hdl-graph-cli -- index --project $(DIR)

watch:
	cargo run -p hdl-graph-cli -- index --watch --project $(DIR)

stats:
	cargo run -p hdl-graph-cli -- stats --project $(DIR)

release:
	cargo build --release --workspace

# Cross-compilation targets
release-macos-x64:
	cargo build --release --target x86_64-apple-darwin

release-macos-arm64:
	cargo build --release --target aarch64-apple-darwin

release-linux:
	cargo build --release --target x86_64-unknown-linux-gnu
