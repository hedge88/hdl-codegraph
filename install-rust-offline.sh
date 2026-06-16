#!/bin/bash
# install-rust-offline.sh
# 在离线 Rocky Linux 服务器上安装 Rust 工具链

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd)"

echo "=== Rust 离线安装脚本 ==="
echo ""

# 设置 Rust 目录
export RUSTUP_HOME="$HOME/.rustup"
export CARGO_HOME="$HOME/.cargo"

mkdir -p "$RUSTUP_HOME"
mkdir -p "$CARGO_HOME"

# 检查 rustup-init 是否存在
if [ ! -f "$SCRIPT_DIR/rust-toolchain-offline/rustup-init" ]; then
    echo "错误: rustup-init 不存在"
    echo "请确认 rust-toolchain-offline/rustup-init 文件存在"
    exit 1
fi

# 安装 Rust
echo "[1/3] 安装 Rust 工具链..."
chmod +x "$SCRIPT_DIR/rust-toolchain-offline/rustup-init"
"$SCRIPT_DIR/rust-toolchain-offline/rustup-init" -y --default-toolchain stable --no-modify-path

# 创建 cargo 链接
echo "[2/3] 配置 PATH..."
if [ ! -f "$HOME/.cargo/env" ]; then
    cat > "$HOME/.cargo/env" << 'ENV'
. "$HOME/.cargo/env"
ENV
fi

# 添加到 .bashrc
if ! grep -q "\.cargo/env" "$HOME/.bashrc" 2>/dev/null; then
    echo '. "$HOME/.cargo/env"' >> "$HOME/.bashrc"
fi

source "$HOME/.cargo/env" 2>/dev/null || true

echo "[3/3] 验证安装..."
echo ""

# 验证安装
if command -v rustc &> /dev/null; then
    echo "=== 安装完成 ==="
    echo ""
    rustc --version
    cargo --version
    echo ""
    echo "请运行: source ~/.cargo/env"
else
    echo "错误: Rust 安装失败"
    exit 1
fi
