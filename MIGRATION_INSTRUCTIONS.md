# HDL-Graph 离线迁移指南

## 概述

本指南帮助你将 hdl-graph 工具迁移到 Rocky Linux 8.9 离线机器。

## 准备好的离线包

离线包 `hdl-graph-offline.tar.gz` 已准备好，包含：

- ✅ Rust 工具链安装脚本 (`rust-toolchain-offline/rustup-init`)
- ✅ Cargo 依赖 (`vendor/` 目录)
- ✅ 源码和配置文件
- ✅ 离线安装脚本 (`install-rust-offline.sh`)

## 迁移步骤

### 步骤 1: 传输离线包到 Rocky Linux

```bash
# 在 Windows 上使用 scp/rsync 传输
scp C:\Users\lixiaoxin1\Downloads\AI\hdl-graph-offline.tar.gz user@rocky-linux:/tmp/
```

### 步骤 2: 在 Rocky Linux 上解压

```bash
cd /tmp
tar xzf hdl-graph-offline.tar.gz
cd hdl-codegraph
```

### 步骤 3: 安装系统依赖

```bash
# 如果有 dnf 源
sudo dnf install -y gcc gcc-c++ cmake make

# 如果完全离线，需要手动安装 RPM 包
# 从 Rocky Linux ISO 或 vault 镜像获取以下 RPM:
# - gcc
# - gcc-c++
# - cmake
# - make
# - glibc-devel
```

### 步骤 4: 安装 Rust 工具链

```bash
# 运行离线安装脚本
bash install-rust-offline.sh

# 或者手动安装
chmod +x rust-toolchain-offline/rustup-init
./rust-toolchain-offline/rustup-init -y --default-toolchain stable --no-modify-path
source ~/.cargo/env
```

### 步骤 5: 编译 hdl-graph

```bash
# 编译 release 版本
cargo build --release

# 或者使用 Makefile
make release
```

### 步骤 6: 安装到系统路径

```bash
sudo cp target/release/hdl-graph /usr/local/bin/
sudo chmod +x /usr/local/bin/hdl-graph
```

### 步骤 7: 验证安装

```bash
# 检查版本
hdl-graph --version

# 初始化测试项目
mkdir /tmp/test-hdl && cd /tmp/test-hdl
hdl-graph init .

# 创建简单 SV 文件
cat > test.sv << 'EOF'
module test;
  wire clk;
  wire rst_n;
endmodule
EOF

# 索引文件
hdl-graph index

# 查询模块层次
hdl-graph query hierarchy top
```

## 故障排查

### 问题: 编译失败，提示找不到 cmake

```bash
# 安装 cmake
sudo dnf install -y cmake

# 或者从源码编译 cmake
# 下载 cmake 源码: https://cmake.org/download/
```

### 问题: 编译失败，提示找不到 gcc/g++

```bash
# 安装 gcc/g++
sudo dnf install -y gcc gcc-c++

# 或者从 Rocky Linux ISO 获取 RPM 包
```

### 问题: Rust 工具链安装失败

```bash
# 检查 rustup-init 是否可执行
ls -la rust-toolchain-offline/rustup-init

# 手动安装
chmod +x rust-toolchain-offline/rustup-init
./rust-toolchain-offline/rustup-init -y --default-toolchain stable --no-modify-path
source ~/.cargo/env
```

### 问题: cargo build 失败，提示网络错误

```bash
# 检查 .cargo/config.toml 是否正确配置
cat .cargo/config.toml

# 应该包含:
# [source.crates-io]
# replace-with = "vendored-sources"
#
# [source.vendored-sources]
# directory = "vendor"
```

## 文件说明

| 文件/目录 | 说明 |
|-----------|------|
| `rust-toolchain-offline/rustup-init` | Rust 安装程序 |
| `vendor/` | Cargo 依赖离线包 |
| `.cargo/config.toml` | Cargo 配置，使用本地 vendor |
| `install-rust-offline.sh` | Rust 离线安装脚本 |
| `scripts/build-offline.sh` | 离线编译脚本 |
| `Makefile` | 构建脚本 |

## 参考文档

- `README.md` — 项目概述
- `PORTING.md` — 移植指南
- `docs/MIGRATION_PLAN.md` — 迁移计划
- `docs/user-guide.md` — 用户指南
- `docs/developer-guide.md` — 开发者指南
