/// Standalone SV preprocessor for external project indexing.
///
/// Handles `define, `ifdef/`ifndef/`elsif/`else/`endif, `include,
/// and inline macro expansion. Designed to be used by the external
/// project test harness so that real-world SV files can be parsed
/// by tree-sitter.
///
/// This is separate from the UVM-focused 4-pass pipeline in mod.rs.
/// It reads files from the filesystem for `include resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A macro definition, optionally with formal parameters.
#[derive(Debug, Clone)]
pub struct MacroDef {
    /// Formal parameter names (None for object-like macros).
    pub params: Option<Vec<String>>,
    /// Default values for parameters, keyed by param name.
    pub defaults: HashMap<String, String>,
    /// The macro body text.
    pub body: String,
}

impl MacroDef {
    /// Create a simple object-like macro (no parameters).
    fn object(body: String) -> Self {
        MacroDef { params: None, defaults: HashMap::new(), body }
    }

    /// Create a function-like macro with parameters.
    fn function(params: Vec<String>, defaults: HashMap<String, String>, body: String) -> Self {
        MacroDef { params: Some(params), defaults, body }
    }
}

/// Preprocess a single SV file with full `define/`ifdef/`include support.
///
/// `file_path` is the absolute path to the file being preprocessed.
/// `defines` is the initial define table (shared across files).
/// `include_dirs` are directories to search for `include files.
/// `visited` tracks include cycles.
pub fn preprocess_sv_file(
    file_path: &Path,
    defines: &mut HashMap<String, MacroDef>,
    include_dirs: &[PathBuf],
    visited: &mut Vec<PathBuf>,
) -> String {
    // Cycle detection
    let canonical = file_path.canonicalize().unwrap_or_else(|_| file_path.to_path_buf());
    if visited.contains(&canonical) {
        return format!("// `include cycle detected: {}\n", file_path.display());
    }
    visited.push(canonical);

    let source = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(_) => {
            visited.pop();
            return String::new();
        }
    };

    let result = preprocess_sv_source(&source, defines, include_dirs, file_path, visited);
    visited.pop();
    result
}

