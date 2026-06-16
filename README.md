# HDL Code Graph

Verilog / SystemVerilog / UVM 代码智能分析工具。将 HDL 源码解析为可查询的代码图，支持定义跳转、引用查找、模块层次分析、UVM 工厂/TLM/config_db 追踪、信号驱动追踪，以及 MCP 和 LSP 集成。

## 快速开始

```bash
# 初始化项目
hdl-graph init .

# 索引源码
hdl-graph index

# 查询
hdl-graph query def fifo              # 查找定义
hdl-graph query refs fifo             # 查找引用
hdl-graph query hierarchy top         # 模块层次树
hdl-graph search apb_*                # 搜索符号
hdl-graph uvm factory my_driver       # UVM 工厂分析
```

## 安装

```bash
# macOS (Homebrew)
brew install hdl-graph/tap/hdl-graph

# Cargo
cargo install hdl-graph-cli

# Linux (deb)
sudo dpkg -i hdl-graph_0.2.0_amd64.deb

# VS Code 扩展
# 从 marketplace 安装: hdl-graph.vsix

# 离线安装（无网络环境）
# 参见 OFFLINE_INSTALL.md
```

### 离线安装

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

自定义路径：`bash install.sh --prefix=/opt/tools`

详见 [OFFLINE_INSTALL.md](OFFLINE_INSTALL.md)。

## 功能特性

- **模块层次树** — 查看设计结构
- **定义跳转** — 定位任意标识符的声明位置
- **引用查找** — 查找信号/模块/类的所有使用点
- **信号流追踪** — 追踪信号的驱动和读取
- **影响分析** — 评估修改的影响范围（BFS 爆破半径）
- **UVM 支持** — 工厂注册、类型覆盖、TLM 连接、config_db 追踪
- **MCP 服务器** — 为 AI 助手提供 15 个代码分析工具
- **LSP 服务器** — 集成 VS Code / Neovim / Emacs
- **SCIP 导出** — 兼容 Sourcegraph 和 GitHub Code Search
- **增量索引** — 编辑时实时更新

## CLI 命令

### 基础命令

| 命令 | 说明 |
|------|------|
| `hdl-graph init [dir]` | 初始化 `.hdl-graph` 项目配置 |
| `hdl-graph index [--watch]` | 索引 HDL 源码，构建代码图 |
| `hdl-graph search <pattern>` | 按名称模式搜索符号（支持通配符） |
| `hdl-graph stats` | 显示图统计：节点/边数量和分类 |
| `hdl-graph check [--ci]` | 运行一致性检查（悬挂边、未解析实例等） |
| `hdl-graph files [glob]` | 列出已索引文件及统计信息 |
| `hdl-graph watch` | 启动 LSP 语言服务器（stdio） |
| `hdl-graph version` | 显示版本号 |

### 查询命令 (`hdl-graph query`)

| 命令 | 说明 |
|------|------|
| `query def <symbol>` | 查找符号的定义位置 |
| `query refs <symbol>` | 查找符号的所有引用 |
| `query hierarchy <name>` | 显示模块/类的实例化层次树 |
| `query calls <name>` | 显示函数/任务的调用图 |
| `query drivers <signal>` | 追踪信号的驱动（写）和读取 |
| `query inst <module>` | 查找模块的所有实例化位置 |
| `query explore <name>` | 探索模块详情：端口、信号、实例、always 块 |
| `query impact <symbol>` | 分析修改的影响范围（BFS 深度 3） |
| `query node <symbol>` | 获取符号的详细信息 |

### UVM 命令 (`hdl-graph uvm`)

| 命令 | 说明 |
|------|------|
| `uvm factory <type>` | 显示 UVM 类型的工厂注册、覆盖和 create 调用 |
| `uvm tlm <component>` | 显示 UVM 组件的 TLM 端口连接 |
| `uvm config <path>` | 显示匹配路径的 `uvm_config_db` set/get 操作 |
| `uvm hierarchy` | 显示 UVM 类继承层次（extends 树） |

### 导出命令 (`hdl-graph export`)

| 命令 | 说明 |
|------|------|
| `export scip <output>` | 导出 SCIP JSON（用于 Sourcegraph / GitHub Code Search） |
| `export json <output>` | 导出完整图为 JSON（节点、边、文件、元数据） |
| `export markdown <output>` | 导出为可读的 Markdown 文档 |

### 全局选项

