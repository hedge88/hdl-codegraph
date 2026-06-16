fn main() {
    let src_dir = std::path::PathBuf::from("grammar/src");

    if !src_dir.join("parser.c").exists() {
        println!("cargo:warning=parser.c not found. Run: make fetch-grammar");
        return;
    }

    cc::Build::new()
        .cpp(false)
        .warnings(false)
        .extra_warnings(false)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-function")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-missing-field-initializers")
        .file(src_dir.join("parser.c"))
        .include(src_dir.join("tree_sitter"))
        .compile("tree-sitter-systemverilog");

    println!("cargo:rerun-if-changed=grammar/src/parser.c");
    println!("cargo:rerun-if-changed=grammar/src/tree_sitter/parser.h");
}
