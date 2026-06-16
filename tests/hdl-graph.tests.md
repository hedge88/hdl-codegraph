# hdl-graph 工具质量评估报告

> 生成日期: 2026-06-11
> 测试集版本: 99 tests, 全部通过
> 外部基准: sv-tests (chipsalliance), darkriscv, uart, ibex, axi

---

## 一、测试集总览

| 模块 | 测试数 | 状态 |
|------|--------|------|
| verilog_basic_tests | 7 | ✅ 7/7 |
| sv_oop_tests | 6 | ✅ 6/6 |
| sv_advanced_tests | 7 | ✅ 7/7 |
| uvm_extraction_tests | 10 | ✅ 10/10 |
| uvm_preprocessor_tests | 6 | ✅ 6/6 |
| uvm_macro_fixture_tests | 5 | ✅ 5/5 |
| query_tools_tests | 8 | ✅ 8/8 |
| export_tests | 4 | ✅ 4/4 |
| edge_case_tests | 5 | ✅ 5/5 |
| additional_edge_case_tests | 12 | ✅ 12/12 |
| incremental_tests | 5 | ✅ 5/5 |
| scanner_tests | 6 | ✅ 6/6 |
| external_project_tests | 18 | ✅ 18/18 |
| **总计** | **99** | **✅ 99/99** |

### 测试模块明细

#### 1. verilog_basic_tests (7)
- test_module_extraction — 模块、端口、信号、always、assign、Contains/Drives/References 边
- test_module_hierarchy — 4个fixture文件各自产生一个模块
- test_instantiation_edges — top.v 中 u_counter/u_adder 实例化
- test_port_connections — 实例化端口连接结构
- test_parameter_extraction — params.v 端口/信号/always/assign
- test_signal_flow — counter.v 的 Drives/References 边
- test_multi_file_index — 4文件索引

#### 2. sv_oop_tests (6)
- test_class_extraction — base_driver 类、构造函数、属性、Defines/Contains 边
- test_class_extends — my_driver extends base_driver
- test_class_extends_cross_file — sv_oop 目录跨文件索引
- test_virtual_methods — virtual/non-virtual 方法区分
- test_package_and_import — my_pkg/another_pkg 包提取
- test_interface_modport — my_bus_if 接口、master/slave modport

#### 3. sv_advanced_tests (7)
- test_generate_for — for-generate 块
- test_generate_if_case — if-generate / case-generate
- test_assertion_property — AssertProperty / PropertyDecl / SequenceDecl
- test_covergroup — CoverGroup / CoverPoint
- test_dpi_import — DPI-C 函数导入
- test_bind_directive — config block 烟雾测试
- test_always_kinds — always_comb / always_ff 检测

#### 4. uvm_extraction_tests (10)
- test_factory_registration — uvm_component_utils → FactoryReg
- test_factory_create — type_id::create → FactoryCreate
- test_factory_override_type — set_type_override → FactoryOverride
- test_factory_override_inst — set_inst_override → FactoryOverride
- test_tlm_analysis_port — uvm_analysis_port → TLMPort(AnalysisPort)
- test_tlm_connect — agent.mon.ap.connect → 类/方法提取
- test_config_db_set — uvm_config_db::set → ConfigDBSet
- test_config_db_get — uvm_config_db::get → ConfigDBGet
- test_uvm_class_hierarchy — 预处理后索引 uvm_components，≥5个类
- test_full_uvm_env_index — 完整 UVM 环境索引

#### 5. uvm_preprocessor_tests (6)
- test_component_utils_expansion — uvm_component_utils → uvm_component_registry
- test_object_utils_expansion — uvm_object_utils → uvm_object_registry
- test_field_macros_expand — uvm_field_int 展开
- test_info_macros_expand — uvm_info/error/warning 展开
- test_do_macros_expand — uvm_do/uvm_do_with → start_item
- test_macros_then_extract — 预处理 → 解析 → 提取完整流水线

#### 6. uvm_macro_fixture_tests (5)
- test_macro_utils_fixture_extraction — macro_utils.sv 完整流水线
- test_macro_fields_fixture_extraction — uvm_field 宏预处理+容错解析
- test_macro_info_fixture_extraction — uvm_info/warning/error 展开验证
- test_macro_do_fixture_extraction — uvm_do/create/send 展开验证
- test_uvm_macro_fixtures_index — 4个宏fixture全量索引

