use crate::preprocessor::pass2_scanner::UVMMacroCall;
use crate::preprocessor::SourceMapping;

/// Expand UVM macro calls into synthetic SV code.
///
/// Operates line-by-line on the preprocessed source.  Lines that contain
/// UVM macro calls are commented out and replaced by expanded SV code.
/// Returns (expanded_source, source_mapping).
pub fn expand(source: &str, calls: &[UVMMacroCall]) -> (String, Vec<SourceMapping>) {
    if calls.is_empty() {
        return (source.to_string(), Vec::new());
    }

    let mut result = String::new();
    let mut mappings = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    // Sort calls by line for sequential processing
    let mut sorted_calls: Vec<&UVMMacroCall> = calls.iter().collect();
    sorted_calls.sort_by_key(|c| c.line);

    let mut call_idx = 0;

    for (i, line) in lines.iter().enumerate() {
        let line_num = i as u32 + 1;

        // Collect all UVM calls on this line (guaranteed sorted by line)
        let mut line_calls: Vec<&UVMMacroCall> = Vec::new();
        while call_idx < sorted_calls.len() && sorted_calls[call_idx].line == line_num {
            line_calls.push(sorted_calls[call_idx]);
            call_idx += 1;
        }

        if !line_calls.is_empty() {
            // Comment out the original macro line for traceability
            result.push_str("// ");
            result.push_str(line);
            result.push('\n');

            for call in &line_calls {
                let expanded = expand_one(&call.macro_name, &call.args);
                let exp_start = result.len();
                result.push_str(&expanded);
                result.push('\n');
                let exp_end = result.len();

                mappings.push(SourceMapping {
                    expanded_start: exp_start,
                    expanded_end: exp_end,
                    original_file: String::new(),
                    original_line: line_num,
                    macro_name: call.macro_name.clone(),
                    macro_args: call.args.clone(),
                });
            }
        } else {
            // Regular line: pass through verbatim
            result.push_str(line);
            result.push('\n');
        }
    }

    (result, mappings)
}

