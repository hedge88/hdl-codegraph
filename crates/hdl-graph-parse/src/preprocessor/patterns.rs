//! Central library of UVM macro pattern definitions.
//!
//! In the current implementation the expansion rules live directly inside
//! `pass3_expander` as match arms.  This module provides the type definitions
//! that a future refactored pattern-table could use.

/// A single UVM macro pattern definition.
pub struct MacroPattern {
    pub name: &'static str,
    pub arg_count: std::ops::Range<usize>,
    pub description: &'static str,
    pub expand_fn: fn(&[String]) -> String,
}
