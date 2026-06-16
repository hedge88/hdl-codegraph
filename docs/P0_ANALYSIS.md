# hdl-graph P0 修复分析 — ultracode 并行执行评估

> 基于 hdl-codegraph v0.1.0 源码深度审查，针对 amba_svt (481 files, 1870 classes) 实际测试暴露的三个 P0 问题。

---

## 🔍 根因诊断

### P0-1: Module/Port/Signal/Always/Package 统计全为 0

**现象**: `stats` 显示 Modules=2, Ports=0, Signals=0, Always=0, Packages=0, Functions=0

**根因**: `cmd_stats` (main.rs:1098-1155) 只统计**直接子节点**，不递归：

```rust
// main.rs:1120 — 只看 file → child 两层
if let Ok(kids) = state.graph.get_outgoing(e.target) {
    for ke in &kids {
        // 只统计 Module 的直接子节点
    }
}
```

但 amba_svt 的实际结构是：
```
SourceFile → Class → {Property, Method → {CallSite, Assignment}}
```

所有 Module/Port/Signal/Always 节点都挂在 Class 节点下面，而不是直接挂在 SourceFile 下。`cmd_stats` 的两层遍历完全漏掉了它们。

**同时**，`collect_sv_files` (main.rs:582-622) 只收集 `.sv/.svh/.v/.vh` 扩展名，**不收集 `.svi` 和 `.pkg`**：
```rust
if matches!(ext, "sv" | "svh" | "v" | "vh") {  // ← 缺少 "svi", "pkg"
```

所以 153 个 include 文件完全没被索引。

**修复方案**:

1. **`collect_sv_files`**: 加入 `"svi"`, `"pkg"` 扩展名
2. **`cmd_stats`**: 改为递归遍历整个图，或改为按 NodeKind 全局统计（遍历所有节点而非按文件层级）

```rust
// 修复: 全局统计而非层级遍历
fn cmd_stats(state: &ProjectState) -> anyhow::Result<()> {
    let mut counts = HashMap::new();
    // 遍历所有节点，按 kind 分类计数
    for node in state.graph.all_nodes() {  // 需要新增 all_nodes() 方法
        *counts.entry(kind_name(&node.kind)).or_insert(0) += 1;
    }
    // ...
}
```

### P0-2: `query refs` / `query calls` 完全失效

**现象**: `refs svt_axi_transaction` → "No references found", `calls svt_axi_master_sequencer` → "(none found)"

**根因**: 这是**两层问题叠加**：

**问题 A — References/Drives 边从未被创建**:

`cmd_refs` (main.rs:727-765) 搜索的是 `EdgeType::References` 和 `EdgeType::Drives` 边。但这些边只在以下场景创建：
- `extract_procedural_assignment` — 阻塞/非阻塞赋值中 LHS/RHS 信号引用
- `extract_refs_from_expr` — 表达式中的信号引用

对于 **class 级别的类型引用**（如 `svt_axi_transaction xact` 声明、`extends svt_axi_transaction`），没有任何 extractor 创建 References 边。Class 的 extends 创建的是 `EdgeType::Extends` 边，但 `cmd_refs` 不搜索 Extends 边。

**问题 B — `cmd_refs` 搜索深度不足**:

```rust
// main.rs:731-758 — 只搜索两层: file → container → children
for (_file, fid) in &state.file_map {
    let outgoing = state.graph.get_outgoing(*fid)?;     // 第1层
    for edge in &outgoing {
        // ...
        let container_edges = state.graph.get_outgoing(edge.target)?;  // 第2层
        // 只检查 References/Drives 边
        recurse_refs(...)  // 递归，但也只检查 References/Drives
    }
}
```

即使边存在，对于深层嵌套（Class → Method → BeginBlock → Assignment → References），递归函数只沿着 `Contains` 边往下走，但 amba_svt 的 class 方法体没有被深度解析。

**修复方案**:

1. **扩展 `cmd_refs`**: 搜索所有边类型（References, Drives, Extends, Calls, ConfigSets, ConfigGets 等）
2. **新增 class 级类型引用边**: 在 `extract_class_property` 中，对类型声明创建 `References` 边
3. **新增全局节点索引**: `InMemoryGraph` 增加 `nodes_by_name: HashMap<String, Vec<u64>>` 反向索引，避免每次查询都遍历全部文件