/// Preprocess SV source text (the core engine).
pub fn preprocess_sv_source(
    source: &str,
    defines: &mut HashMap<String, MacroDef>,
    include_dirs: &[PathBuf],
    current_file: &Path,
    visited: &mut Vec<PathBuf>,
) -> String {
    let mut result = String::with_capacity(source.len());
    // ifdef_stack: each entry is (parent_active, this_block_active, has_matched)
    // has_matched: true once any branch in an if/elsif/else chain has been taken
    let mut ifdef_stack: Vec<IfdefState> = vec![IfdefState {
        parent_active: true,
        this_active: true,
        has_matched: true,
    }];

    for line in source.lines() {
        let trimmed = line.trim();

        // Check if we're in an active block
        let active = ifdef_stack.last().map(|s| s.parent_active && s.this_active).unwrap_or(true);

        // --- Preprocessor directives (only processed when active or structural) ---
        if let Some(rest) = strip_directive(trimmed, "ifdef") {
            let cond = rest.trim();
            let parent = ifdef_stack.last().map(|s| s.parent_active && s.this_active).unwrap_or(true);
            let cond_active = defines.contains_key(cond);
            ifdef_stack.push(IfdefState {
                parent_active: parent,
                this_active: cond_active,
                has_matched: cond_active,
            });
            continue;
        }

        if let Some(rest) = strip_directive(trimmed, "ifndef") {
            let cond = rest.trim();
            let parent = ifdef_stack.last().map(|s| s.parent_active && s.this_active).unwrap_or(true);
            let cond_active = !defines.contains_key(cond);
            ifdef_stack.push(IfdefState {
                parent_active: parent,
                this_active: cond_active,
                has_matched: cond_active,
            });
            continue;
        }

        if let Some(rest) = strip_directive(trimmed, "elsif") {
            let cond = rest.trim();
            if let Some(top) = ifdef_stack.last_mut() {
                if !top.has_matched {
                    let cond_active = defines.contains_key(cond);
                    top.this_active = cond_active;
                    top.has_matched = cond_active;
                } else {
                    top.this_active = false;
                }
            }
            continue;
        }

        if strip_directive(trimmed, "else").is_some() || trimmed == "`else" {
            if let Some(top) = ifdef_stack.last_mut() {
                if !top.has_matched {
                    top.this_active = true;
                    top.has_matched = true;
                } else {
                    top.this_active = false;
                }
            }
            continue;
        }

        if strip_directive(trimmed, "endif").is_some() || trimmed == "`endif" {
            if ifdef_stack.len() > 1 {
                ifdef_stack.pop();
            }
            continue;
        }

        // Skip lines in inactive blocks
        if !active {
            continue;
        }

        // --- `define name[(params)] [value] ---
        if let Some(rest) = strip_directive(trimmed, "define") {
            let rest = rest.trim();
            // Parse: name, optional (params), then body
            // Find the end of the name (could be followed by `(` or whitespace)
            let name_end = rest.find(|c: char| c.is_whitespace() || c == '(').unwrap_or(rest.len());
            let name = rest[..name_end].trim();
            let after_name = rest[name_end..].trim_start();

            if name.is_empty() {
                result.push_str(&format!("// {}\n", line));
                continue;
            }

            if after_name.starts_with('(') {
                // Function-like macro: `define NAME(params) body
                let params_and_body = &after_name[1..]; // skip '('
                if let Some(paren_close) = find_matching_paren(params_and_body) {
                    let params_str = params_and_body[..paren_close].trim();
                    let body = params_and_body[paren_close + 1..].trim();

                    let (param_names, defaults) = parse_macro_params(params_str);
                    defines.insert(name.to_string(), MacroDef::function(param_names, defaults, body.to_string()));
                } else {
                    // Malformed — treat as object-like
                    defines.insert(name.to_string(), MacroDef::object(after_name.to_string()));
                }
            } else {
                // Object-like macro: `define NAME body
                let value = after_name;
                defines.insert(name.to_string(), MacroDef::object(value.to_string()));
            }
            // Comment out the define line
            result.push_str(&format!("// {}\n", line));
            continue;
        }

        // --- `undef name ---
        if let Some(rest) = strip_directive(trimmed, "undef") {
            let name = rest.trim();
            defines.remove(name);
            result.push_str(&format!("// {}\n", line));
            continue;
        }

        // --- `include "path" ---
        if let Some(rest) = strip_directive(trimmed, "include") {
            let path_spec = rest.trim();
            let inc_path = path_spec
                .trim_start_matches('"')
                .trim_end_matches('"')
                .trim_start_matches('<')
                .trim_end_matches('>')
                .trim();

            // Try to resolve the include path
            let resolved = resolve_include(inc_path, current_file, include_dirs);
            if let Some(resolved_path) = resolved {
                let inlined = preprocess_sv_file(&resolved_path, defines, include_dirs, visited);
                result.push_str(&inlined);
            } else {
                // Can't resolve — emit a comment and continue
                result.push_str(&format!("// `include \"{}\" // not resolved\n", inc_path));
            }
            continue;
        }

        // --- Pass-through directives (no expansion needed) ---
        // These directives should be emitted as-is (or commented out) without
        // inline macro expansion.
        if strip_directive(trimmed, "pragma").is_some()
            || strip_directive(trimmed, "celldefine").is_some()
            || strip_directive(trimmed, "endcelldefine").is_some()
            || strip_directive(trimmed, "default_nettype").is_some()
            || strip_directive(trimmed, "undefineall").is_some()
            || strip_directive(trimmed, "resetall").is_some()
            || strip_directive(trimmed, "timescale").is_some()
            || strip_directive(trimmed, "begin_keywords").is_some()
            || strip_directive(trimmed, "end_keywords").is_some()
            || strip_directive(trimmed, "line").is_some()
        {
            // Emit as comment so tree-sitter doesn't choke
            result.push_str(&format!("// {}\n", line));
            continue;
        }

        // --- Inline macro expansion ---
        // Expand `MACRO_NAME and `MACRO_NAME(args) anywhere in the line.
        let expanded = expand_inline_macros(line, defines);
        result.push_str(&expanded);
        result.push('\n');
    }

    result
}