#### 7. query_tools_tests (8)
- test_search_glob — 通配符搜索
- test_search_case_insensitive — 大小写不敏感搜索
- test_hierarchy_tree — 模块层次树构建
- test_callers_of_signal — 信号引用者查找
- test_drivers_of_signal — 信号驱动者查找
- test_stats_counts — 节点类型统计
- test_explore_detail — counter 模块子节点遍历
- test_impact_analysis — BFS 影响分析

#### 8. export_tests (4)
- test_json_export_roundtrip — JSON 导出 → 解析 → 验证结构
- test_json_export_schema — JSON schema 验证
- test_markdown_export_single — 单文件 Markdown 导出
- test_markdown_export_per_module — 按模块分文件 Markdown 导出

#### 9. edge_case_tests (5)
- test_empty_module — 空模块最小图结构
- test_nonblocking_tlm_detection — uvm_nonblocking_* TLM 端口方向检测
- test_nested_generate_deep — 三层嵌套 generate
- test_multi_file_cross_ref — multi_file_top/other 跨文件实例化
- test_ifdef_preprocessing — ifdef/ifndef 预处理分支解析

#### 10. additional_edge_case_tests (12)
- test_ifdef_full_extraction_pipeline — ifdef_macros.sv 预处理→提取完整流水线
- test_empty_module_structure — 空模块：2节点+1边
- test_single_port_module — 单端口模块提取
- test_output_port_direction — input/output/inout 端口名提取
- test_signal_kinds_wire_reg_logic — wire/reg/logic 信号类型检测
- test_multiple_modules_one_file — 单文件多模块
- test_always_block_kinds_all_three — always_comb/ff/latch 三种全部检测
- test_function_and_task_extraction — function(is_task=false) + task(is_task=true)
- test_nonblocking_assignment_detection — 非阻塞赋值的 Drives/References 边
- test_begin_block_with_label — 命名 begin-end 块
- test_edge_case_all_parse_errors_skipped — 解析失败文件优雅跳过
- test_nested_generate_contains_hierarchy — 嵌套 generate 的 Contains 边层次

#### 11. incremental_tests (5)
- test_changeset_same_source_empty — 相同源码无变更
- test_changeset_add_signal — 新增信号产生 added_nodes
- test_changeset_apply_to_graph — 空 changeset 不改变图
- test_changeset_apply_adds_nodes — apply_to 添加节点和边
- test_changeset_has_changes — changeset 结构验证

#### 12. scanner_tests (6)
- test_file_scanner_parse_source — parse_source 字符串解析
- test_file_scanner_parse_file — parse_file 文件解析
- test_scanner_incremental_no_old_tree — 增量解析(None) ≈ 全量解析
- test_scanner_incremental_with_old_tree — 增量解析不崩溃
- test_scanner_with_include_dirs — include_dirs 构造
- test_file_scanner_parse_all_fixtures — 所有 fixture 文件可解析

#### 13. external_project_tests (18)
- Tier 1 冒烟: test_tier1_darkriscv_smoke, test_tier1_uart_smoke
- Tier 2 sv-tests: chapter 5/6/7/8/9/10/16/18/23 + uvm + all_chapters_summary
- Tier 3 真实 RTL: test_tier3_ibex_rtl, test_tier3_axi_src, test_tier3_axi_test_classes
- Tier 4: test_tier4_full_project_no_crash
- 跨层: test_cross_tier_all_projects_no_panic

---

## 二、外部项目解析率

### sv-tests（chipsalliance 官方合规性基准，830 个 SV 文件）

