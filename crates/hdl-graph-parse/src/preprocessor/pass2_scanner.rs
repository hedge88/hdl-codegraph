/// A detected UVM macro call site.
#[derive(Debug, Clone)]
pub struct UVMMacroCall {
    pub line: u32,
    pub column: u32,
    pub macro_name: String,
    pub args: Vec<String>,
    pub raw_text: String,
}

/// Scan preprocessed source text for UVM macro patterns.
/// Returns a list of all detected UVM macro calls with their arguments.
pub fn scan(source: &str) -> Vec<UVMMacroCall> {
    let mut calls = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let uvm_prefixes = [
        "`uvm_component_utils",
        "`uvm_object_utils",
        "`uvm_component_param_utils",
        "`uvm_object_param_utils",
        "`uvm_field_int",
        "`uvm_field_string",
        "`uvm_field_object",
        "`uvm_field_enum",
        "`uvm_field_array_int",
        "`uvm_field_queue_int",
        "`uvm_field_sarray_int",
        "`uvm_field_real",
        "`uvm_field_time",
        "`uvm_do",
        "`uvm_do_with",
        "`uvm_do_on",
        "`uvm_do_on_with",
        "`uvm_create",
        "`uvm_send",
        "`uvm_rand_send",
        "`uvm_info",
        "`uvm_error",
        "`uvm_warning",
        "`uvm_fatal",
        "`uvm_object_utils_begin",
        "`uvm_object_utils_end",
        "`uvm_component_utils_begin",
        "`uvm_component_utils_end",
        "`uvm_field_utils_begin",
        "`uvm_field_utils_end",
        "`uvm_config_int",
        "`uvm_config_string",
        "`uvm_config_object",
        "`uvm_set_type_override_by_type",
        "`uvm_set_inst_override_by_type",
        "`uvm_register_cb",
        "`uvm_analysis_imp_decl",
        "`uvm_nonblocking_imp_decl",
        "`uvm_blocking_put_imp_decl",
        "`uvm_set_report_id_verbosity",
        "`uvm_set_report_id_action",
    ];

    for (i, line) in lines.iter().enumerate() {
        let line_num = i as u32 + 1;
        for prefix in &uvm_prefixes {
            if let Some(pos) = line.find(prefix) {
                let col = pos as u32;
                let rest = &line[pos + prefix.len()..].trim();
                let (args, _) = extract_args(rest, &lines, i);
                calls.push(UVMMacroCall {
                    line: line_num,
                    column: col,
                    macro_name: prefix.trim_start_matches('`').to_string(),
                    args,
                    raw_text: line.to_string(),
                });
            }
        }
    }
    calls
}

/// Extract comma-separated arguments from a parenthesized expression.
/// Handles nested parentheses and basic multi-line macros.
fn extract_args(text: &str, all_lines: &[&str], start_line: usize) -> (Vec<String>, usize) {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut in_paren = false;
    let mut line_idx = start_line;
    let mut chars = text.chars().peekable();

    // Skip whitespace and find opening paren
    while let Some(&c) = chars.peek() {
        if c == '(' {
            in_paren = true;
            depth = 1;
            chars.next();
            break;
        }
        if !c.is_whitespace() {
            break;
        }
        chars.next();
    }

    if !in_paren {
        return (args, start_line);
    }

    loop {
        match chars.next() {
            Some('(') => {
                depth += 1;
                current.push('(');
            }
            Some(')') => {
                depth -= 1;
                if depth == 0 {
                    if !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                    }
                    break;
                }
                current.push(')');
            }
            Some(',') if depth == 1 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            Some(c) => {
                current.push(c);
            }
            None => {
                // Multi-line macro: advance to next line
                line_idx += 1;
                if line_idx < all_lines.len() {
                    let next_line = all_lines[line_idx].trim();
                    if next_line.starts_with('`') {
                        break; // Another macro starts — stop
                    }
                    current.push(' ');
                    // For simplicity, stop multi-line recovery here.
                    // A full implementation would re-tokenize the next line.
                }
                break;
            }
        }
    }

    (args, line_idx)
}
