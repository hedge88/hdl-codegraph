#!/bin/bash
# Release script for hdl-codegraph
# Packages platform-specific binaries + artifacts

set -euo pipefail

VERSION="${1:-0.1.0}"
echo "Building release v$VERSION"

# Build for all targets
TARGETS=(
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
    "x86_64-unknown-linux-gnu"
    "aarch64-unknown-linux-gnu"
)

for target in "${TARGETS[@]}"; do
    echo "Building for $target..."
    rustup target add "$target" 2>/dev/null || true
    RUSTFLAGS="-C lto=fat -C codegen-units=1 -C strip=symbols" \
        cargo build --release --target "$target" -p hdl-graph-cli
done

# Package
for target in "${TARGETS[@]}"; do
    echo "Packaging $target..."
    mkdir -p "dist/hdl-graph-$target"
    cp "target/$target/release/hdl-graph" "dist/hdl-graph-$target/"

    cd dist
    tar czf "hdl-graph-$target.tar.gz" "hdl-graph-$target"
    cd ..

    # Compute SHA256
    shasum -a 256 "dist/hdl-graph-$target.tar.gz" >> "dist/SHA256SUMS"
done

echo ""
echo "Release artifacts in dist/"
echo "SHA256 checksums:"
cat dist/SHA256SUMS