| 章节 | 主题 | 解析率 | 分析 |
|------|------|--------|------|
| ch5 | Data Types | 90% (45/50) | 5个失败 — packed union、chandle 等高级类型 |
| ch6 | Expressions | 99% (83/84) | 1个失败 |
| ch7 | Assignments | **100%** (103/103) | |
| ch8 | Tasks/Functions | **100%** (53/53) | |
| ch9 | Threads | **100%** (46/46) | |
| ch10 | Assertions | **100%** (10/10) | |
| ch11 | Clocking | 99% (87/88) | 1个失败 |
| ch12 | Random | **100%** (27/27) | |
| ch13 | Coverage | **100%** (15/15) | |
| ch14 | Scheduling | **100%** (5/5) | |
| ch15 | Specify | **100%** (5/5) | |
| ch16 | Interfaces | **100%** (52/52) | |
| ch18 | Packages | **100%** (134/134) | |
| ch20 | Sequences | **100%** (47/47) | |
| ch21 | Properties | **100%** (29/29) | |
| ch22 | DPI/Bind/Pragma | 65% (49/75) | 纯预处理器测试文件（pragma/line/celldefine）不含可解析 SV 结构 |
| ch23 | Generate | **100%** (3/3) | |
| ch24-26 | Misc | **100%** (4/4) | |
| **总计** | | **96% (797/830)** | 14/20 章节 100% |

### 真实开源项目

| 项目 | 文件数 | 解析率 | 节点 | 边 | 说明 |
|------|--------|--------|------|-----|------|
| darkriscv (RV32I CPU) | 15 | **87%** (13/15) | 834 | 1147 | 2个文件含外部 include 未解析 |
| uart (UART TX/RX) | 5 | **100%** (5/5) | 161 | 214 | 完美 |
| ibex (RV32IMC CPU) | 30 | **67%** (20/30) | 2909 | 2959 | 10个文件依赖 OpenTitan prim 库的 include |
| axi/src (AXI IP) | 64 | **98%** (63/64) | 5929 | 5963 | 1个文件失败 |
| axi/test (验证类) | 26 | **100%** (26/26) | 2279 | 2317 | 完美 |

### 外部项目来源

| 项目 | 仓库 | 用途 |
|------|------|------|
| darkriscv | https://github.com/darklife/darkriscv | Tier 1 冒烟 — 极简 RV32I CPU |
| uart | https://github.com/jamieiles/uart | Tier 1 冒烟 — UART TX/RX |
| sv-tests | https://github.com/chipsalliance/sv-tests | Tier 2 — LRM 合规性基准 |
| ibex | https://github.com/lowRISC/ibex | Tier 3 — 真实 SV RTL |
| axi | https://github.com/pulp-platform/axi | Tier 3+4 — 接口/类/包 |

---

## 三、提取能力矩阵

