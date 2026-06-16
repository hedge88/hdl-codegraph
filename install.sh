#!/bin/bash
# install.sh — HDL-Graph 一键离线安装脚本
# 用法: bash install.sh [--prefix=/usr/local] [--skip-deps] [--skip-rust]
#
# 此脚本在 Rocky Linux 8.9 / CentOS 7+ 离线服务器上执行：
#   1. 检查系统依赖 (gcc, g++, cmake)
#   2. 安装 Rust 工具链 (离线)
#   3. 编译 hdl-graph (release)
#   4. 安装到指定目录
#   5. 验证安装

set -e

# ========== 颜色输出 ==========
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

info()  { echo -e "${CYAN}$*${NC}"; }
ok()    { echo -e "${GREEN}$*${NC}"; }
warn()  { echo -e "${YELLOW}$*${NC}"; }
err()   { echo -e "${RED}$*${NC}"; }
gray()  { echo -e "${GRAY}$*${NC}"; }

# ========== 参数解析 ==========
PREFIX="/usr/local"
SKIP_DEPS=false
SKIP_RUST=false

for arg in "$@"; do
    case $arg in
        --prefix=*)   PREFIX="${arg#*=}" ;;
        --skip-deps)  SKIP_DEPS=true ;;
        --skip-rust)  SKIP_RUST=true ;;
        --help|-h)
            echo "用法: bash install.sh [选项]"
            echo ""
            echo "选项:"
            echo "  --prefix=DIR    安装目录 (默认: /usr/local/bin)"
            echo "  --skip-deps     跳过系统依赖检查"
            echo "  --skip-rust     跳过 Rust 安装 (假设已安装)"
            echo "  --help          显示此帮助"
            exit 0
            ;;
        *)
            err "未知参数: $arg"
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$PREFIX/bin"

echo ""
echo "============================================"
info "  HDL-Graph 离线安装"
echo "============================================"
echo ""
gray "项目目录: $SCRIPT_DIR"
gray "安装目录: $INSTALL_DIR"
gray "系统:     $(uname -s) $(uname -m)"
gray "内核:     $(uname -r)"
echo ""

# ========== Step 1: 检查系统依赖 ==========
echo -e "${YELLOW}[1/5] 检查系统依赖...${NC}"

check_cmd() {
    local cmd=$1
    local pkg=$2
    if command -v "$cmd" &>/dev/null; then
        gray "  ✓ $cmd: $(command -v "$cmd")"
        return 0
    else
        warn "  ✗ $cmd 未找到 (需要安装: $pkg)"
        return 1
    fi
}

MISSING_DEPS=false

if [ "$SKIP_DEPS" = false ]; then
    check_cmd gcc "gcc"        || MISSING_DEPS=true
    check_cmd g++ "gcc-c++"    || MISSING_DEPS=true
    check_cmd cmake "cmake"    || MISSING_DEPS=true
    check_cmd make "make"      || MISSING_DEPS=true

    if [ "$MISSING_DEPS" = true ]; then
        echo ""
        err "缺少系统依赖。请安装:"
        echo ""
        echo "  # Rocky Linux / CentOS:"
        echo "  sudo dnf install -y gcc gcc-c++ cmake make"
        echo ""
        echo "  # 或从 Rocky Linux ISO 获取离线 RPM 包"
        echo ""
        echo "  # 跳过检查: bash install.sh --skip-deps"
        exit 1
    fi
else
    warn "  跳过系统依赖检查"
fi

echo ""

# ========== Step 2: 安装 Rust 工具链 ==========
echo -e "${YELLOW}[2/5] 安装 Rust 工具链...${NC}"