```rust
// 修复: cmd_refs 搜索所有语义边
fn cmd_refs(state: &ProjectState, symbol: &str, _scope: Option<&str>) -> anyhow::Result<()> {
    // 1. 先通过 name 索引找到目标节点 ID
    let target_ids = state.symbols.find_by_name(symbol);
    // 2. 遍历所有节点的 incoming 边，找到指向 target 的
    for target_id in &target_ids {
        let incoming = state.graph.get_incoming(*target_id)?;
        for edge in &incoming {
            if matches!(edge.edge_type,
                EdgeType::References | EdgeType::Drives | EdgeType::Extends
                | EdgeType::Calls | EdgeType::ConfigSets | EdgeType::ConfigGets
            ) {
                // 输出
            }
        }
    }
}
```

### P0-3: UVM 语义分析全部失效 (factory/config_db/TLM)

**现象**: `uvm factory svt_axi_transaction` → "(no factory info found)"

**根因**: 这是**预处理器未被调用**的问题。

代码中有一个完整的 4-pass 预处理器 (`preprocessor/mod.rs`)，能展开 `` `uvm_component_utils `` → `typedef uvm_component_registry #(T, "T") type_id;`，然后 `extract_factory_registration` 就能识别。

但 **`cmd_index` 和 `load_or_build` 直接调用 `scanner.parse_file()`，完全没有调用预处理器**：

```rust
// main.rs:270-274 — 直接 parse，没有 preprocess
match scanner.parse_file(path) {
    Ok(tree) => {
        let source = std::fs::read_to_string(path).unwrap_or_default();
        let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), 0);
```

而 `FileScanner::parse_file()` 也只是直接调 tree-sitter：
```rust
// scanner.rs:28-35
pub fn parse_file(&mut self, path: &Path) -> Result<tree_sitter::Tree> {
    let content = std::fs::read_to_string(path)?;
    let tree = self.parser.parse(&content, None)...;
    Ok(tree)
}
```

所以 `` `uvm_component_utils(my_driver) `` 没有被展开，tree-sitter 把它当作一个宏调用节点，而不是 `typedef uvm_component_registry...`，导致 `extract_factory_registration` 永远匹配不到。

**关键发现 — 宏展开同时影响 extends 边**:

amba_svt 的 class 继承大量使用宏：
```systemverilog
class svt_axi_transaction extends `SVT_TRANSACTION_TYPE;    // ← 宏！
class svt_agent extends `SVT_XVM(agent);                    // ← 宏！
```

`find_parent_class` (class.rs:77-89) 在 `extends` 后找 `class_type` 节点，但 `` `SVT_TRANSACTION_TYPE `` 被 tree-sitter 解析为宏调用而非 `class_type`，导致：
- **所有 `Extends` 边都丢失** → UVM 类型层次树完全扁平化
- **`uvm hierarchy` 只显示顶层无父类的类**（21 个文件使用 `uvm_config_db`，全部丢失）

同理，`uvm_config_db#(T)::set(...)` 和 TLM port 声明在原始源码中是合法的 SV 语法（不依赖宏展开），但 **config_db 的 `is_uvm_config_db_set` 检查 `class_type` 节点是否以 `"uvm_config_db"` 开头** — 如果 tree-sitter 把 `uvm_config_db#(int)` 解析为泛型类型节点而非 `class_type`，匹配也会失败。

**修复方案**:

1. **在 `cmd_index` / `load_or_build` 中集成预处理器**:

```rust
// 修复: parse 前先 preprocess
fn index_with_preprocess(project: &Path, include_dirs: &[String]) -> ... {
    let defines = collect_defines(include_dirs);  // 从 CLI --defines 收集
    for path in &files {
        let source = std::fs::read_to_string(path)?;
        let preprocessed = hdl_graph_parse::preprocessor::preprocess(
            &source, &rel, &defines, include_dirs
        );
        let tree = scanner.parse_source(&preprocessed.expanded_source);
        let (nodes, edges) = extractor.extract(&tree, preprocessed.expanded_source.as_bytes(), 0);
        // ...
    }
}
```

