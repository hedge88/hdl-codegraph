use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InternedString(pub u64);

#[derive(Debug, Default)]
pub struct SymbolTable {
    strings: Vec<String>,
    lookup: HashMap<String, u64>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> InternedString {
        if let Some(&id) = self.lookup.get(s) {
            return InternedString(id);
        }
        let id = self.strings.len() as u64;
        self.strings.push(s.to_string());
        self.lookup.insert(s.to_string(), id);
        InternedString(id)
    }

    pub fn resolve(&self, id: InternedString) -> Option<&str> {
        self.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    pub fn contains(&self, s: &str) -> bool {
        self.lookup.contains_key(s)
    }

    /// Look up the InternedString id for a given string without interning it.
    /// Returns None if the string has not been interned yet.
    pub fn resolve_id(&self, s: &str) -> Option<u64> {
        self.lookup.get(s).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intern_and_resolve() {
        let mut table = SymbolTable::new();
        let id = table.intern("counter");
        assert_eq!(table.len(), 1);
        assert_eq!(table.resolve(id), Some("counter"));
    }

    #[test]
    fn test_intern_dedup() {
        let mut table = SymbolTable::new();
        let a = table.intern("clk");
        let b = table.intern("clk");
        assert_eq!(a, b);
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_contains() {
        let mut table = SymbolTable::new();
        table.intern("data_out");
        assert!(table.contains("data_out"));
        assert!(!table.contains("unknown"));
    }
}