install_rust_offline() {
    local rustup_bin="$SCRIPT_DIR/rust-toolchain-offline/rustup-init"

    if [ ! -f "$rustup_bin" ]; then
        err "  错误: rustup-init 不存在: $rustup_bin"
        echo ""
        echo "  请确保离线包中包含 rust-toolchain-offline/rustup-init"
        echo "  或使用 --skip-rust 跳过 (如果已安装 Rust)"
        exit 1
    fi

    gray "  使用离线 rustup-init 安装..."
    chmod +x "$rustup_bin"

    export RUSTUP_HOME="$HOME/.rustup"
    export CARGO_HOME="$HOME/.cargo"
    mkdir -p "$RUSTUP_HOME" "$CARGO_HOME"

    "$rustup_bin" -y --default-toolchain stable --no-modify-path 2>&1 | while IFS= read -r line; do
        gray "    $line"
    done

    # 配置 PATH
    if [ -f "$HOME/.cargo/env" ]; then
        source "$HOME/.cargo/env"
    fi

    # 添加到 .bashrc
    if ! grep -q "\.cargo/env" "$HOME/.bashrc" 2>/dev/null; then
        echo '. "$HOME/.cargo/env"' >> "$HOME/.bashrc"
    fi
}

if [ "$SKIP_RUST" = true ]; then
    warn "  跳过 Rust 安装 (--skip-rust)"
else
    if command -v cargo &>/dev/null; then
        gray "  Rust 已安装: $(rustc --version 2>/dev/null || echo 'unknown')"

        # 检查版本是否够新 (需要 1.75+)
        RUST_VERSION=$(rustc --version 2>/dev/null | grep -oP '\d+\.\d+' | head -1)
        RUST_MAJOR=$(echo "$RUST_VERSION" | cut -d. -f1)
        RUST_MINOR=$(echo "$RUST_VERSION" | cut -d. -f2)

        if [ "$RUST_MAJOR" -lt 1 ] || ([ "$RUST_MAJOR" -eq 1 ] && [ "$RUST_MINOR" -lt 75 ]); then
            warn "  Rust 版本 $RUST_VERSION 过旧，需要 1.75+"
            warn "  尝试使用离线安装器更新..."
            install_rust_offline
        fi
    else
        install_rust_offline
    fi
fi

# 验证 Rust
if ! command -v cargo &>/dev/null; then
    source "$HOME/.cargo/env" 2>/dev/null || true
fi

if command -v cargo &>/dev/null; then
    ok "  Rust: $(rustc --version 2>/dev/null)"
    gray "  Cargo: $(cargo --version 2>/dev/null)"
else
    err "  错误: Rust 安装失败"
    err "  请手动安装后重试: bash install.sh --skip-rust"
    exit 1
fi

echo ""

# ========== Step 3: 验证离线配置 ==========
echo -e "${YELLOW}[3/5] 验证离线配置...${NC}"

cd "$SCRIPT_DIR"

# 检查 vendor 目录
if [ ! -d "vendor" ]; then
    err "  错误: vendor/ 目录不存在"
    err "  离线包可能不完整，请重新生成"
    exit 1
