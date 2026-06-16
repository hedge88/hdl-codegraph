# Changelog

All notable changes to hdl-graph will be documented in this file.

## [0.2.0] — 2026-06-16

### 统一 CLI 与 MCP 命令集

CLI 和 MCP 服务器现在提供完全一致的 15 个查询/分析工具。

#### 新增 CLI 命令 (来自 MCP)

- `query explore <name>` — 深度探索模块/类：端口、信号、实例、always 块、函数一览
- `query impact <symbol>` — 变更影响分析，BFS 深度 3 爆炸半径
- `query node <symbol>` — 符号详细信息：类型、父作用域、源文件、所有出入边
- `files [pattern]` — 列出已索引文件，按文件统计模块/类/实例/信号数量

#### 新增 MCP 工具 (来自 CLI)

- `hdl_def` — 查找符号定义位置 (module、class、port、signal、function)
- `hdl_inst` — 查找模块类型的所有实例化
- `hdl_check` — 图一致性检查：悬空边、未解析实例、孤立节点、未解析父类
- `hdl_export` — 导出代码图 (SCIP/JSON/Markdown 格式)

#### 基础设施

- 新增 `crates/hdl-graph-core/src/helpers.rs` — 提取共享工具函数到 core crate，消除 CLI/MCP 间的代码重复
  - `node_kind_str` / `kind_display_name` — 节点类型到字符串映射 (覆盖全部 30+ NodeKind 变体)
  - `node_label` — 人类可读的节点标签
  - `find_file_for_node` — 通过 Contains 边查找节点所属文件 (最深 3 跳)
  - `find_containing_module` — 向上追溯包含模块
  - `is_ref_edge` / `is_impact_edge` — 边类型分类
  - `glob_match` — 简单 glob 模式匹配 (支持 `*` 和 `?`)

#### 修复

- 将 `rmcp-sdk` (MCP SDK) 从外部路径依赖移入项目内部，修复离线编译缺失 rmcp 源码的问题
- 重新 vendor 全部 Cargo 依赖 (242 → 352 个 crate)

### 完整命令对照表

| 能力 | CLI 命令 | MCP 工具 |
|------|---------|---------|
| 符号搜索 | `search <pattern>` | `hdl_search` |
| 定义查找 | `query def <symbol>` | `hdl_def` |
| 查找引用 | `query refs <symbol>` | `hdl_callers` |
| 实例查找 | `query inst <module>` | `hdl_inst` |
| 模块层次 | `query hierarchy <name>` | `hdl_hierarchy` |
| 调用图 | `query calls <name>` | `hdl_callees` |
| 信号驱动 | `query drivers <signal>` | `hdl_drivers` |
| 深度探索 | `query explore <name>` | `hdl_explore` |
| 影响分析 | `query impact <symbol>` | `hdl_impact` |
| 符号详情 | `query node <symbol>` | `hdl_node` |
| 文件列表 | `files [pattern]` | `hdl_files` |
| UVM 分析 | `uvm factory/tlm/config/hierarchy` | `hdl_uvm` |
| 图统计 | `stats` | `hdl_stats` |
| 一致性检查 | `check` | `hdl_check` |
| 导出 | `export scip/json/markdown` | `hdl_export` |

---

## [0.1.1] — 2026-06-12

### 初始版本

- tree-sitter SystemVerilog 解析器 (816 条语法规则)
- 代码图构建：模块、端口、信号、实例、函数、类、包
- 边类型：Contains, References, Drives, Calls, Instantiates, Extends, Connects, ConfigSets/Get, FactoryRegisters/Overrides, TLMBinds
- CLI 命令：init, index, query (def/refs/hierarchy/calls/drivers/inst), uvm, search, stats, check, export, watch (LSP)
- MCP 服务器：11 个工具 (search, hierarchy, callers, callees, drivers, uvm, explore, stats, impact, node, files)
- LSP 服务器 (tower-lsp)
- SCIP 导出 (Sourcegraph 兼容)
- JSON / Markdown 导出
- 增量索引 (tree-sitter 增量解析 + 文件 hash 去重)
- RocksDB 持久化存储
- VS Code 扩展
- 离线安装支持 (vendor + rustup-init)
