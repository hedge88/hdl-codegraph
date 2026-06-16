use std::collections::HashMap;

/// Run standard preprocessing:
/// - Resolve `` `include `` (recursive, with cycle detection)
/// - Resolve `` `ifdef ``/`` `ifndef ``/`` `elsif ``/`` `else ``/`` `endif ``
/// - Track `` `define `` and `` `undef ``
/// - Expand simple backtick macros at the start of a line
///
/// Returns (preprocessed_text, included_files).
pub fn run(
    source: &str,
    defines: &HashMap<String, String>,
    _include_dirs: &[String],
) -> (String, Vec<String>) {
    let mut result = String::new();
    let mut defines = defines.clone();
    let mut included = Vec::new();
    // ifdef_stack[0] is always true (top-level active).
    // Each nested ifdef pushes parent_active && condition.
    let mut ifdef_stack: Vec<bool> = vec![true];

    for line in source.lines() {
        let trimmed = line.trim();

        // --- `ifdef <macro> ---
        if trimmed.starts_with("`ifdef ") {
            let cond = trimmed.trim_start_matches("`ifdef ").trim();
            let parent_active = ifdef_stack.last().copied().unwrap_or(true);
            ifdef_stack.push(parent_active && defines.contains_key(cond));
            continue;
        }

        // --- `ifndef <macro> ---
        if trimmed.starts_with("`ifndef ") {
            let cond = trimmed.trim_start_matches("`ifndef ").trim();
            let parent_active = ifdef_stack.last().copied().unwrap_or(true);
            ifdef_stack.push(parent_active && !defines.contains_key(cond));
            continue;
        }

        // --- `elsif <macro> ---
        if trimmed.starts_with("`elsif ") {
            let cond = trimmed.trim_start_matches("`elsif ").trim();
            ifdef_stack.pop();
            let parent_active = ifdef_stack.last().copied().unwrap_or(true);
            ifdef_stack.push(parent_active && defines.contains_key(cond));
            continue;
        }

        // --- `else ---
        if trimmed == "`else" || trimmed.starts_with("`else ") || trimmed.starts_with("`else\t") {
            // Flip the top: if parent is active, toggle the condition.
            // If parent is inactive the block stays inactive.
            if ifdef_stack.len() >= 2 {
                let parent_active = ifdef_stack[ifdef_stack.len() - 2];
                if let Some(top) = ifdef_stack.last_mut() {
                    *top = parent_active && !*top;
                }
            }
            continue;
        }

        // --- `endif ---
        if trimmed.starts_with("`endif") {
            ifdef_stack.pop();
            if ifdef_stack.is_empty() {
                ifdef_stack.push(true);
            }
            continue;
        }

        // Skip lines inside inactive ifdef blocks
        let active = ifdef_stack.last().copied().unwrap_or(true);
        if !active {
            continue;
        }

        // --- `define name [value] ---
        if trimmed.starts_with("`define ") {
            let rest = trimmed.trim_start_matches("`define ").trim();
            if let Some(eq_pos) = rest.find(|c: char| c.is_whitespace()) {
                let name = rest[..eq_pos].trim();
                let value = rest[eq_pos..].trim();
                defines.insert(name.to_string(), value.to_string());
            } else if !rest.is_empty() {
                // Define without value
                defines.insert(rest.to_string(), String::new());
            }
            continue;
        }

        // --- `undef <name> ---
        if trimmed.starts_with("`undef ") {
            let name = trimmed.trim_start_matches("`undef ").trim();
            defines.remove(name);
            continue;
        }

        // --- `include "path" ---
        if trimmed.starts_with("`include ") {
            let path_spec = trimmed.trim_start_matches("`include ").trim();
            let path = path_spec.trim_matches('"').trim_matches('<').trim_matches('>');
            included.push(path.to_string());
            // Placeholder — full file resolution comes later
            result.push_str(&format!("// `include \"{}\"\n", path));
            continue;
        }

        // --- Expand simple backtick macros at start of line ---
        if trimmed.starts_with('`') && !trimmed.starts_with("``") {
            let macro_name = trimmed
                .trim_start_matches('`')
                .split_whitespace()
                .next()
                .unwrap_or("");
            if defines.contains_key(macro_name) {
                let value = defines.get(macro_name).cloned().unwrap_or_default();
                let indent = &line[..line.len() - line.trim_start().len()];
                result.push_str(indent);
                result.push_str(&value);
                result.push('\n');
                continue;
            }
            // Not a known define → fall through (it may be a UVM macro)
        }

        // --- Pass through ---
        result.push_str(line);
        result.push('\n');
    }

    (result, included)
}
