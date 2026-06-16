use tree_sitter::Language;

/// Returns the SystemVerilog tree-sitter Language.
///
/// This wraps the gmlarumbe/tree-sitter-systemverilog grammar
/// (816 grammar rules, IEEE 1800-2023 coverage).
pub fn language() -> Language {
    unsafe { tree_sitter_systemverilog() }
}

extern "C" {
    fn tree_sitter_systemverilog() -> Language;
}

/// Convenience: get the language reference for Parser::set_language.
pub fn language_ref() -> Language {
    language()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_grammar_loads() {
        let mut parser = Parser::new();
        assert!(parser.set_language(&language_ref()).is_ok());
    }

    #[test]
    fn test_parse_empty() {
        let mut parser = Parser::new();
        parser.set_language(&language_ref()).unwrap();
        let tree = parser.parse("", None).unwrap();
        // Empty input is valid SV — parser produces a source_file with 0 children
        assert_eq!(tree.root_node().kind(), "source_file");
        assert_eq!(tree.root_node().child_count(), 0);
    }

    #[test]
    fn test_parse_simple_module() {
        let mut parser = Parser::new();
        parser.set_language(&language_ref()).unwrap();

        let sv = r#"
module counter #(parameter WIDTH = 8) (
    input  logic        clk,
    input  logic        rst_n,
    output logic [WIDTH-1:0] count
);
    always_ff @(posedge clk or negedge rst_n) begin
        if (!rst_n)
            count <= '0;
        else
            count <= count + 1'b1;
    end
endmodule
"#;
        let tree = parser.parse(sv, None).unwrap();
        let root = tree.root_node();
        assert!(!root.has_error(), "Parse failed for simple SV module");
        assert_eq!(root.kind(), "source_file");

        // Walk tree to find module_declaration
        let mut found = false;
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if child.kind() == "module_declaration" {
                found = true;
            }
        }
        assert!(found, "Should find module_declaration node");
    }
}