| SV 构造 | 提取质量 | 备注 |
|---------|----------|------|
| Module | ⭐⭐⭐⭐⭐ | 名称、层次、实例化全支持 |
| Port (input/output/inout) | ⭐⭐⭐ | 名称正确，方向检测有 fallback 到 Inout 的问题 |
| Signal (wire/reg/logic) | ⭐⭐⭐⭐ | wire 准确，reg 有时误判为 logic |
| Always (comb/ff/latch) | ⭐⭐⭐⭐⭐ | 三种类型精确区分 |
| Assign (continuous/procedural) | ⭐⭐⭐⭐⭐ | Drives/References 边正确 |
| Function/Task | ⭐⭐⭐⭐⭐ | is_task 标志、Calls 边 |
| Generate (for/if/case) | ⭐⭐⭐⭐⭐ | 三种类型全支持 |
| Class/Method/Property | ⭐⭐⭐⭐⭐ | extends、virtual、OOP 完整 |
| Package/Import | ⭐⭐⭐⭐ | 包提取准确，import 有时不提取 |
| Interface/Modport | ⭐⭐⭐⭐⭐ | 名称、modport 全支持 |
| Parameter | ⭐⭐⭐ | `#()` 参数列表提取，localparam 支持 |
| Assertions (assert/assume/cover) | ⭐⭐⭐⭐ | PropertyDecl/SequenceDecl/AssertProperty |
| CoverGroup | ⭐⭐⭐ | CoverGroup 能提取，CoverPoint 不稳定 |
| DPI Import | ⭐⭐⭐⭐ | 函数名提取准确 |
| Factory (reg/create/override) | ⭐⭐⭐⭐⭐ | **差异化优势**，UVM factory 完整 |
| ConfigDB (set/get) | ⭐⭐⭐⭐⭐ | **差异化优势**，字段名准确 |
| TLM Port | ⭐⭐⭐⭐ | AnalysisPort/Blocking/Nonblocking |
| `define/`ifdef | ⭐⭐⭐⭐ | 新增预处理器，真实项目 87-98% |
| `include | ⭐⭐ | 能解析路径但依赖外部文件存在 |

---

## 四、已知弱点与根因

| 弱点 | 影响范围 | 根因 |
|------|----------|------|
| 端口方向检测不准 | 单端口模块默认 Inout | ANSI port_declaration 的 port_direction 节点在 tree-sitter CST 中位置不固定 |
| reg 信号类型误判 | 某些 reg 声明被识别为 Logic | detect_signal_kind 对 reg 关键字的匹配路径有歧义 |
| InternedString 跨文件合并 | 真实项目索引时符号冲突 | 每个文件独立 extractor，合并时 remap 依赖名称匹配 |
| include 文件依赖外部库 | ibex 67% | prim_assert.sv 来自 OpenTitan prim 库，不在 ibex 仓库中 |
| CoverPoint 不稳定 | 覆盖率模型 | extract_covergroup_declaration 不遍历内部 coverpoint |
| ch22 解析率 65% | sv-tests | 纯预处理器测试文件（pragma/line/celldefine）不含可解析 SV 结构 |

---

## 五、质量结论

| 维度 | 评分 | 依据 |
|------|------|------|
| 基本解析 | ⭐⭐⭐⭐ | sv-tests 96%，14/20 章节 100% |
| UVM 支持 | ⭐⭐⭐⭐⭐ | factory/config_db/TLM 是其他开源工具没有的 |
| 鲁棒性 | ⭐⭐⭐⭐⭐ | 11000+ 节点真实项目零崩溃 |
| 预处理器 | ⭐⭐⭐⭐ | `define/`ifdef/`include 支持，真实项目 87-98% |
| 图正确性 | ⭐⭐⭐ | 能提取，但端口方向/信号类型有误判 |
| 跨文件解析 | ⭐⭐ | InternedString remap 依赖名称匹配 |

**最大价值场景:** UVM 验证代码的快速结构分析（factory 注册、config_db 依赖、TLM 连接拓扑）

**最大短板:** 跨文件符号解析和预处理器 include 路径解析

---

## 六、测试集文件清单

```
tests/
├── hdl-graph.tests.md              ← 本报告
├── Cargo.toml
├── fixtures/                       ← 手写测试 fixtures
│   ├── verilog_basic/              (counter.v, adder.v, top.v, params.v)
│   ├── sv_oop/                     (base_class.sv, derived_class.sv, package_defs.sv, interface_modport.sv)
│   ├── sv_advanced/                (generate_blocks.sv, assertions.sv, dpi_bind.sv, always_comb_ff_latch.sv)
│   ├── uvm_components/             (my_transaction.sv, sequences.sv, my_driver.sv, my_monitor.sv, ...)
│   ├── uvm_macros/                 (macro_utils.sv, macro_fields.sv, macro_info.sv, macro_do.sv)
│   └── edge_cases/                 (empty_module.v, nonblocking_tlm.sv, nested_generate.sv, ...)
├── external_fixtures/              ← 外部开源项目 (git clone)
│   ├── darkriscv/
│   ├── uart/
│   ├── sv-tests/
│   ├── ibex/
│   └── axi/
└── src/
    ├── lib.rs
    └── integration/
        ├── mod.rs
        ├── common/mod.rs                    ← 共享工具函数
        ├── verilog_basic_tests.rs           ← 7 tests
        ├── sv_oop_tests.rs                  ← 6 tests
        ├── sv_advanced_tests.rs             ← 7 tests
        ├── uvm_extraction_tests.rs          ← 10 tests
        ├── uvm_preprocessor_tests.rs        ← 6 tests
        ├── uvm_macro_fixture_tests.rs       ← 5 tests
        ├── query_tools_tests.rs             ← 8 tests
        ├── export_tests.rs                  ← 4 tests
        ├── edge_case_tests.rs               ← 5 tests
        ├── additional_edge_case_tests.rs    ← 12 tests
        ├── incremental_tests.rs             ← 5 tests
        ├── scanner_tests.rs                 ← 6 tests
        └── external_project_tests.rs        ← 18 tests
```
