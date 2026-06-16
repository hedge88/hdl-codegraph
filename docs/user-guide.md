# HDL Code Graph — User Guide

## Getting Started

```bash
# Initialize a project
hdl-graph init my_project

# Index SV files
cd my_project
hdl-graph index

# See project statistics
hdl-graph stats
```

## Project Configuration

Configuration lives in `.hdl-graph/config.toml`:

```toml
[project]
name = "my_soc"
root = "."

[index]
include_dirs = ["rtl", "tb", "sim"]
uvm_home = "/tools/uvm-1.2"
defines = ["UVM_NO_DPI"]
jobs = 8
```

## Query Commands

### `hdl-graph query def <symbol> [scope]`
Find the definition of a symbol (module, signal, port, class, function).

```bash
hdl-graph query def clk
hdl-graph query def apb_ready top.sv:245
```

### `hdl-graph query refs <symbol> [scope]`
Find all references to a symbol.

```bash
hdl-graph query refs apb_driver
```

### `hdl-graph query hierarchy <name>`
Show the module hierarchy tree.

```bash
$ hdl-graph query hierarchy top
top
├── clk        (port)
├── rst_n      (port)
├── u_apb      (instance: apb_slave)
│   ├── clk    (port)
│   ├── addr   (port)
│   ├── psel   (port)
│   └── pready (port)
├── u_uart     (instance: uart)
│   ├── tx     (port)
│   └── rx     (port)
├── internal   (signal)
└── always
```

### `hdl-graph query inst <module_type>`
Find all instantiations of a module type.

```bash
$ hdl-graph query inst fifo
  u_fifo_0 (in module top)
  u_fifo_1 (in module memory_subsystem)
```

### `hdl-graph query drivers <signal>`
Find all drivers and readers of a signal.

## UVM Commands

### `hdl-graph uvm factory <type_name>`
Show factory registrations and overrides.

```bash
$ hdl-graph uvm factory my_driver
  Registration: my_driver extends uvm_component
  Create: type_id::create("m_driver")
  Override: my_driver → my_other_driver
```

### `hdl-graph uvm tlm <component>`
Show TLM port connections.

```bash
$ hdl-graph uvm tlm env
  Port: ap (analysis_port)
    → connected to: analysis_export
  Port: put_port (blocking_put_port)
```

### `hdl-graph uvm config <path>`
Show config DB set/get operations.

```bash
$ hdl-graph uvm config "*.driver.vif"
  SET   "axi_vif"  (in module test)
  GET   "vif"      (in module my_driver)
```

### `hdl-graph uvm hierarchy`
Show UVM type hierarchy (class extends tree).

## Search

```bash
# Search by pattern
hdl-graph search "apb_*"
hdl-graph search "fifo"
```

## LSP Server

```bash
# Start on stdio (for VS Code/Neovim)
hdl-graph watch

# Or as a background daemon
hdl-graph daemon start
```

## SCIP Export

```bash
hdl-graph export scip ./hdl-graph.scip
```

The SCIP export is compatible with [Sourcegraph](https://sourcegraph.com) and [GitHub Code Search](https://cs.github.com).

## Output Formats

```bash
hdl-graph stats --format json
hdl-graph query hierarchy top --format json
```
