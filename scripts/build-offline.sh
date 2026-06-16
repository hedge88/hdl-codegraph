#!/bin/bash
# build-offline.sh
# 在 Rocky Linux 离线服务器上编译 hdl-graph
# 用法: bash scripts/build-offline.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "=== HDL-Graph 离线编译脚本 ==="
echo ""

# 检查 Rust 工具链
if ! command -v cargo &> /dev/null; then
    echo "错误: 未找到 cargo"
    echo ""
    echo "请先安装 Rust (以下任一方式):"
    echo "  方式1 (有网络): curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  方式2 (离线):   使用预下载的 rustup-init 安装"
    echo ""
    echo "离线安装步骤:"
    echo "  1. 在有网机器下载: https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"
    echo "  2. 传输到服务器"
    echo "  3. chmod +x rustup-init && ./rustup-init -y --default-toolchain stable"
    echo "  4. source \$HOME/.cargo/env"
    exit 1
fi

# 检查 C 编译器
if ! command -v gcc &> /dev/null; then
    echo "错误: 未找到 gcc"
    echo ""
    echo "请安装: sudo dnf install -y gcc gcc-c++ cmake make"
    echo "或从 Rocky Linux ISO 获取离线 RPM 包"
    exit 1
fi

# 检查 cmake
if ! command -v cmake &> /dev/null; then
    echo "错误: 未找到 cmake"
    echo ""
    echo "请安装: sudo dnf install -y cmake"
    exit 1
fi

cd "$PROJECT_ROOT"

# 验证 vendor 目录存在
if [ ! -d "vendor" ]; then
    echo "错误: vendor/ 目录不存在"
    echo "请确认已使用 prepare-offline.ps1 准备离线包"
    exit 1
fi

# 验证 .cargo/config.toml 存在
if [ ! -f ".cargo/config.toml" ]; then
    echo "错误: .cargo/config.toml 不存在"
    echo "请确认已使用 prepare-offline.ps1 准备离线包"
    exit 1
fi

echo "[1/2] 编译 release 版本..."
echo "(首次编译可能需要 5-15 分钟)"
echo ""

cargo build --release

if [ $? -ne 0 ]; then
    echo ""
    echo "错误: 编译失败"
    echo ""
    echo "常见问题排查:"
    echo "  1. 确认 gcc/g++/cmake 已安装: gcc --version && g++ --version && cmake --version"
    echo "  2. 确认 Rust 版本: rustc --version (需要 1.75+)"
    echo "  3. 磁盘空间: df -h (需要 ~2GB)"
    echo "  4. 内存: free -h (需要 ~2GB RAM)"
    exit 1
fi

echo ""
echo "[2/2] 安装到 /usr/local/bin..."
echo ""

# 复制二进制
sudo cp target/release/hdl-graph /usr/local/bin/
sudo chmod +x /usr/local/bin/hdl-graph

# 验证
echo "=== 安装完成 ==="
echo ""
hdl-graph --version
echo ""
echo "用法:"
echo "  hdl-graph init .          # 初始化项目"
echo "  hdl-graph index           # 构建代码图"
echo "  hdl-graph query hierarchy top  # 查看模块层次"
echo "  hdl-graph watch           # 启动 LSP 服务器"
