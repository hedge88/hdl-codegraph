use std::process::Command;

fn main() {
    // Compile and run a small program that dumps the tree
    let src = r#"
fn dump_tree() {
    let sv = r#"module top(input wire clk, output reg [7:0] data, inout wire bidir); endmodule"#;
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&hdl_graph_grammar::language_ref()).unwrap();
    let tree = parser.parse(sv, None).unwrap();
    let root = tree.root_node();
    println!("Root kind: {}", root.kind());
    println!("Root child count: {}", root.child_count());
    println!();
    print_tree(root, sv.as_bytes(), 0);
}

fn print_tree(node: tree_sitter::Node, source: &[u8], depth: usize) {
    let indent = "  ".repeat(depth);
    let kind = node.kind();
    let text = node.utf8_text(source).unwrap_or("");
    println!("{}{} \"{}\"", indent, kind, text);
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            print_tree(child, source, depth + 1);
        }
    }
}

fn main() {
    dump_tree();
}
"#;
    println!("{}", src);
}
