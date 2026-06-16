// UVM 4-pass preprocessor — transforms SV source before tree-sitter parsing
//
// Pipeline:
//   raw source → Pass1(standard preproc) → Pass2(UVM scanner)
//     → Pass3(expansion) → Pass4(merge) → expanded source → tree-sitter

pub mod pass1_standard;
pub mod pass2_scanner;
pub mod pass3_expander;
pub mod pass4_merger;
pub mod patterns;

use std::collections::HashMap;

/// Result of preprocessing a single file.
pub struct PreprocessedFile {
    /// The expanded source text (ready for tree-sitter).
    pub expanded_source: String,
    /// Original file path
    pub file_path: String,
    /// Mapping: expanded_byte_range → (original_file, original_line, macro_name)
    pub source_map: Vec<SourceMapping>,
    /// Include dependency graph (file → [included files])
    pub includes: Vec<String>,
    /// Whether UVM macros were expanded
    pub has_uvm_macros: bool,
}

#[derive(Debug, Clone)]
pub struct SourceMapping {
    pub expanded_start: usize,
    pub expanded_end: usize,
    pub original_file: String,
    pub original_line: u32,
    pub macro_name: String,
    pub macro_args: Vec<String>,
}

/// Run the full 4-pass pipeline on a source file.
pub fn preprocess(
    source: &str,
    file_path: &str,
    defines: &HashMap<String, String>,
    include_dirs: &[String],
) -> PreprocessedFile {
    // Pass 1: Standard preprocessing
    let (after_pass1, includes) = pass1_standard::run(source, defines, include_dirs);

    // Pass 2: Scan for UVM macro patterns
    let uvm_calls = pass2_scanner::scan(&after_pass1);

    // Pass 3: Expand UVM macros
    let (expanded, source_map) = pass3_expander::expand(&after_pass1, &uvm_calls);

    // Pass 4: Merge into final source
    let result = pass4_merger::merge(source, &expanded, &source_map);

    PreprocessedFile {
        expanded_source: result,
        file_path: file_path.to_string(),
        source_map,
        includes,
        has_uvm_macros: !uvm_calls.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: run the pipeline and panic-print on failure.
    fn preprocess_str(source: &str) -> PreprocessedFile {
        preprocess(source, "test.sv", &HashMap::new(), &[])
    }

    #[test]
    fn test_uvm_component_utils_expansion() {
        let sv = r#"
class my_driver extends uvm_driver;
    `uvm_component_utils(my_driver)

    function new(string name, uvm_component parent);
        super.new(name, parent);
    endfunction
endclass
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        // The expansion should contain factory registration code
        assert!(
            out.contains("type_id"),
            "Expected expanded source to contain 'type_id', got:\n{out}"
        );
        assert!(
            out.contains("uvm_component_registry"),
            "Expected expanded source to contain 'uvm_component_registry', got:\n{out}"
        );
        assert!(
            out.contains("my_driver"),
            "Expected expanded source to reference 'my_driver', got:\n{out}"
        );
        assert!(
            result.has_uvm_macros,
            "Expected has_uvm_macros = true"
        );
        // The original macro line should be commented out
        assert!(
            out.contains("//"),
            "Expected macro line to be commented out, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_object_utils_expansion() {
        let sv = r#"
class my_item extends uvm_sequence_item;
    `uvm_object_utils(my_item)
endclass
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("uvm_object_registry"),
            "Expected uvm_object_registry, got:\n{out}"
        );
        assert!(
            out.contains("my_item"),
            "Expected my_item reference, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_info_expansion() {
        let sv = r#"
initial begin
    `uvm_info("DRIVER", "running", UVM_MEDIUM)
end
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("uvm_report_info"),
            "Expected uvm_report_info call, got:\n{out}"
        );
        assert!(
            out.contains("DRIVER"),
            "Expected DRIVER id in output, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_error_expansion() {
        let sv = r#"`uvm_error("TB", "fail")"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("uvm_report_error"),
            "Expected uvm_report_error, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_warning_expansion() {
        let sv = r#"`uvm_warning("TB", "warn")"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("uvm_report_warning"),
            "Expected uvm_report_warning, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_fatal_expansion() {
        let sv = r#"`uvm_fatal("TB", "dead")"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("uvm_report_fatal"),
            "Expected uvm_report_fatal, got:\n{out}"
        );
    }

    #[test]
    fn test_include_resolution() {
        let sv = r#"
`include "stimulus.sv"
module test;
    // ...
endmodule
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("// `include \"stimulus.sv\""),
            "Expected include placeholder comment, got:\n{out}"
        );
        assert!(
            result.includes.contains(&"stimulus.sv".to_string()),
            "Expected includes list to contain 'stimulus.sv', got: {:?}",
            result.includes
        );
    }

    #[test]
    fn test_ifdef_resolution() {
        let mut defines = HashMap::new();
        defines.insert("ENABLE_FEATURE".to_string(), "1".to_string());
        let sv = r#"
`ifdef ENABLE_FEATURE
    wire feature_enabled = 1;
`else
    wire feature_disabled = 1;
`endif
wire always_present;
"#;
        let result = preprocess(sv, "test.sv", &defines, &[]);
        let out = &result.expanded_source;

        assert!(
            out.contains("feature_enabled"),
            "Expected feature_enabled (ifdef active), got:\n{out}"
        );
        assert!(
            !out.contains("feature_disabled"),
            "Expected feature_disabled to be excluded, got:\n{out}"
        );
        assert!(
            out.contains("always_present"),
            "Expected always_present (outside ifdef), got:\n{out}"
        );
    }

    #[test]
    fn test_define_expansion() {
        let sv = r#"
`define WIDTH 8
`define DATA_TYPE logic
`DATA_TYPE [7:0] data;
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("logic"),
            "Expected `DATA_TYPE to expand to 'logic', got:\n{out}"
        );
        // The macro line itself should be replaced by the expansion
        assert!(
            !out.contains("`DATA_TYPE"),
            "Expected `DATA_TYPE to be expanded away, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_do_expansion() {
        let sv = r#"
task body();
    `uvm_do(req)
endtask
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("start_item"),
            "Expected start_item in expansion, got:\n{out}"
        );
        assert!(
            out.contains("finish_item"),
            "Expected finish_item in expansion, got:\n{out}"
        );
    }

    #[test]
    fn test_uvm_create_send_expansion() {
        let sv = r#"
`uvm_create(my_item)
`uvm_send(my_item)
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("type_id::create"),
            "Expected type_id::create in `uvm_create expansion, got:\n{out}"
        );
        assert!(
            out.contains("start_item"),
            "Expected start_item in `uvm_send expansion, got:\n{out}"
        );
    }

    #[test]
    fn test_no_uvm_macros() {
        let sv = "module empty;\nendmodule\n";
        let result = preprocess_str(sv);
        assert!(
            !result.has_uvm_macros,
            "Expected has_uvm_macros = false for plain SV"
        );
        // Source should pass through unchanged (modulo trailing newline)
        assert!(
            result.expanded_source.contains("module empty"),
            "Expected module declaration to pass through"
        );
    }

    #[test]
    fn test_source_mapping_entries() {
        let sv = r#"
`uvm_component_utils(my_driver)
`uvm_info("ID", "msg", UVM_MEDIUM)
"#;
        let result = preprocess_str(sv);

        assert_eq!(
            result.source_map.len(),
            2,
            "Expected two source mapping entries, got {}",
            result.source_map.len()
        );

        // First mapping should be `uvm_component_utils
        assert_eq!(result.source_map[0].macro_name, "uvm_component_utils");
        assert_eq!(result.source_map[0].original_line, 2);

        // Second mapping should be uvm_info
        assert_eq!(result.source_map[1].macro_name, "uvm_info");
        assert_eq!(result.source_map[1].original_line, 3);

        // Expanded positions should be non-zero
        assert!(result.source_map[0].expanded_start < result.source_map[0].expanded_end);
        assert!(result.source_map[1].expanded_start < result.source_map[1].expanded_end);
    }

    #[test]
    fn test_uvm_empty_source() {
        let sv = "";
        let result = preprocess_str(sv);
        assert!(!result.has_uvm_macros);
        assert!(result.expanded_source.is_empty());
    }

    #[test]
    fn test_multiple_calls_same_line() {
        // Two field macros on the same line
        let sv = r#"class test;
    `uvm_field_int(A, UVM_ALL_ON) `uvm_field_string(B, UVM_ALL_ON)
endclass
"#;
        let result = preprocess_str(sv);
        let out = &result.expanded_source;

        assert!(
            out.contains("print_field_int"),
            "Expected print_field_int, got:\n{out}"
        );
        assert!(
            out.contains("print_field_string"),
            "Expected print_field_string, got:\n{out}"
        );
    }
}