| 选项 | 说明 |
|------|------|
| `--project <dir>` | HDL 项目根目录（默认：`.`） |
| `--include-dirs <dirs>` | 额外扫描目录 |
| `--uvm-home <dir>` | UVM 库目录 |
| `--defines <DEF>` | 预处理器宏定义 |
| `--jobs <N>` | 并行索引任务数（默认：CPU 核心数） |
| `--format <fmt>` | 输出格式：`text`（默认）或 `json` |
| `--db <path>` | 图数据库路径（默认：内存） |
| `-v, --verbose` | 启用调试日志 |

## MCP 服务器

hdl-graph 内置 MCP (Model Context Protocol) 服务器，可为 Claude、Cursor 等 AI 助手提供 HDL 代码分析能力。

### 启动 MCP 服务器

```bash
# 直接启动（stdio 模式）
hdl-graph-mcp --project /path/to/hdl/project

# 或指定数据库
hdl-graph-mcp --project . --db .hdl-graph/db
```

### MCP 工具列表 (15 个)

| 工具 | 参数 | 说明 |
|------|------|------|
| `hdl_search` | `query` | 按模式搜索符号 |
| `hdl_hierarchy` | `name` | 模块/类实例化层次树 |
| `hdl_callers` | `symbol` | 查找符号的调用者/引用 |
| `hdl_callees` | `name` | 显示函数/任务的调用目标 |
| `hdl_drivers` | `signal` | 追踪信号的驱动和读取 |
| `hdl_uvm` | `analysis`, `query?` | UVM 分析（factory/tlm/config/hierarchy） |
| `hdl_explore` | `name` | 探索模块详情 |
| `hdl_impact` | `symbol` | 修改影响范围分析 |
| `hdl_node` | `symbol` | 获取符号详细信息 |
| `hdl_def` | `symbol` | 查找符号定义位置 |
| `hdl_inst` | `module_type` | 查找模块的所有实例化 |
| `hdl_stats` | — | 图统计信息 |
| `hdl_files` | `pattern?` | 列出已索引文件 |
| `hdl_check` | — | 图一致性检查 |
| `hdl_export` | `format`, `output` | 导出图（scip/json/markdown） |

### Claude Desktop 配置

```json
{
  "mcpServers": {
    "hdl-graph": {
      "command": "hdl-graph-mcp",
      "args": ["--project", "/path/to/your/hdl/project"]
    }
  }
}
```

### Claude Code 配置

```json
{
  "mcpServers": {
    "hdl-graph": {
      "command": "hdl-graph-mcp",
      "args": ["--project", "."]
    }
  }
}
```

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

### 外部依赖

| 依赖 | 用途 |
|------|------|
| `tree-sitter` | 增量解析器 |
| `rocksdb` | 持久化存储 |
| `rmcp` | MCP 协议（内嵌 rmcp-sdk） |
| `clap` | CLI 参数解析 |
| `tokio` | 异步运行时 |
| `tower-lsp` | LSP 协议 |
| `serde` / `serde_json` | 序列化 |
| `sha2` | 文件哈希（增量索引） |
| `notify` | 文件变更监听（watch 模式） |

## 开发

```bash
# 克隆仓库
git clone https://github.com/hudge88/hdl-codegraph.git
cd hdl-codegraph

# 编译所有 crate
cargo build --release

# 运行测试
cargo test --workspace

# 仅编译 CLI
cargo build --release -p hdl-graph-cli

# 仅编译 MCP 服务器
cargo build --release -p hdl-graph-mcp
```

### VS Code 扩展开发

```bash
cd vscode-extension
npm install
npm run compile
# 按 F5 启动调试
```

## 项目状态

| 阶段 | 进度 |
|------|------|
| Phase 1: 基础架构 | ✅ 完成 |
| Phase 2: SystemVerilog | ✅ 完成 |
| Phase 3: UVM | ✅ 完成 |
| Phase 4: 生产就绪 | 🟢 进行中 |

## 文档

- [CHANGELOG.md](CHANGELOG.md) — 版本变更记录
- [OFFLINE_INSTALL.md](OFFLINE_INSTALL.md) — 离线安装指南
- [docs/user-guide.md](docs/user-guide.md) — 用户指南
- [docs/developer-guide.md](docs/developer-guide.md) — 开发者指南
- [PORTING.md](PORTING.md) — 跨平台移植指南

## 许可证

MIT
