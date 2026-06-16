use std::path::Path;
use anyhow::Result;
use tree_sitter::Parser;

pub struct FileScanner {
    parser: Parser,
    include_dirs: Vec<String>,
}

impl FileScanner {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&hdl_graph_grammar::language_ref())
            .map_err(|e| anyhow::anyhow!("Failed to set grammar: {e}"))?;
        Ok(Self {
            parser,
            include_dirs: Vec::new(),
        })
    }

    pub fn with_include_dirs(dirs: Vec<String>) -> Result<Self> {
        let mut s = Self::new()?;
        s.include_dirs = dirs;
        Ok(s)
    }

    pub fn parse_file(&mut self, path: &Path) -> Result<tree_sitter::Tree> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", path.display()))?;
        let tree = self
            .parser
            .parse(&content, None)
            .ok_or_else(|| anyhow::anyhow!("Parse returned None for {}", path.display()))?;
        Ok(tree)
    }

    pub fn parse_source(&mut self, source: &str) -> tree_sitter::Tree {
        self.parser.parse(source, None).unwrap()
    }

    /// Parse source text, reusing an old tree for incremental parsing.
    ///
    /// Tree-sitter's incremental parser is O(log n) for typical edits,
    /// reusing the old tree's state to avoid a full re-parse.
    /// Pass `None` for `old_tree` to perform a full parse (identical to
    /// `parse_source`).
    pub fn parse_source_incremental(&mut self, source: &str, old_tree: Option<&tree_sitter::Tree>) -> tree_sitter::Tree {
        self.parser.parse(source, old_tree).unwrap()
    }
}
