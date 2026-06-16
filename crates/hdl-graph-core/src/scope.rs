use serde::{Deserialize, Serialize};
use crate::node::NodeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeStack {
    pub scopes: Vec<ScopeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeEntry {
    pub scope_id: NodeId,
    pub name: String,
    pub depth: u32,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn push(&mut self, scope_id: NodeId, name: &str) {
        let depth = self.scopes.len() as u32;
        self.scopes.push(ScopeEntry {
            scope_id,
            name: name.to_string(),
            depth,
        });
    }

    pub fn pop(&mut self) -> Option<ScopeEntry> {
        self.scopes.pop()
    }

    pub fn current_scope(&self) -> Option<&ScopeEntry> {
        self.scopes.last()
    }

    pub fn scope_path(&self) -> Vec<String> {
        self.scopes.iter().map(|s| s.name.clone()).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.scopes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.scopes.len()
    }
}
