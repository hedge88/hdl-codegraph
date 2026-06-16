use serde::{Deserialize, Serialize};
use crate::symbol::{InternedString, SymbolTable};

pub type NodeId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub scope_id: Option<NodeId>,
    #[serde(default)]
    pub file_id: u32,
    #[serde(default)]
    pub line: u32,
    #[serde(default)]
    pub col: u32,
}

impl Default for GraphNode {
    fn default() -> Self {
        Self {
            id: 0,
            kind: NodeKind::SourceFile,
            scope_id: None,
            file_id: 0,
            line: 0,
            col: 0,
        }
    }
}

impl GraphNode {
    /// Returns the human-readable name of this node by resolving its InternedString
    /// through the given SymbolTable. Centralizes the `node_name_str` logic that was
    /// previously duplicated across multiple crates.
    pub fn name_str(&self, symbols: &SymbolTable) -> Option<String> {
        node_name_from_kind(&self.kind, symbols)
    }

    /// Returns the InternedString id for this node's name, if it has one.
    /// Useful for index-based lookups without needing a SymbolTable.
    pub fn name_interned_id(&self) -> Option<u64> {
        extract_name_id(&self.kind)
    }
}

/// Resolve the name of a NodeKind through a SymbolTable.
pub fn node_name_from_kind(kind: &NodeKind, symbols: &SymbolTable) -> Option<String> {
    match kind {
        NodeKind::Module { name }
        | NodeKind::Class { name, .. }
        | NodeKind::Package { name }
        | NodeKind::Interface { name }
        | NodeKind::Function { name, .. }
        | NodeKind::SignalDecl { name, .. }
        | NodeKind::ModulePort { name, .. }
        | NodeKind::ModuleInstance { name, .. }
        | NodeKind::Property { name }
        | NodeKind::VariableRef { name }
        | NodeKind::Method { name, .. }
        | NodeKind::TLMPort { name, .. }
        | NodeKind::SequenceDecl { name, .. }
        | NodeKind::PropertyDecl { name, .. }
        | NodeKind::CoverGroup { name, .. }
        | NodeKind::CoverPoint { name, .. }
        | NodeKind::Modport { name }
        | NodeKind::ConfigBlock { name } => symbols.resolve(*name).map(|s| s.to_string()),
        NodeKind::CallSite { target: name }
        | NodeKind::DPIImport { function_name: name }
        | NodeKind::ConfigDBSet { field: name }
        | NodeKind::ConfigDBGet { field: name }
        | NodeKind::Parameter { name } => symbols.resolve(*name).map(|s| s.to_string()),
        NodeKind::FactoryReg { type_name, .. } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryCreate { type_name } => {
            symbols.resolve(*type_name).map(|s| s.to_string())
        }
        NodeKind::FactoryOverride { original_type, .. } => {
            symbols.resolve(*original_type).map(|s| s.to_string())
        }
        _ => None,
    }
}

/// Extract the raw InternedString id for the name field of a NodeKind.
/// Returns None for variants without a meaningful name (SourceFile, Assignment, etc.).
pub fn extract_name_id(kind: &NodeKind) -> Option<u64> {
    match kind {
        NodeKind::Module { name }
        | NodeKind::Class { name, .. }
        | NodeKind::Package { name }
        | NodeKind::Interface { name }
        | NodeKind::Function { name, .. }
        | NodeKind::SignalDecl { name, .. }
        | NodeKind::ModulePort { name, .. }
        | NodeKind::ModuleInstance { name, .. }
        | NodeKind::Property { name }
        | NodeKind::VariableRef { name }
        | NodeKind::Method { name, .. }
        | NodeKind::TLMPort { name, .. }
        | NodeKind::SequenceDecl { name, .. }
        | NodeKind::PropertyDecl { name, .. }
        | NodeKind::CoverGroup { name, .. }
        | NodeKind::CoverPoint { name, .. }
        | NodeKind::Modport { name }
        | NodeKind::ConfigBlock { name } => Some(name.0),
        NodeKind::CallSite { target: name }
        | NodeKind::DPIImport { function_name: name }
        | NodeKind::ConfigDBSet { field: name }
        | NodeKind::ConfigDBGet { field: name }
        | NodeKind::Parameter { name } => Some(name.0),
        NodeKind::FactoryReg { type_name, .. } => Some(type_name.0),
        NodeKind::FactoryCreate { type_name } => Some(type_name.0),
        NodeKind::FactoryOverride { original_type, .. } => Some(original_type.0),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeKind {
    // --- Structural ---
    SourceFile,
    Module { name: InternedString },
    ModulePort { name: InternedString, direction: PortDirection },
    ModuleInstance { name: InternedString, module_type: InternedString },
    PortConnection { port_name: InternedString, actual: InternedString },
    GenerateBlock { kind: GenerateKind },
    AlwaysBlock { kind: AlwaysKind },
    SignalDecl { name: InternedString, kind: SignalKind },
    Assignment,
    Function { name: InternedString, is_task: bool },
    BeginBlock { label: Option<InternedString> },
    VariableRef { name: InternedString },
    CallSite { target: InternedString },

    // --- OOP ---
    Class { name: InternedString, parent: Option<InternedString> },
    Method { name: InternedString, is_virtual: bool },
    Property { name: InternedString },

    // --- Packages & Interfaces ---
    Package { name: InternedString },
    PackageImport { source: InternedString },
    Interface { name: InternedString },
    Modport { name: InternedString },

    // --- UVM ---
    FactoryReg { type_name: InternedString, base_type: InternedString },
    FactoryCreate { type_name: InternedString },
    FactoryOverride { original_type: InternedString, override_type: InternedString },
    TLMPort { name: InternedString, direction: TLMDirection },
    TLMBinding,
    ConfigDBSet { field: InternedString },
    ConfigDBGet { field: InternedString },

    // --- Assertions ---
    AssertProperty,
    SequenceDecl { name: InternedString },
    PropertyDecl { name: InternedString },
    CoverGroup { name: InternedString },
    CoverPoint { name: InternedString },

    // --- Misc ---
    DPIImport { function_name: InternedString },
    BindDirective { module_type: InternedString },
    ConfigBlock { name: InternedString },
    Parameter { name: InternedString },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PortDirection {
    Input,
    Output,
    Inout,
    Ref,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerateKind {
    If,
    Case,
    For,
    Loop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlwaysKind {
    Combinational,
    Sequential,
    Latch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalKind {
    Wire,
    Reg,
    Logic,
    Integer,
    Bit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TLMDirection {
    AnalysisPort,
    AnalysisExport,
    Blocking,
    Nonblocking,
    Fifo,
}
