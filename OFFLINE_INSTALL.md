# HDL-Graph 离线安装指南

## 概述

本离线安装包包含在 Rocky Linux 8.9 / CentOS 7+ 离线服务器上编译和安装 `hdl-graph` 所需的全部文件。

### 包内容

| 目录/文件 | 说明 | 大小 |
|-----------|------|------|
| `crates/` | 源代码 (11 个 Rust crate) | ~5 MB |
| `vendor/` | 已下载的 Cargo 依赖 (~350 个 crate) | ~350 MB |
| `rmcp-sdk/` | MCP SDK 源码 (rmcp + rmcp-macros) | ~3 MB |
| `.cargo/config.toml` | Cargo 离线配置 | <1 KB |
| `rust-toolchain-offline/` | Rust 离线安装器 (rustup-init) | ~20 MB |
| `Cargo.lock` | 依赖锁文件 | ~60 KB |
| `tests/` | 测试用例和 fixture | ~50 MB |
| `docs/` | 文档 | ~100 KB |
| `install.sh` | 一键安装脚本 | ~5 KB |
| `scripts/` | 辅助脚本 | ~10 KB |

### 系统要求

| 项目 | 最低要求 |
|------|---------|
| 操作系统 | Rocky Linux 8.x / CentOS 7+ / RHEL 7+ |
| 架构 | x86_64 |
| 磁盘空间 | 3 GB (编译时) |
| 内存 | 2 GB RAM |
| 系统依赖 | gcc, g++, cmake, make |

---

## 快速安装 (推荐)

### 1. 传输离线包到服务器

```bash
# 从 Windows 传输
scp hdl-graph-offline-v0.1.1-*.tar.gz user@server:/tmp/

# 或使用 rsync
rsync -avP hdl-graph-offline-v0.1.1-*.tar.gz user@server:/tmp/
```

### 2. 解压并安装

```bash
cd /tmp
tar xzf hdl-graph-offline-v0.1.1-*.tar.gz
cd hdl-codegraph
bash install.sh
```

安装脚本会自动：
1. ✅ 检查系统依赖 (gcc, g++, cmake)
2. ✅ 安装 Rust 工具链 (离线)
3. ✅ 配置 Cargo 使用本地 vendor
4. ✅ 编译 release 版本
5. ✅ 安装到 `/usr/local/bin`

### 3. 验证

```bash
hdl-graph --version
```

---

## 分步安装

如果一键安装失败，可以分步执行：

### Step 1: 安装系统依赖

```bash
# 如果有 dnf/yum 源
sudo dnf install -y gcc gcc-c++ cmake make

# 如果完全离线，需要从 Rocky Linux ISO 获取以下 RPM:
# - gcc, gcc-c++, cmake, make, glibc-devel, kernel-headers
```

### Step 2: 安装 Rust

```bash
chmod +x rust-toolchain-offline/rustup-init
./rust-toolchain-offline/rustup-init -y --default-toolchain stable --no-modify-path
source ~/.cargo/env
```

### Step 3: 配置 Cargo 离线源

确认 `.cargo/config.toml` 存在且内容正确：

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"

[net]
offline = true
```

### Step 4: 编译

```bash
cargo build --release -p hdl-graph-cli
```

### Step 5: 安装

```bash
sudo cp target/release/hdl-graph /usr/local/bin/
sudo chmod +x /usr/local/bin/hdl-graph
```

---

## 自定义安装路径

```bash
bash install.sh --prefix=/opt/tools
# 二进制安装到 /opt/tools/bin/hdl-graph
```

---

## VS Code 扩展

离线包中包含 VS Code 扩展源码 (`vscode-extension/`)。

### 在线安装 (推荐)

```bash
# 在有网络的机器上打包
cd vscode-extension
npm install
npm run package
# 生成 hdl-graph-0.1.0.vsix
```

然后在 VS Code 中：`Extensions` → `...` → `Install from VSIX...`

### 配置二进制路径

在 VS Code 设置中指定 hdl-graph 二进制路径：

```json
{
  "hdl-graph.binaryPath": "/usr/local/bin/hdl-graph"
}
```

---

## 故障排查

### 编译失败: 找不到 cmake

```bash
# 安装 cmake
sudo dnf install -y cmake

# 或设置 CMAKE 环境变量
export CMAKE=/path/to/cmake
```

### 编译失败: 找不到 libclang

```bash
# 安装 clang-devel
sudo dnf install -y clang-devel llvm-devel

# 或设置 LIBCLANG_PATH
export LIBCLANG_PATH=/usr/lib64/llvm/lib
```

### 编译失败: Rust 版本过旧

```bash
# 检查版本
rustc --version  # 需要 1.75+

# 重新安装
rustup default stable
```

### 编译失败: 磁盘空间不足

```bash
# 检查空间
df -h  # 需要 ~2GB

# 清理后重试
cargo clean
cargo build --release -p hdl-graph-cli
```

### cargo build 提示网络错误

```bash
# 检查 .cargo/config.toml 是否正确
cat .cargo/config.toml
# 应包含: replace-with = "vendored-sources"
#         directory = "vendor"
```

### 链接错误: 找不到 -lstdc++ 或 -lm

```bash
# 安装 C++ 标准库
sudo dnf install -y gcc-c++ libstdc++-devel
```

---

## 架构说明

```
hdl-graph
├── hdl-graph-cli     ← 主二进制 (入口)
├── hdl-graph-core    ← 核心数据结构
├── hdl-graph-grammar ← tree-sitter SystemVerilog 语法
├── hdl-graph-parse   ← CST 遍历和节点提取
├── hdl-graph-storage ← RocksDB 持久化存储
├── hdl-graph-query   ← 查询引擎和 SCIP 导出
├── hdl-graph-lsp     ← LSP 服务器
├── hdl-graph-mcp     ← MCP 服务器 (AI 工具集成)
└── hdl-graph-build   ← 构建辅助 (stub)
```

### 外部依赖 (已 vendor)

| 依赖 | 用途 |
|------|------|
| `tree-sitter` | 增量解析器 |
| `rocksdb` | 持久化存储 |
| `clap` | CLI 参数解析 |
| `tokio` | 异步运行时 |
| `tower-lsp` | LSP 协议 |
| `serde` / `serde_json` | 序列化 |
| `rmcp` | MCP 协议 |

---

## 更多文档

- `README.md` — 项目概述和功能列表
- `docs/user-guide.md` — 详细用户指南
- `docs/developer-guide.md` — 开发者指南
- `PORTING.md` — 跨平台移植指南
- `MIGRATION_INSTRUCTIONS.md` — 迁移指南