/// Expand a single UVM macro into equivalent SV code.
fn expand_one(macro_name: &str, args: &[String]) -> String {
    match macro_name {
        // ===================== Factory Registration =====================
        "uvm_component_utils" => {
            let t = args.first().map(|s| s.as_str()).unwrap_or("T");
            format!(
                "typedef uvm_component_registry #({t}, \"{t}\") type_id;
static function type_id get_type();
    return type_id::get();
endfunction
virtual function uvm_object_wrapper get_object_type();
    return type_id::get();
endfunction
function string get_type_name();
    return \"{t}\";
endfunction"
            )
        }

        "uvm_object_utils" => {
            let t = args.first().map(|s| s.as_str()).unwrap_or("T");
            format!(
                "typedef uvm_object_registry #({t}, \"{t}\") type_id;
static function type_id get_type();
    return type_id::get();
endfunction
virtual function uvm_object_wrapper get_object_type();
    return type_id::get();
endfunction
function string get_type_name();
    return \"{t}\";
endfunction"
            )
        }

        "uvm_component_param_utils" => {
            let t = args.first().map(|s| s.as_str()).unwrap_or("T");
            format!(
                "typedef uvm_component_registry #({t}, \"{t}\") type_id;
static function type_id get_type();
    return type_id::get();
endfunction"
            )
        }

        "uvm_object_param_utils" => {
            let t = args.first().map(|s| s.as_str()).unwrap_or("T");
            format!(
                "typedef uvm_object_registry #({t}, \"{t}\") type_id;
static function type_id get_type();
    return type_id::get();
endfunction"
            )
        }

        // ===================== Field Automation =====================
        "uvm_field_int" => {
            let field = args.first().map(|s| s.as_str()).unwrap_or("field");
            let flags = args.get(1).map(|s| s.as_str()).unwrap_or("UVM_ALL_ON");
            format!(
                "function void do_print(uvm_printer printer);
    super.do_print(printer);
    printer.print_field_int(\"{field}\", {field}, $bits({field}), {flags});
endfunction
function void do_copy(uvm_object rhs);
    _rhs_type _rhs;
    if (!$cast(_rhs, rhs)) return;
    super.do_copy(rhs);
    this.{field} = _rhs.{field};
endfunction"
            )
        }

        "uvm_field_string" => {
            let field = args.first().map(|s| s.as_str()).unwrap_or("field");
            format!(
                "function void do_print(uvm_printer printer);
    super.do_print(printer);
    printer.print_field_string(\"{field}\", {field}, UVM_DEC);
endfunction
function void do_copy(uvm_object rhs);
    _rhs_type _rhs;
    if (!$cast(_rhs, rhs)) return;
    super.do_copy(rhs);
    this.{field} = _rhs.{field};
endfunction"
            )
        }

        // ===================== Sequence Macros =====================
        "uvm_do" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            format!(
                "begin
    {item} = {item}_type::type_id::create(\"{item}\", get_full_name(), , get_context());
    start_item({item});
    assert({item}.randomize());
    finish_item({item});
end"
            )
        }

        "uvm_do_with" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            let constraints = args.get(1).map(|s| s.as_str()).unwrap_or("");
            format!(
                "begin
    {item} = {item}_type::type_id::create(\"{item}\", get_full_name(), , get_context());
    start_item({item});
    assert({item}.randomize() with {{{constraints}}});
    finish_item({item});
end"
            )
        }

        "uvm_do_on" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            let sequencer = args.get(1).map(|s| s.as_str()).unwrap_or("sequencer");
            format!(
                "begin
    {item} = {item}_type::type_id::create(\"{item}\", get_full_name(), , get_context());
    start_item({item}, {sequencer});
    assert({item}.randomize());
    finish_item({item});
end"
            )
        }

        "uvm_do_on_with" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            let sequencer = args.get(1).map(|s| s.as_str()).unwrap_or("sequencer");
            let constraints = args.get(2).map(|s| s.as_str()).unwrap_or("");
            format!(
                "begin
    {item} = {item}_type::type_id::create(\"{item}\", get_full_name(), , get_context());
    start_item({item}, {sequencer});
    assert({item}.randomize() with {{{constraints}}});
    finish_item({item});
end"
            )
        }

        "uvm_create" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            format!(
                "{item} = {item}_type::type_id::create(\"{item}\", get_full_name(), , get_context());"
            )
        }

        "uvm_send" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            format!(
                "begin
    start_item({item});
    finish_item({item});
end"
            )
        }

        "uvm_rand_send" => {
            let item = args.first().map(|s| s.as_str()).unwrap_or("item");
            format!(
                "begin
    start_item({item});
    assert({item}.randomize());
    finish_item({item});
end"
            )
        }

        // ===================== Message / Report Macros =====================
        "uvm_info" => {
            let id = args.first().map(|s| s.as_str()).unwrap_or("\"ID\"");
            let msg = args.get(1).map(|s| s.as_str()).unwrap_or("\"msg\"");
            let verbosity = args.get(2).map(|s| s.as_str()).unwrap_or("UVM_MEDIUM");
            format!(
                "begin
    if (uvm_report_enabled({verbosity}, UVM_INFO, {id}))
        uvm_report_info({id}, $sformatf({msg}), {verbosity}, `__FILE__, `__LINE__);
end"
            )
        }

        "uvm_error" => {
            let id = args.first().map(|s| s.as_str()).unwrap_or("\"ID\"");
            let msg = args.get(1).map(|s| s.as_str()).unwrap_or("\"msg\"");
            format!(
                "uvm_report_error({id}, $sformatf({msg}), UVM_NONE, `__FILE__, `__LINE__);"
            )
        }

        "uvm_warning" => {
            let id = args.first().map(|s| s.as_str()).unwrap_or("\"ID\"");
            let msg = args.get(1).map(|s| s.as_str()).unwrap_or("\"msg\"");
            format!(
                "uvm_report_warning({id}, $sformatf({msg}), UVM_NONE, `__FILE__, `__LINE__);"
            )
        }

        "uvm_fatal" => {
            let id = args.first().map(|s| s.as_str()).unwrap_or("\"ID\"");
            let msg = args.get(1).map(|s| s.as_str()).unwrap_or("\"msg\"");
            format!(
                "uvm_report_fatal({id}, $sformatf({msg}), UVM_NONE, `__FILE__, `__LINE__);"
            )
        }

        // ===================== Registration Blocks =====================
        "uvm_component_utils_begin" | "uvm_object_utils_begin" => {
            let t = args.first().map(|s| s.as_str()).unwrap_or("T");
            format!(
                "typedef uvm_component_registry #({t}, \"{t}\") type_id;
static function type_id get_type();
    return type_id::get();
endfunction
virtual function uvm_object_wrapper get_object_type();
    return type_id::get();
endfunction
function string get_type_name();
    return \"{t}\";
endfunction
function void build_phase(uvm_phase phase);
    super.build_phase(phase);"
            )
        }

        "uvm_component_utils_end" | "uvm_object_utils_end" => {
            "endfunction".to_string()
        }

        // ===================== Factory Override =====================
        "uvm_set_type_override_by_type" => {
            let a = args.first().map(|s| s.as_str()).unwrap_or("A");
            let b = args.get(1).map(|s| s.as_str()).unwrap_or("B");
            format!("{a}::type_id::set_type_override({b}::get_type());")
        }

        // ===================== Default (safety net) =====================
        _ => String::new(),
    }
}
