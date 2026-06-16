use hdl_graph_core::*;
use crate::server::ProjectState;

pub fn run(state: &ProjectState) -> String {
    let mut mods = 0;
    let mut sigs = 0;
    let mut insts = 0;
    let mut ports = 0;
    let mut classes = 0;
    let mut packages = 0;
    let mut interfaces = 0;
    let mut funcs = 0;
    let mut always = 0;
    let mut assigns = 0;
    let mut params = 0;
    let mut properties = 0;
    let mut methods = 0;
    let mut tlm_ports = 0;
    let mut factory_regs = 0;
    let mut factory_creates = 0;
    let mut factory_overrides = 0;
    let mut config_sets = 0;
    let mut config_gets = 0;
    let mut call_sites = 0;
    let mut assertions = 0;
    let mut dpi_imports = 0;

    for node in state.graph.all_nodes() {
        match &node.kind {
            NodeKind::Module { .. } => mods += 1,
            NodeKind::Class { .. } => classes += 1,
            NodeKind::Package { .. } => packages += 1,
            NodeKind::Interface { .. } => interfaces += 1,
            NodeKind::SignalDecl { .. } => sigs += 1,
            NodeKind::ModuleInstance { .. } => insts += 1,
            NodeKind::ModulePort { .. } => ports += 1,
            NodeKind::AlwaysBlock { .. } => always += 1,
            NodeKind::Assignment => assigns += 1,
            NodeKind::Function { .. } => funcs += 1,
            NodeKind::Parameter { .. } => params += 1,
            NodeKind::Property { .. } => properties += 1,
            NodeKind::Method { .. } => methods += 1,
            NodeKind::TLMPort { .. } => tlm_ports += 1,
            NodeKind::FactoryReg { .. } => factory_regs += 1,
            NodeKind::FactoryCreate { .. } => factory_creates += 1,
            NodeKind::FactoryOverride { .. } => factory_overrides += 1,
            NodeKind::ConfigDBSet { .. } => config_sets += 1,
            NodeKind::ConfigDBGet { .. } => config_gets += 1,
            NodeKind::CallSite { .. } => call_sites += 1,
            NodeKind::AssertProperty
            | NodeKind::SequenceDecl { .. }
            | NodeKind::PropertyDecl { .. }
            | NodeKind::CoverGroup { .. }
            | NodeKind::CoverPoint { .. } => assertions += 1,
            NodeKind::DPIImport { .. } => dpi_imports += 1,
            _ => {}
        }
    }

    format!(
        "Graph Statistics:\n\
         \x20 Files:       {}\n\
         \x20 Nodes:       {}\n\
         \x20 Edges:       {}\n\
         \x20 --- Structural ---\n\
         \x20 Modules:     {}\n\
         \x20 Ports:       {}\n\
         \x20 Signals:     {}\n\
         \x20 Instances:   {}\n\
         \x20 Always:      {}\n\
         \x20 Assigns:     {}\n\
         \x20 Parameters:  {}\n\
         \x20 Functions:   {}\n\
         \x20 --- OOP ---\n\
         \x20 Classes:     {}\n\
         \x20 Properties:  {}\n\
         \x20 Methods:     {}\n\
         \x20 --- Packages & Interfaces ---\n\
         \x20 Packages:    {}\n\
         \x20 Interfaces:  {}\n\
         \x20 --- UVM ---\n\
         \x20 TLM Ports:   {}\n\
         \x20 Factory Reg: {}\n\
         \x20 Factory New: {}\n\
         \x20 Factory Ovr: {}\n\
         \x20 ConfigDB Set:{}\n\
         \x20 ConfigDB Get:{}\n\
         \x20 --- Misc ---\n\
         \x20 Call Sites:  {}\n\
         \x20 Assertions:  {}\n\
         \x20 DPI Imports: {}",
        state.file_map.len(),
        state.graph.node_count(),
        state.graph.edge_count(),
        mods, ports, sigs, insts, always, assigns, params, funcs,
        classes, properties, methods,
        packages, interfaces,
        tlm_ports, factory_regs, factory_creates, factory_overrides,
        config_sets, config_gets,
        call_sites, assertions, dpi_imports,
    )
}
