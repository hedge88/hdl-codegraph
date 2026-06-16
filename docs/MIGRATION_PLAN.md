# HDL-Graph 迁移到离线 Rocky Linux 服务器

## 项目概况

- **语言**: Rust (stable toolchain)
- **构建系统**: Cargo workspace (10 crates)
- **Native 依赖**:
  - tree-sitter-systemverilog (C grammar, parser.c + scanner.c)
  - RocksDB (via `rocksdb` crate, needs cmake + gcc)
  - 压缩库: lz4, zstd, bzip2, zlib
- **最终产物**: `hdl-graph` CLI binary

## 迁移策略

### 方案: 离线源码包 + 交叉编译

在有网络的 Windows 机器上准备完整的离线源码包，然后在 Rocky Linux 上编译。

## 步骤

### 1. 准备离线 Cargo 注册表

在 Windows 上运行:

```powershell
cd C:\Users\lixiaoxin1\Downloads\AI\hdl-codegraph

# 下载所有依赖到本地缓存
cargo fetch --target x86_64-unknown-linux-gnu

# 导出离线注册表
cargo vendor vendor/
```

### 2. 修改 Cargo.toml 使用本地 vendor

`cargo vendor` 会输出配置提示，需要在 `.cargo/config.toml` 中添加:

```toml
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
```

### 3. 打包源码

```powershell
# 创建迁移包 (排除 target/ 和 .git/)
tar czf hdl-graph-offline.tar.gz \
  --exclude='target' \
  --exclude='.git' \
  --exclude='._*' \
  -C .. hdl-codegraph
```

### 4. Rocky Linux 服务器准备

```bash
# 安装编译工具链 (需要 yum/dnf 源或离线 RPM)
sudo dnf install -y gcc gcc-c++ cmake make

# 安装 Rust (离线安装)
# 方案A: 从 rustup 下载离线安装包
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

# 方案B: 离线安装 (如果有预下载的 rustup-init)
chmod +x rustup-init
./rustup-init -y --default-toolchain stable
source $HOME/.cargo/env
```

### 5. 在 Rocky Linux 上编译

```bash
cd hdl-codegraph
cargo build --release
```

产物在 `target/release/hdl-graph`。

### 6. 安装到系统路径

```bash
sudo cp target/release/hdl-graph /usr/local/bin/
hdl-graph --version
```

## 离线 Rocky Linux 特殊处理

如果 Rocky Linux 完全离线 (无 dnf 源), 需要预先准备:

### Rust 工具链离线安装

1. 在有网机器下载 rustup-init: https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init
2. 下载 toolchain: `rustup toolchain download stable-x86_64-unknown-linux-gnu`
3. 打包 `~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/`

### 系统依赖 RPM 包

需要的 RPM (Rocky Linux 8/9):
- `gcc`
- `gcc-c++`
- `cmake`
- `make`
- `glibc-devel`
- `kernel-headers` (如果需要)

可以从 CentOS/Rocky 的 ISO 或 vault 镜像获取。

## 快速迁移脚本

见 `scripts/migrate-offline.sh`。
