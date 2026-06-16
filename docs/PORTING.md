# Porting Guide: hdl-graph to Windows 11 & Rocky Linux 8.9

## Platform Requirements

### Common (both)
| Tool | Version | Purpose |
|------|---------|---------|
| Rust | ≥ 1.79 | Compiler |
| Rust target | native | `rustup target add <triple>` |
| C compiler | gcc/clang/msvc | Compile 61MB parser.c |
| C++ compiler | g++/clang++/msvc | Compile librocksdb-sys |
| CMake | ≥ 3.14 | Build librocksdb-sys |
| libclang | clang/LLVM | bindgen for rocksdb-sys |

### Rocky Linux 8.9
```
sudo dnf install -y gcc gcc-c++ cmake make clang-devel llvm-devel curl git
```

### Windows 11
```
# Visual Studio 2022 Build Tools (or VS2022 Community)
#   Workload: "Desktop development with C++"
#   Individual: Windows 10/11 SDK, MSVC v143

winget install Kitware.CMake Kitware.CMake 2>/dev/null  # or manually from cmake.org
winget install LLVM.LLVM                                # or manually from llvm.org
winget install Git.Git
```

## Build Methods

### Method 1: Native build (recommended)
```bash
git clone https://github.com/lixiaoxin/hdl-codegraph.git
cd hdl-codegraph
cargo build --release -p hdl-graph-cli
cargo install --path crates/hdl-graph-cli
```

On Windows, set this before building:
```powershell
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"
```

### Method 2: Docker (Rocky Linux)
```bash
# Build binary inside Rocky Linux 8.9 container
docker build -f Dockerfile.rocky -t hdl-graph-builder .
docker run --rm -v $(pwd)/dist:/dist hdl-graph-builder cp /usr/local/bin/hdl-graph /dist/
# Binary in ./dist/hdl-graph
```

### Method 3: Cross-compile from macOS to Linux
```bash
# Requires: musl-cross or zigbuild
cargo install cargo-zigbuild
rustup target add x86_64-unknown-linux-musl
RUSTFLAGS="-C target-feature=-crt-static" \
  cargo zigbuild --release -p hdl-graph-cli --target x86_64-unknown-linux-musl
```

For Windows cross-compile:
```bash
rustup target add x86_64-pc-windows-msvc
# Needs LLVM + MSVC (cross-compile from macOS needs mingw or msvc linker)
cargo build --release -p hdl-graph-cli --target x86_64-pc-windows-msvc
```

## Binary Output

| Platform | Binary Location | Size (approx) |
|----------|---------------|---------------|
| macOS | `target/release/hdl-graph` | 8-15 MB |
| Rocky Linux | `target/release/hdl-graph` | 15-25 MB |
| Windows | `target\release\hdl-graph.exe` | 8-15 MB |

Size depends on `RUSTFLAGS="-C lto=fat -C strip=symbols"` (release builds already have LTO).

## Runtime Dependencies (requirements to RUN)

The built binary is **statically linked** — no Rust runtime needed.
- `libc.so.6` / `msvcrt.dll` — OS standard
- No Python, Node.js, or JRE required
- Single binary deployment

## Distribution

Pre-built binaries for CI:
```yaml
# .github/workflows/release.yml already configured for:
#   - aarch64-apple-darwin
#   - x86_64-apple-darwin
#   - x86_64-unknown-linux-gnu
#   - aarch64-unknown-linux-gnu
# Add for Windows:
#   - x86_64-pc-windows-msvc
```