struct IfdefState {
    parent_active: bool,
    this_active: bool,
    has_matched: bool,
}

/// Strip a backtick directive prefix, returning the rest of the line.
fn strip_directive<'a>(line: &'a str, directive: &str) -> Option<&'a str> {
    let prefix = format!("`{}", directive);
    if line.starts_with(&prefix) {
        let rest = &line[prefix.len()..];
        // Must be followed by whitespace, end of line, or nothing
        if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace()) {
            return Some(rest);
        }
    }
    None
}

/// Resolve an include path against the current file and include dirs.
fn resolve_include(inc_path: &str, current_file: &Path, include_dirs: &[PathBuf]) -> Option<PathBuf> {
    // 1. Try relative to current file
    if let Some(parent) = current_file.parent() {
        let candidate = parent.join(inc_path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 2. Try each include directory
    for dir in include_dirs {
        let candidate = dir.join(inc_path);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Expand inline backtick macros in a line.
/// Handles: `NAME, `NAME(args)
/// Does NOT expand directives (ifdef/define/etc.) — those are handled above.
fn expand_inline_macros(line: &str, defines: &HashMap<String, MacroDef>) -> String {
    let mut result = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '`' && i + 1 < chars.len() && chars[i + 1].is_ascii_alphabetic() {
            // Read the macro name
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let macro_name: String = chars[start..end].iter().collect();

            // Skip directives — they should not be expanded inline
            if is_directive(&macro_name) {
                result.push(chars[i]);
                i += 1;
                continue;
            }

            if let Some(macro_def) = defines.get(&macro_name) {
                // Check if this is a function-like macro: `NAME(...)
                let mut arg_end = end;
                if end < chars.len() && chars[end] == '(' {
                    // Find matching closing paren
                    let mut depth = 1;
                    arg_end = end + 1;
                    while arg_end < chars.len() && depth > 0 {
                        match chars[arg_end] {
                            '(' => depth += 1,
                            ')' => depth -= 1,
                            _ => {}
                        }
                        arg_end += 1;
                    }

                    // Extract the arguments text (between the parens)
                    let args_text: String = chars[end + 1..arg_end - 1].iter().collect();

                    if let Some(ref formal_params) = macro_def.params {
                        // Function-like macro: parse actual args and substitute
                        let actual_args = parse_macro_args(&args_text);
                        let expanded = substitute_macro_body(
                            &macro_def.body,
                            formal_params,
                            &macro_def.defaults,
                            &actual_args,
                        );
                        result.push_str(&expanded);
                    } else {
                        // Object-like macro used with parens — just emit body as-is
                        result.push_str(&macro_def.body);
                    }
                } else {
                    // Object-like macro (no parens)
                    result.push_str(&macro_def.body);
                }
                i = arg_end;
            } else {
                // Unknown macro — leave as-is (may be a UVM macro or tree-sitter can handle it)
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Find the position of the matching `)` in a string that starts right after `(`.
/// Returns the index (relative to the start of `s`) of the closing `)`.
fn find_matching_paren(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.chars().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    return Some(i);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    None
}

/// Parse macro formal parameter list: "x, y, z=value" -> (["x","y","z"], {"z":"value"})
fn parse_macro_params(s: &str) -> (Vec<String>, HashMap<String, String>) {
    let mut names = Vec::new();
    let mut defaults = HashMap::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(eq_pos) = part.find('=') {
            let name = part[..eq_pos].trim().to_string();
            let default = part[eq_pos + 1..].trim().to_string();
            defaults.insert(name.clone(), default);
            names.push(name);
        } else {
            names.push(part.to_string());
        }
    }
    (names, defaults)
}

/// Parse actual arguments from a macro invocation: `"hello", "world"` -> ["\"hello\"", "\"world\""]
/// Respects nested parentheses and quoted strings.
fn parse_macro_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_quote: Option<char> = None;

    for ch in s.chars() {
        match ch {
            '"' | '\'' if in_quote.is_none() => {
                in_quote = Some(ch);
                current.push(ch);
            }
            q if in_quote == Some(q) => {
                in_quote = None;
                current.push(ch);
            }
            _ if in_quote.is_some() => {
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    args
}

/// Substitute formal parameters in a macro body with actual arguments.
/// If an actual arg is missing, uses the default value if available, otherwise empty string.
fn substitute_macro_body(
    body: &str,
    params: &[String],
    defaults: &HashMap<String, String>,
    actual_args: &[String],
) -> String {
    // Build a substitution map: formal param name -> actual value
    let mut substitutions: HashMap<&str, &str> = HashMap::new();
    for (idx, param_name) in params.iter().enumerate() {
        let value = if idx < actual_args.len() {
            actual_args[idx].as_str()
        } else if let Some(default) = defaults.get(param_name) {
            default.as_str()
        } else {
            ""
        };
        substitutions.insert(param_name.as_str(), value);
    }

    // Sort params by length descending to avoid partial replacements
    // (e.g., replacing "ab" before "a" could corrupt "abc")
    let mut sorted_params: Vec<&str> = substitutions.keys().copied().collect();
    sorted_params.sort_by_key(|p| std::cmp::Reverse(p.len()));

    let mut result = body.to_string();
    for param in &sorted_params {
        // Only replace whole-word occurrences (word boundary check)
        // Simple approach: replace `param` when preceded/followed by non-alphanumeric/underscore
        result = replace_whole_word(&result, param, substitutions[param]);
    }
    result
}

/// Replace whole-word occurrences of `from` with `to` in `s`.
fn replace_whole_word(s: &str, from: &str, to: &str) -> String {
    if from.is_empty() {
        return s.to_string();
    }
    let mut result = String::with_capacity(s.len());
    let from_chars: Vec<char> = from.chars().collect();
    let s_chars: Vec<char> = s.chars().collect();
    let from_len = from_chars.len();
    let mut i = 0;

    while i <= s_chars.len() {
        if i + from_len <= s_chars.len() && s_chars[i..i + from_len] == from_chars[..] {
            // Check word boundary before
            let before_ok = i == 0 || {
                let prev = s_chars[i - 1];
                !(prev.is_ascii_alphanumeric() || prev == '_')
            };
            // Check word boundary after
            let after_ok = i + from_len >= s_chars.len() || {
                let next = s_chars[i + from_len];
                !(next.is_ascii_alphanumeric() || next == '_')
            };
            if before_ok && after_ok {
                result.push_str(to);
                i += from_len;
                continue;
            }
        }
        if i < s_chars.len() {
            result.push(s_chars[i]);
        }
        i += 1;
    }
    result
}

/// Check if a macro name is a preprocessor directive (should not be expanded inline).
fn is_directive(name: &str) -> bool {
    matches!(
        name,
        "ifdef" | "ifndef" | "elsif" | "else" | "endif"
            | "define" | "undef" | "include"
            | "begin_keywords" | "end_keywords"
            | "line" | "resetall" | "timescale"
            | "celldefine" | "endcelldefine"
            | "default_nettype" | "undefineall"
            | "pragma"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defines_from(pairs: &[(&str, &str)]) -> HashMap<String, MacroDef> {
        pairs.iter().map(|(k, v)| (k.to_string(), MacroDef::object(v.to_string()))).collect()
    }

    #[test]
    fn test_basic_define_expand() {
        let mut defs = defines_from(&[("WIDTH", "8")]);
        let src = "logic [`WIDTH-1:0] data;\n";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("logic [8-1:0] data;"), "Got: {}", result);
    }

    #[test]
    fn test_ifdef_active() {
        let mut defs = defines_from(&[("ENABLE", "1")]);
        let src = "\
`ifdef ENABLE
wire active;
`else
wire inactive;
`endif
wire always_here;
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("active"), "Expected active, got:\n{}", result);
        assert!(!result.contains("inactive"), "Expected no inactive, got:\n{}", result);
        assert!(result.contains("always_here"), "Expected always_here, got:\n{}", result);
    }

    #[test]
    fn test_ifndef() {
        let mut defs = HashMap::new();
        let src = "\
`ifndef GUARD
wire unguarded;
`endif
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("unguarded"), "Expected unguarded, got:\n{}", result);
    }

    #[test]
    fn test_ifdef_inactive() {
        let mut defs = HashMap::new();
        let src = "\
`ifdef NOT_DEFINED
wire should_not_appear;
`else
wire should_appear;
`endif
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sd"), &mut vec![]);
        assert!(!result.contains("should_not_appear"), "Got:\n{}", result);
        assert!(result.contains("should_appear"), "Got:\n{}", result);
    }

    #[test]
    fn test_nested_ifdef() {
        let mut defs = defines_from(&[("A", "1")]);
        let src = "\
`ifdef A
  `ifdef B
    wire ab;
  `else
    wire a_only;
  `endif
`else
  wire neither;
`endif
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("a_only"), "Expected a_only, got:\n{}", result);
        assert!(!result.contains("ab"), "Expected no ab, got:\n{}", result);
        assert!(!result.contains("neither"), "Expected no neither, got:\n{}", result);
    }

    #[test]
    fn test_elsif() {
        let mut defs = defines_from(&[("MODE", "2")]);
        let src = "\
`ifdef MODE_1
  wire mode1;
`elsif MODE
  wire mode2;
`else
  wire mode_default;
`endif
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("mode2"), "Expected mode2, got:\n{}", result);
        assert!(!result.contains("mode1"), "Got:\n{}", result);
        assert!(!result.contains("mode_default"), "Got:\n{}", result);
    }

    #[test]
    fn test_define_then_use() {
        let mut defs = HashMap::new();
        let src = "\
`define ADDR_W 32
`define DATA_W 64
logic [`ADDR_W-1:0] addr;
logic [`DATA_W-1:0] data;
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("[32-1:0] addr"), "Got:\n{}", result);
        assert!(result.contains("[64-1:0] data"), "Got:\n{}", result);
    }

    #[test]
    fn test_undef() {
        let mut defs = defines_from(&[("FLAG", "1")]);
        let src = "\
`undef FLAG
`ifdef FLAG
  wire should_not;
`else
  wire should_exist;
`endif
";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("should_exist"), "Got:\n{}", result);
        assert!(!result.contains("should_not"), "Got:\n{}", result);
    }

    #[test]
    fn test_unknown_macro_passthrough() {
        let mut defs = HashMap::new();
        let src = "module test;\n  `ASSERT(something)\nendmodule\n";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        // Unknown macros should pass through
        assert!(result.contains("`ASSERT"), "Got:\n{}", result);
    }

    #[test]
    fn test_function_like_macro_expansion() {
        let mut defs = HashMap::new();
        let src = "`define D(x,y) initial $display(x, y);\n`D(\"hello\", \"world\")\n";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("initial $display(\"hello\", \"world\");"), "Got:\n{}", result);
    }

    #[test]
    fn test_function_like_macro_with_defaults() {
        let mut defs = HashMap::new();
        let src = "`define LOG(msg, level=INFO) $display(\"[%0s] %0s\", level, msg);\n`LOG(\"test\")\n";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("$display(\"[%0s] %0s\", INFO, \"test\");"), "Got:\n{}", result);
    }

    #[test]
    fn test_function_like_macro_expansion_with_nested_parens() {
        let mut defs = HashMap::new();
        let src = "`define WRAP(x) (x)\n`WRAP(a + b)\n";
        let result = preprocess_sv_source(src, &mut defs, &[], Path::new("test.sv"), &mut vec![]);
        assert!(result.contains("(a + b)"), "Got:\n{}", result);
    }
}
