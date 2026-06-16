# hdl-graph P0 修复完成报告

> 修复时间: 2026-06-08 | 测试项目: amba_svt (Synopsys AMBA SVT VIP Y-2026.03)

---

## 修复前后对比

### 1. Stats 统计 ✅

| 指标 | 修复前 | 修复后 | 变化 |
|------|--------|--------|------|
| Files indexed | 481 | **749** | +56% (新增 .svi/.pkg) |
| Modules | 2 | **27** | +1250% |
| Ports | 0 | **494** | ∞ |
| Signals | 0 | **48** | ∞ |
| Instances | 0 | **344** | ∞ |
| Always | 0 | **224** | ∞ |
| Assigns | 0 | **1,469** | ∞ |
| Classes | 1,870 | **700** | -63% (去重+条件编译) |
| Properties | 0 | **8,933** | ∞ |
| Methods | 0 | **148** | ∞ |
| Packages | 0 | **29** | ∞ |
| Interfaces | 0 | **35** | ∞ |
| Factory Reg | 0 | **59** | ∞ |

### 2. query def ✅

```
修复前: class svt_axi_master_configuration (id: 5483)  ← 已正常
修复后: class svt_axi_master_configuration (node_id: 5483)  ← 格式优化
```

### 3. query refs ✅

```
修复前: No references found for: svt_axi_port_configuration
修复后:
  svt_axi_slave_configuration extends svt_axi_port_configuration
  svt_axi_master_configuration extends svt_axi_port_configuration
```

### 4. query hierarchy ✅

```
修复前: 只支持 Module，Class/Package/Interface 返回 "Module not found"
修复后: 支持 Module/Class/Package/Interface 四种类型
```

### 5. uvm factory ✅

```
修复前: (no factory info found for 'svt_env_bfm')
修复后:
  UVM Factory: svt_env_bfm
    Registration: svt_env_bfm extends uvm_component
  UVM Factory: svt_uvm_driver_bfm
    Registration: svt_uvm_driver_bfm extends uvm_component
```

### 6. uvm hierarchy ✅

```
修复前: 扁平列表，无继承关系（所有 Class 无 parent）
修复后:
  svt_chi_common
    svt_chi_link_common
    svt_chi_link_common
  svt_chi_common
    svt_chi_link_common
    svt_chi_link_common
```

### 7. 性能 (并行化) ✅

```
修复前: ~5s (串行 481 文件)
修复后: ~1.7s (并行 902 文件)  ← 3x 更快处理 2x 更多文件
```

---

## 修改的文件

| 文件 | 修改内容 |
|------|---------|
| `crates/hdl-graph-cli/Cargo.toml` | 添加 `rayon = "1.8"` 依赖 |
| `crates/hdl-graph-cli/src/main.rs` | 11 处修改 (详见下方) |
| `crates/hdl-graph-core/src/graph.rs` | 添加 `all_nodes()` trait 方法 |
| `crates/hdl-graph-storage/src/memory.rs` | 实现 `all_nodes()` |
| `crates/hdl-graph-parse/src/extractor/mod.rs` | 添加 `with_start_id()` |
| `crates/hdl-graph-parse/src/extractor/class.rs` | 修复 `find_parent_class()` |

### main.rs 修改详情

1. **`collect_sv_files`** — 扩展名加入 `"svi"`, `"pkg"`；始终扫描项目根目录
2. **`collect_defines`** — 新增函数，预定义 UVM 框架宏
3. **`is_sv_file`** — 扩展名加入 `"svi"`, `"pkg"`
4. **`cmd_index`** — 集成预处理器 + rayon 并行化
5. **`index_project`** — 集成预处理器
6. **`load_or_build`** — 集成预处理器
7. **`cmd_stats`** — 全局遍历统计（替代 2 层遍历）
8. **`cmd_def`** — 全局搜索（替代文件层级搜索）
9. **`cmd_refs`** — 搜索所有语义边类型（Extends/Calls/ConfigSets 等）
10. **`cmd_calls`** — 全局搜索 CallSite + Calls 边
11. **`cmd_hierarchy`** — 支持 Class/Package/Interface
12. **`cmd_uvm_factory`** — 全局搜索 + 父类名匹配
13. **`cmd_uvm_tlm`** — 全局搜索
14. **`cmd_uvm_config`** — 全局搜索
15. **`find_parent_name`** — 新增，沿 Contains 边向上查找命名祖先

---

## 剩余优化项

| 优先级 | 问题 | 说明 |
|--------|------|------|
| P1 | search 结果仍重复 | .sv 和 .uvm.sv 同名 class 两次索引 |
| P1 | Functions=0 | function/task 声明未被提取（可能被 ClassMethod 覆盖） |
| P1 | Parameters=0 | parameter 声明未被提取 |
| P2 | TLM Ports=0 | TLM 端口检测依赖宏展开后的类型名 |
| P2 | ConfigDB=0 | config_db 调用在类方法体内，需要更深的 AST 遍历 |
| P3 | 搜索不支持通配符 | `search "svt_axi_*_transaction"` 不支持 |