fi
VENDOR_COUNT=$(ls -d vendor/*/ 2>/dev/null | wc -l)
gray "  vendor/: $VENDOR_COUNT 个 crate"

# 检查 .cargo/config.toml
if [ ! -f ".cargo/config.toml" ]; then
    warn "  .cargo/config.toml 不存在，创建中..."
    mkdir -p .cargo
    cat > .cargo/config.toml << 'CARGO_CONFIG'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
CARGO_CONFIG
    ok "  .cargo/config.toml 已创建"
else
    gray "  .cargo/config.toml 已存在"
    # 验证是否指向 vendor
    if ! grep -q "vendored-sources" .cargo/config.toml; then
        warn "  警告: .cargo/config.toml 未配置 vendor 源"
        warn "  请确认配置正确 (参见 MIGRATION_INSTRUCTIONS.md)"
    fi
fi

# 验证 Cargo.lock 存在
if [ ! -f "Cargo.lock" ]; then
    err "  错误: Cargo.lock 不存在"
    exit 1
fi
gray "  Cargo.lock: $(wc -l < Cargo.lock) 行"

echo ""

# ========== Step 4: 编译 ==========
echo -e "${YELLOW}[4/5] 编译 hdl-graph (release)...${NC}"
echo ""
gray "  首次编译可能需要 5-15 分钟，取决于机器性能"
gray "  需要约 2GB 磁盘空间和 2GB RAM"
echo ""

# 编译 release 版本
cd "$SCRIPT_DIR"
cargo build --release -p hdl-graph-cli 2>&1 | while IFS= read -r line; do
    # 只显示关键信息
    if echo "$line" | grep -qE "Compiling|Finished|error|warning.*unused"; then
        gray "  $line"
    fi
done

# 检查编译结果
BINARY="$SCRIPT_DIR/target/release/hdl-graph"
if [ ! -f "$BINARY" ]; then
    # Windows 子系统可能生成 .exe
    BINARY="$SCRIPT_DIR/target/release/hdl-graph.exe"
fi

if [ ! -f "$BINARY" ]; then
    err "  错误: 编译失败，未找到二进制文件"
    echo ""
    echo "  请检查:"
    echo "    1. gcc/g++/cmake 版本: gcc --version && g++ --version && cmake --version"
    echo "    2. Rust 版本: rustc --version (需要 1.75+)"
    echo "    3. 磁盘空间: df -h (需要 ~2GB)"
    echo "    4. 内存: free -h (需要 ~2GB RAM)"
    echo ""
    echo "  详细错误:"
    cargo build --release -p hdl-graph-cli 2>&1 | tail -20
    exit 1
fi

BINARY_SIZE=$(du -h "$BINARY" | cut -f1)
ok "  编译成功: $BINARY ($BINARY_SIZE)"

echo ""

# ========== Step 5: 安装 ==========
echo -e "${YELLOW}[5/5] 安装到 $INSTALL_DIR...${NC}"

# 创建安装目录
if [ ! -d "$INSTALL_DIR" ]; then
    if [ -w "$(dirname "$INSTALL_DIR")" ] || [ "$(id -u)" -eq 0 ]; then
        mkdir -p "$INSTALL_DIR"
    else
        sudo mkdir -p "$INSTALL_DIR"
    fi
fi

# 复制二进制
if [ -w "$INSTALL_DIR" ] || [ "$(id -u)" -eq 0 ]; then
    cp "$BINARY" "$INSTALL_DIR/hdl-graph"
    chmod +x "$INSTALL_DIR/hdl-graph"
else
    sudo cp "$BINARY" "$INSTALL_DIR/hdl-graph"
    sudo chmod +x "$INSTALL_DIR/hdl-graph"
fi

ok "  已安装: $INSTALL_DIR/hdl-graph"

# 验证安装
echo ""
echo "============================================"
ok "  安装完成!"
echo "============================================"
echo ""

# 运行版本检查
if "$INSTALL_DIR/hdl-graph" --version 2>/dev/null; then
    echo ""
fi

# 检查 PATH
if echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
    gray "  $INSTALL_DIR 已在 PATH 中"
else
    warn "  $INSTALL_DIR 不在 PATH 中"
    echo ""
    echo "  请添加到 PATH:"
    echo "    echo 'export PATH=\"$INSTALL_DIR:\$PATH\"' >> ~/.bashrc"
    echo "    source ~/.bashrc"
fi

echo ""
echo "快速开始:"
echo "  hdl-graph --help                    # 查看帮助"
echo "  hdl-graph init /path/to/project     # 初始化项目"
echo "  hdl-graph index --project /path     # 构建代码图"
echo "  hdl-graph query hierarchy top       # 查看模块层次"
echo "  hdl-graph watch --project /path     # 启动 LSP 服务器"
echo ""
echo "文档:"
gray "  docs/user-guide.md      — 用户指南"
gray "  docs/developer-guide.md — 开发者指南"
gray "  README.md               — 项目概述"
echo ""