2. **`FileScanner` 增加 `parse_with_preprocess` 方法**:
```rust
pub fn parse_with_preprocess(&mut self, source: &str, path: &str, defines: &HashMap<String, String>) -> (Tree, String) {
    let preprocessed = preprocessor::preprocess(source, path, defines, &self.include_dirs);
    let tree = self.parser.parse(&preprocessed.expanded_source, None).unwrap();
    (tree, preprocessed.expanded_source)
}
```

---

## 📊 并行化分析

### 当前瓶颈

`cmd_index` 是**完全串行**的单线程循环：

```rust
// main.rs:266 — 逐文件串行
for path in &files {
    let tree = scanner.parse_file(path)?;
    let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), 0);
    for n in &nodes { graph.add_node(n.clone()).ok(); }
    for e in &edges { graph.add_edge(e.clone()).ok(); }
}
```

三个阶段的并行化机会：

### 阶段 1: 文件扫描 — 无需并行（已很快）

`walkdir` 遍历文件系统，不是瓶颈。

### 阶段 2: 解析 + 提取 — 可并行 ✅

**关键约束**:
- `tree_sitter::Parser` 不是 `Send`，每个线程需要独立的 Parser 实例
- `GraphExtractor` 有 `scope_symbols` 状态，每个线程需要独立的 Extractor
- `SymbolTable` 的 `intern()` 需要全局唯一 — 需要 `Arc<Mutex<SymbolTable>>` 或事后合并

**方案**: **parse 阶段并行，graph 构建阶段串行合并**

```rust
use rayon::prelude::*;

fn cmd_index_parallel(project: &Path, include_dirs: &[String]) -> anyhow::Result<()> {
    let files = collect_sv_files(project, include_dirs);

    // Phase 1: 并行 parse + extract（每个线程独立的 parser + extractor）
    let results: Vec<(String, Vec<GraphNode>, Vec<Edge>)> = files
        .par_iter()  // rayon 并行迭代
        .filter_map(|path| {
            let rel = path.strip_prefix(project).unwrap_or(path)
                .to_string_lossy().to_string();
            let source = std::fs::read_to_string(path).ok()?;

            // 每个线程创建独立的 parser 和 extractor
            let mut parser = tree_sitter::Parser::new();
            parser.set_language(&hdl_graph_grammar::language_ref()).ok()?;
            let tree = parser.parse(&source, None)?;

            let mut extractor = GraphExtractor::new();
            let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), 0);

            Some((rel, nodes, edges, extractor.symbols))
        })
        .collect();

    // Phase 2: 串行合并到全局 graph（避免锁竞争）
    let mut graph = InMemoryGraph::new();
    let mut global_symbols = SymbolTable::new();
    for (rel, nodes, edges, local_symbols) in results {
        // 合并 symbol table
        for (name, _id) in local_symbols.iter() {
            global_symbols.intern(name);
        }
        for n in &nodes { graph.add_node(n.clone()).ok(); }
        for e in &edges { graph.add_edge(e.clone()).ok(); }
    }

    Ok(())
}
```

**预期收益**: amba_svt 481 文件，当前串行 ~5s，并行后 ~1-2s（取决于 CPU 核数）。

### 阶段 3: Graph 查询 — 难以并行

`cmd_refs` / `cmd_calls` 等查询是单次遍历，数据量不大（20K 节点），不是瓶颈。

### 并行化风险评估

| 风险 | 严重度 | 缓解方案 |
|------|--------|----------|
| SymbolTable ID 不一致 | 高 | 每个线程独立 SymbolTable，合并时用 name 重映射 |
| node ID 冲突 | 高 | 每个线程用独立的 `next_id` 起始值（线程 i 从 `i * 100_000` 开始） |
| scope_symbols 跨文件泄漏 | 中 | 当前已经是每文件独立的（`new()` 创建新 extractor） |
| 预处理器 define 跨文件传播 | 中 | 预处理器需要全局 define 表，需要 `Arc<HashMap>` 共享 |

### 推荐并行化策略

**Phase 1 — 最小改动，最大收益**:

```rust
// 将 cmd_index 的 for 循环改为 rayon par_iter
// 需要: Cargo.toml 加 rayon = "1.8"
// 需要: GraphExtractor::new() 改为接受起始 ID 参数
// 需要: SymbolTable 改为支持 merge
```

**Phase 2 — 预处理器集成**:

```rust
// 在 parallel parse 前，先串行收集全局 defines
// 然后将 defines 作为 Arc<HashMap> 传入并行 parse
// 预处理器本身的 ifdef/define 逻辑需要 thread-safe
```

---

## 🗂️ 修复任务拆分

### Task 1: 收集文件扩展名修复 [10 min]
- 文件: `main.rs:598-599`
- 改动: `matches!(ext, "sv" | "svh" | "v" | "vh")` → 加 `"svi" | "pkg"`
- 验证: `hdl-graph index` 应显示 634 文件而非 481

### Task 2: stats 递归统计 [30 min]
- 文件: `main.rs:1098-1155`
- 改动: 新增 `InMemoryGraph::all_nodes()` 方法，`cmd_stats` 改为全局遍历
- 验证: `stats` 应显示非零的 Ports/Signals/Always/Packages/Functions

### Task 3: cmd_refs 搜索所有语义边 [1 h]
- 文件: `main.rs:727-765`
- 改动:
  1. `cmd_refs` 搜索 `Extends`, `Calls`, `ConfigSets`, `ConfigGets` 等边
  2. 增加反向索引 `nodes_by_kind` 加速查询
- 验证: `query refs svt_axi_transaction` 应返回结果

### Task 4: 集成预处理器到 index pipeline [2 h]
- 文件: `main.rs:253-303`, `main.rs:628-660`, `scanner.rs`
- 改动:
  1. `cmd_index` / `load_or_build` 中调用 `preprocessor::preprocess()`
  2. `FileScanner` 增加 `parse_with_preprocess` 方法
  3. 处理 `--defines` 参数传递
- 验证: `uvm factory svt_axi_transaction` 应返回 FactoryReg 信息

### Task 5: 并行化 index [1.5 h]
- 文件: `main.rs:253-303`, `Cargo.toml`
- 改动:
  1. `Cargo.toml` 加 `rayon = "1.8"`
  2. `GraphExtractor::new()` 改为接受 `start_id` 参数
  3. `SymbolTable` 增加 `merge()` 方法
  4. `cmd_index` 改为 `par_iter` 并行 parse + 串行 merge
- 验证: `time hdl-graph index` 应显示明显加速

---

## 📐 依赖关系图

```
Task 1 (文件扩展名)  ←── 独立，可立即开始
Task 2 (stats 递归)   ←── 独立，可立即开始
Task 3 (refs 语义边)  ←── 依赖 Task 2 (需要 all_nodes 方法)
Task 4 (预处理器)     ←── 独立，可立即开始
Task 5 (并行化)       ←── 依赖 Task 1 + Task 4
```

**并行执行计划**:

```
Wave 1 (并行): Task 1 + Task 2 + Task 4
Wave 2:        Task 3 (依赖 Task 2)
Wave 3:        Task 5 (依赖 Task 1 + Task 4)
```

**预计总工时**: ~5 h（串行），~3.5 h（并行 Wave 1 三任务同时进行）

---

## 🧪 测试策略

每个 Task 完成后，用 amba_svt 项目验证：

```bash
# Task 1 验证
hdl-graph index --project C:/tmp/amba --include-dirs amba_svt/include/sverilog
# 应显示: "Indexing 634 files..." 而非 481

# Task 2 验证
hdl-graph stats --project C:/tmp/amba
# Modules/Ports/Signals/Always/Packages/Functions 应全为非零

# Task 3 验证
hdl-graph query refs svt_axi_transaction --project C:/tmp/amba
# 应返回 class/sequence/monitor 等引用

# Task 4 验证
hdl-graph uvm factory svt_axi_transaction --project C:/tmp/amba
hdl-graph uvm config "svt_axi" --project C:/tmp/amba
hdl-graph uvm tlm svt_axi_master_agent --project C:/tmp/amba
# 三者都应返回结果

# Task 5 验证
time hdl-graph index --project C:/tmp/amba --include-dirs amba_svt/include/sverilog
# 应比串行版本快 2-4x
```
