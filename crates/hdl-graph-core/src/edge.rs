use serde::{Deserialize, Serialize};
use crate::node::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeType {
    Contains = 0,
    Defines = 1,
    References = 2,
    Imports = 3,
    Extends = 4,
    Instantiates = 5,
    Connects = 6,
    Drives = 7,
    Triggers = 8,
    Calls = 9,
    Overrides = 10,
    MacroExpands = 11,
    FactoryRegisters = 12,
    FactoryOverrides = 13,
    TLMBinds = 14,
    ConfigSets = 15,
    ConfigGets = 16,
    ConfigResolves = 17,
}

impl EdgeType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Defines => "defines",
            Self::References => "references",
            Self::Imports => "imports",
            Self::Extends => "extends",
            Self::Instantiates => "instantiates",
            Self::Connects => "connects",
            Self::Drives => "drives",
            Self::Triggers => "triggers",
            Self::Calls => "calls",
            Self::Overrides => "overrides",
            Self::MacroExpands => "macro_expands",
            Self::FactoryRegisters => "factory_registers",
            Self::FactoryOverrides => "factory_overrides",
            Self::TLMBinds => "tlm_binds",
            Self::ConfigSets => "config_sets",
            Self::ConfigGets => "config_gets",
            Self::ConfigResolves => "config_resolves",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
    pub edge_type: EdgeType,
}
