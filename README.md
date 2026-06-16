# HDL Code Graph

> **v0.2.0** — Verilog / SystemVerilog / UVM 代码智能分析工具

将 HDL 源码（`.sv` / `.svh` / `.v` / `.vh`）解析为可查询的代码图，支持定义跳转、引用查找、模块层次分析、信号驱动追踪、UVM 工厂/TLM/config_db 分析、影响范围评估，以及 MCP 和 LSP 集成。

```
                  ┌──────────────┐
                  │ LSP / CLI    │
                  └──────┬───────┘
                         │
                ┌────────▼────────┐
                │  Query Engine   │
                │  (stack-graph)  │
                └────────┬────────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
    ┌─────▼─────┐ ┌────▼────┐ ┌───────▼──┐
    │  RocksDB  │ │ SQLite  │ │ Tantivy  │
    │ (graph)   │ │(meta)   │ │ (search) │
    └───────────┘ └─────────┘ └──────────┘
          │
    ┌─────▼─────────────────────────┐
    │  tree-sitter-systemverilog    │
    │  (816 grammar rules)          │
    └───────────────────────────────┘
```

---

## 目录

- [功能特性](#功能特性)
- [版本信息](#版本信息)
- [安装](#安装)
- [快速开始](#快速开始)
- [CLI 命令参考](#cli-命令参考)
- [MCP 服务器](#mcp-服务器)
- [LSP 服务器](#lsp-服务器)
- [架构](#架构)
- [开发](#开发)
- [文档](#文档)
- [许可证](#许可证)

---

## 功能特性

| 特性 | 说明 |
|------|------|
| **模块层次树** | 查看设计的完整实例化结构 |
| **定义跳转** | 定位任意标识符（模块、类、端口、信号、函数）的声明位置 |
| **引用查找** | 查找信号/模块/类在项目中的所有使用点 |
| **信号流追踪** | 追踪信号的驱动（写）和读取 |
| **调用图分析** | 查看函数/任务的调用者和被调用者 |
| **影响分析** | BFS 评估修改的影响范围（深度 3） |
| **UVM 支持** | 工厂注册/覆盖、TLM 端口连接、config_db 追踪、类继承层次 |
| **MCP 服务器** | 为 Claude、Cursor 等 AI 助手提供 15 个代码分析工具 |
| **LSP 服务器** | 集成 VS Code / Neovim / Emacs |
| **SCIP 导出** | 兼容 Sourcegraph 和 GitHub Code Search |
| **增量索引** | 文件变更时实时更新（100ms 防抖） |
| **图一致性检查** | 检测悬挂边、未解析实例、孤立节点 |
| **多格式导出** | SCIP / JSON / Markdown 三种格式 |

---

## 版本信息

| 组件 | 版本 | 说明 |
|------|------|------|
| `hdl-graph-cli` | 0.2.0 | CLI 主二进制 |
| `hdl-graph-mcp` | 0.2.0 | MCP 服务器 |
| `hdl-graph-core` | 0.2.0 | 核心数据结构 |
| `hdl-graph-grammar` | 0.2.0 | tree-sitter SystemVerilog 语法 |
| `hdl-graph-parse` | 0.2.0 | CST 遍历和节点提取 |
| `hdl-graph-storage` | 0.2.0 | RocksDB 持久化 + InMemoryGraph |
| `hdl-graph-query` | 0.2.0 | 查询引擎和导出器 |
| `hdl-graph-lsp` | 0.2.0 | LSP 语言服务器 |
| `hdl-graph-types` | 0.2.0 | 共享类型定义 |
| `hdl-graph-build` | 0.2.0 | 构建辅助 |
| `hdl-graph-web` | 0.2.0 | Web 接口 |
| VS Code 扩展 | 0.2.0 | 编辑器扩展 |
| `rmcp-sdk` | 1.7.0 | MCP 协议 SDK（内嵌） |

---

## 安装

### Homebrew (macOS)

```bash
brew install hdl-graph/tap/hdl-graph
```

### Cargo

```bash
cargo install hdl-graph-cli
```

### Debian / Ubuntu

```bash
sudo dpkg -i hdl-graph_0.2.0_amd64.deb
```

### VS Code 扩展

从 marketplace 安装 `hdl-graph.vsix`，或手动安装：

```bash
# 下载 .vsix 文件后
code --install-extension hdl-graph-0.2.0.vsix
```

### 离线安装（无网络环境）

适用于 Rocky Linux / CentOS / RHEL 等无网络环境的服务器：

```bash
# 1. 传输离线包
scp hdl-graph-offline-v0.2.0-*.tar.gz user@server:/tmp/

# 2. 解压并安装
ssh user@server
cd /tmp && tar xzf hdl-graph-offline-v0.2.0-*.tar.gz
cd hdl-codegraph && bash install.sh

# 3. 验证
hdl-graph --version
```

安装脚本自动完成：系统依赖检查 → Rust 工具链安装 → Cargo 离线配置 → 编译 → 安装到 `/usr/local/bin`。

自定义安装路径：`bash install.sh --prefix=/opt/tools`

详见 [OFFLINE_INSTALL.md](OFFLINE_INSTALL.md)。

### 从源码编译

```bash
git clone https://github.com/hedge88/hdl-codegraph.git
cd hdl-codegraph
cargo build --release
```

编译产物位于 `target/release/hdl-graph` 和 `target/release/hdl-graph-mcp`。

### 系统要求

| 项目 | 最低要求 |
|------|----------|
| 操作系统 | macOS / Linux (Rocky 8+, CentOS 7+, RHEL 7+) |
| 架构 | x86_64, aarch64 |
| 磁盘 | 3 GB（编译期间） |
| 内存 | 2 GB |
| 系统依赖 | gcc, g++, cmake, make, clang-devel |

---

## 快速开始

```bash
# 1. 初始化项目配置
hdl-graph init .

# 2. 索引 HDL 源码
hdl-graph index

# 3. 查询符号定义
hdl-graph query def my_fifo

# 4. 查找所有引用
hdl-graph query refs clk

# 5. 查看模块层次
hdl-graph query hierarchy top_module

# 6. 搜索符号
hdl-graph search apb_*

# 7. UVM 工厂分析
hdl-graph uvm factory my_driver

# 8. 追踪信号驱动
hdl-graph query drivers data_out

# 9. 评估修改影响
hdl-graph query impact config_reg
```

---

## CLI 命令参考

### 全局选项

以下选项适用于所有子命令：

| 选项 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `--project <dir>` | Path | `.` | HDL 项目根目录 |
| `--include-dirs <dirs>` | Vec | 空 | 额外扫描目录（可多次指定） |
| `--uvm-home <dir>` | Path | 无 | UVM 库目录路径 |
| `--defines <DEF>` | Vec | 空 | 预处理器宏定义（可多次指定） |
| `--jobs <N>` | usize | CPU 核心数 | 并行索引任务数 |
| `--format <fmt>` | text/json | `text` | 输出格式 |
| `--db <path>` | Path | 内存 | 图数据库文件路径 |
| `-v, --verbose` | flag | false | 启用调试日志 |

---

### 基础命令

#### `hdl-graph init [dir]`

初始化 `.hdl-graph/` 项目配置目录，生成默认 `config.toml`。

```bash
hdl-graph init .          # 在当前目录初始化
hdl-graph init ./myproj   # 在指定目录初始化
```

#### `hdl-graph index [--watch]`

解析 HDL 源码并构建代码图索引。扫描 `.sv`、`.svh`、`.v`、`.vh` 文件，使用 tree-sitter 解析，提取节点和边。

```bash
hdl-graph index           # 构建索引
hdl-graph index --watch   # 监听文件变更，增量更新（100ms 防抖）
```

#### `hdl-graph search <pattern>`

按名称模式搜索符号，支持大小写不敏感子串匹配和 glob 通配符（`*`、`?`）。

```bash
hdl-graph search fifo         # 搜索包含 "fifo" 的符号
hdl-graph search apb_*        # 搜索以 "apb_" 开头的符号
hdl-graph search *_driver     # 搜索以 "_driver" 结尾的符号
```

#### `hdl-graph stats`

显示图统计信息：文件数、节点/边总数，以及按类型分类的计数（模块、端口、信号、实例、always 块、赋值、类、包、函数等）。

```bash
hdl-graph stats
```

#### `hdl-graph check [--ci]`

运行图一致性检查，报告以下问题：
- **悬挂边** — 源或目标节点不存在的边
- **未解析实例** — `module_type` 未定义的模块实例
- **孤立节点** — 无任何边连接的节点（排除 SourceFile）
- **未解析父类** — `extends` 父类未定义的类

```bash
hdl-graph check          # 显示检查结果
hdl-graph check --ci     # CI 模式（失败时返回非零退出码）
```

#### `hdl-graph files [glob]`

列出已索引文件及每文件的统计信息（节点数、模块数、类数、实例数、信号数）。支持 glob 模式过滤。

```bash
hdl-graph files           # 列出所有文件
hdl-graph files "*.sv"    # 只列出 .sv 文件
hdl-graph files "rtl/*"   # 只列出 rtl/ 目录下的文件
```

#### `hdl-graph watch`

启动 LSP 语言服务器（stdio 模式），用于编辑器集成。

```bash
hdl-graph watch
```

#### `hdl-graph version`

打印版本号。

```bash
hdl-graph version
# 输出: hdl-graph 0.2.0
```

---

### 查询命令 (`hdl-graph query`)

#### `query def <symbol> [scope]`

查找符号的定义位置。支持模块、类、包、端口、信号、实例等符号类型。

```bash
hdl-graph query def my_module
hdl-graph query def data_in
hdl-graph query def apb_slave "file:100"
```

#### `query refs <symbol> [scope]`

查找符号在项目中的所有引用。追踪以下边类型：References、Drives、Extends、Calls、ConfigSets、ConfigGets、Instantiates、Connects、FactoryRegisters、FactoryOverrides、TLMBinds。

```bash
hdl-graph query refs clk
hdl-graph query refs reset_n
```

#### `query hierarchy <name>`

显示模块或类的实例化层次树，以树形格式展示子实例、信号、端口、always 块等。

```bash
hdl-graph query hierarchy top_module
hdl-graph query hierarchy my_env
```

#### `query calls <name>`

显示函数或任务的调用图，列出所有调用该函数的位置。

```bash
hdl-graph query calls my_function
hdl-graph query calls run_phase
```

#### `query drivers <signal>`

追踪信号的驱动（写操作）和读取（读操作），递归遍历包含关系。

```bash
hdl-graph query drivers data_out
hdl-graph query drivers fifo_wr_en
```

#### `query inst <module_type>`

查找模块类型在项目中的所有实例化位置，显示实例名、父模块和文件路径。

```bash
hdl-graph query inst fifo
hdl-graph query inst apb_slave
```

#### `query explore <name>`

探索模块或类的详细信息，一次性返回端口、信号、实例、always 块、赋值、函数/任务等分类信息。

```bash
hdl-graph explore my_module
hdl-graph explore my_class
```

#### `query impact <symbol>`

分析修改符号的影响范围（BFS 深度 3）。追踪 References、Drives、Calls、Instantiates、Connects、ConfigSets、ConfigGets、FactoryRegisters、FactoryOverrides、TLMBinds 等边类型。按深度分组报告受影响的节点。

```bash
hdl-graph query impact config_reg
hdl-graph query impact my_driver
```

#### `query node <symbol>`

获取符号的详细信息，包括：类型、节点 ID、父作用域、源文件、类型特定属性（端口方向、信号类型、实例化模块、类继承、函数/任务、虚方法、TLM 方向、工厂注册/覆盖）、出边和入边。

```bash
hdl-graph query node my_signal
hdl-graph query node apb_if
```

---

### UVM 命令 (`hdl-graph uvm`)

#### `uvm factory <type_name>`

显示 UVM 类型的工厂注册、类型覆盖和 `type_id::create` 调用。

```bash
hdl-graph uvm factory my_driver
hdl-graph uvm factory my_scoreboard
```

#### `uvm tlm <component>`

显示 UVM 组件的 TLM 端口连接，列出端口名称、方向和连接目标。

```bash
hdl-graph uvm tlm my_agent
hdl-graph uvm tlm my_env
```

#### `uvm config <path>`

显示匹配路径的 `uvm_config_db` set/get 操作。支持 `*` 通配符。

```bash
hdl-graph uvm config "*.driver.vif"
hdl-graph uvm config "env.agent.*"
```

#### `uvm hierarchy`

显示 UVM 类继承层次（extends 树），从根类开始递归展示。

```bash
hdl-graph uvm hierarchy
```

---

### 导出命令 (`hdl-graph export`)

#### `export scip <output>`

导出 SCIP JSON 格式，兼容 Sourcegraph 和 GitHub Code Search。

```bash
hdl-graph export scip index.scip
```

#### `export json <output>`

导出完整图为 JSON，包含所有节点、边、文件和元数据。

```bash
hdl-graph export json graph.json
```

#### `export markdown <output> [--per-module]`

导出为可读的 Markdown 文档。`--per-module` 选项为每个模块/类生成独立文件。

```bash
hdl-graph export markdown docs/                    # 单文件
hdl-graph export markdown docs/ --per-module        # 每模块一个文件
```

---

## MCP 服务器

hdl-graph 内置 MCP (Model Context Protocol) 服务器，为 Claude、Cursor 等 AI 助手提供 HDL 代码分析能力。服务器通过 stdio 传输，在启动时自动索引项目中的所有 HDL 文件。

### 启动

```bash
# 基本启动
hdl-graph-mcp /path/to/hdl/project

# 指定额外扫描目录
hdl-graph-mcp /path/to/hdl/project /path/to/includes
```

### MCP 工具列表 (15 个)

#### 符号查询

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_def` | `symbol: string` | 查找符号的定义位置（模块、类、端口、信号、函数）。返回类型、名称和节点 ID |
| `hdl_search` | `query: string` | 按模式搜索符号。支持 glob 通配符（`*` 和 `?`），大小写不敏感 |
| `hdl_node` | `symbol: string` | 获取符号的详细信息：类型、父作用域、源文件、类型特定属性、所有出边和入边 |
| `hdl_files` | `pattern?: string` | 列出已索引文件及每文件的节点/模块/类/实例/信号统计。可选 glob 过滤 |

#### 结构分析

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_hierarchy` | `name: string` | 显示模块/类/包/接口的实例化层次树，包含实例、端口、信号和子块 |
| `hdl_explore` | `name: string` | 探索模块或类的详情：端口（含方向）、信号（含类型）、实例（含模块类型）、always 块、赋值、函数/任务 |
| `hdl_inst` | `module_type: string` | 查找模块类型在项目中的所有实例化位置，返回实例名、父模块和文件路径 |

#### 关系追踪

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_callers` | `symbol: string` | 查找符号的所有调用者和引用。追踪 References、Drives、Calls、Instantiates、Connects 等边 |
| `hdl_callees` | `name: string` | 显示函数/任务的调用目标，列出所有被调用的函数 |
| `hdl_drivers` | `signal: string` | 追踪信号的驱动（写）和读取。递归遍历 always 块、赋值和模块 |
| `hdl_impact` | `symbol: string` | 分析修改的影响范围。BFS 深度 3，按层级分组报告所有受影响的节点 |

#### UVM 分析

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_uvm` | `analysis: string`, `query?: string` | UVM 综合分析。`analysis` 取值：`factory`（工厂注册/覆盖）、`tlm`（TLM 端口连接）、`config`（config_db set/get）、`hierarchy`（类继承层次） |

#### 图管理

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_stats` | — | 获取图统计信息：按类型分类的节点/边计数（模块、信号、实例、UVM 组件等） |
| `hdl_check` | — | 运行图一致性检查：悬挂边、未解析模块实例、孤立节点、未解析父类 |
| `hdl_export` | `format: string`, `output: string` | 导出代码图。`format` 取值：`scip`（Sourcegraph）、`json`（完整图）、`markdown`（文档） |

### 客户端配置

#### Claude Desktop

编辑 `~/Library/Application Support/Claude/claude_desktop_config.json`（macOS）或 `%APPDATA%\Claude\claude_desktop_config.json`（Windows）：

```json
{
  "mcpServers": {
    "hdl-graph": {
      "command": "hdl-graph-mcp",
      "args": ["/path/to/your/hdl/project"]
    }
  }
}
```

#### Claude Code

在项目根目录创建 `.mcp.json`：

```json
{
  "mcpServers": {
    "hdl-graph": {
      "command": "hdl-graph-mcp",
      "args": ["."]
    }
  }
}
```

#### Cursor

在 Cursor 设置中添加 MCP 服务器：

```json
{
  "mcpServers": {
    "hdl-graph": {
      "command": "hdl-graph-mcp",
      "args": ["/path/to/your/hdl/project"]
    }
  }
}
```

---

## LSP 服务器

hdl-graph 内置 LSP (Language Server Protocol) 服务器，通过 `hdl-graph watch` 命令启动，支持 VS Code、Neovim、Emacs 等编辑器。

### VS Code

安装 `hdl-graph.vsix` 扩展后自动配置。或手动在 `settings.json` 中添加：

```json
{
  "hdl-graph.serverPath": "hdl-graph"
}
```

### Neovim

使用 `nvim-lspconfig` 配置：

```lua
require('lspconfig').hdl_graph.setup {
  cmd = { 'hdl-graph', 'watch' },
  filetypes = { 'systemverilog', 'verilog' },
  root_dir = require('lspconfig.util').find_git_ancestor,
}
```

---

## 架构

```
hdl-codegraph/
├── crates/
│   ├── hdl-graph-cli       ← CLI 主二进制（入口）
│   ├── hdl-graph-mcp       ← MCP 服务器（AI 工具集成）
│   ├── hdl-graph-core      ← 核心数据结构（Graph trait、Node、Edge、Symbol）
│   ├── hdl-graph-grammar   ← tree-sitter SystemVerilog 语法（816 语法规则）
│   ├── hdl-graph-parse     ← CST 遍历和节点提取
│   ├── hdl-graph-storage   ← RocksDB 持久化 + InMemoryGraph
│   ├── hdl-graph-query     ← 查询引擎和 SCIP/JSON/Markdown 导出
│   ├── hdl-graph-lsp       ← LSP 语言服务器
│   ├── hdl-graph-build     ← 构建辅助
│   ├── hdl-graph-types     ← 共享类型定义
│   └── hdl-graph-web       ← Web 接口
├── rmcp-sdk/               ← MCP SDK（嵌入式依赖）
├── vscode-extension/       ← VS Code 扩展
├── tests/                  ← 集成测试和 fixture
├── docs/                   ← 文档
├── scripts/                ← 构建和打包脚本
└── install.sh              ← 一键安装脚本
```

### 核心依赖

| 依赖 | 版本 | 用途 |
|------|------|------|
| `tree-sitter` | 0.25 | 增量解析器 |
| `rocksdb` | 0.24 | 持久化存储 |
| `rmcp` | 1.7.0 | MCP 协议（内嵌 rmcp-sdk） |
| `clap` | 4 | CLI 参数解析 |
| `tokio` | 1 | 异步运行时 |
| `tower-lsp` | 0.20 | LSP 协议 |
| `serde` / `serde_json` | 1 | 序列化 |
| `sha2` | 0.10 | 文件哈希（增量索引） |
| `notify` | 7 | 文件变更监听 |

---

## 开发

### 构建

```bash
git clone https://github.com/hedge88/hdl-codegraph.git
cd hdl-codegraph

# 编译所有 crate
cargo build --release

# 仅编译 CLI
cargo build --release -p hdl-graph-cli

# 仅编译 MCP 服务器
cargo build --release -p hdl-graph-mcp
```

### 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行特定 crate 测试
cargo test -p hdl-graph-parse
```

### VS Code 扩展开发

```bash
cd vscode-extension
npm install
npm run compile
# 按 F5 启动调试
```

---

## 项目状态

| 阶段 | 进度 |
|------|------|
| Phase 1: 基础架构 | ✅ 完成 |
| Phase 2: SystemVerilog | ✅ 完成 |
| Phase 3: UVM | ✅ 完成 |
| Phase 4: 生产就绪 | 🟢 进行中 |

---

## 文档

- [CHANGELOG.md](CHANGELOG.md) — 版本变更记录
- [OFFLINE_INSTALL.md](OFFLINE_INSTALL.md) — 离线安装指南
- [docs/user-guide.md](docs/user-guide.md) — 用户指南
- [docs/developer-guide.md](docs/developer-guide.md) — 开发者指南
- [PORTING.md](PORTING.md) — 跨平台移植指南

---

## 许可证

MIT
