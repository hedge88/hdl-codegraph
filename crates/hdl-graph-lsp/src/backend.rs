use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

use tower_lsp::{Client, LanguageServer};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use hdl_graph_core::*;
use hdl_graph_parse::FileScanner;
use hdl_graph_storage::InMemoryGraph;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// A raw token with absolute line/col before LSP delta encoding.
struct RawToken {
    line: u32,
    col: u32,
    length: u32,
    token_type: u32,
}

/// Completion-zone classification for context-aware suggestions.
#[derive(PartialEq)]
enum CompletionZone {
    ModuleLevel,
    ModuleBody,
    ClassBody,
}

// ---------------------------------------------------------------------------
// Index state & Backend
// ---------------------------------------------------------------------------

struct IndexState {
    graph: InMemoryGraph,
    symbols: SymbolTable,
    file_map: HashMap<String, u64>,
}

pub struct Backend {
    client: Client,
    state: Arc<tokio::sync::RwLock<Option<IndexState>>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            state: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    async fn build_index(root: &str) -> Option<IndexState> {
        let project = std::path::Path::new(root);

        let mut scanner = FileScanner::new().ok()?;
        let mut extractor = hdl_graph_parse::GraphExtractor::new();
        let mut graph = InMemoryGraph::new();
        let mut file_map = HashMap::new();
        let mut file_id = 0u64;

        // Walk SV files
        let mut files = Vec::new();
        if let Ok(entries) = std::fs::read_dir(project) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "sv" | "svh" | "v" | "vh") {
                        files.push(path);
                    }
                }
            }
        }

        for path in &files {
            let rel = path.strip_prefix(project).unwrap_or(path).to_string_lossy().to_string();
            if let Ok(tree) = scanner.parse_file(path) {
                let source = std::fs::read_to_string(path).unwrap_or_default();
                file_id += 1;
                file_map.insert(rel, file_id);
                let (nodes, edges) = extractor.extract(&tree, source.as_bytes(), file_id);
                for n in nodes { graph.add_node(n).ok(); }
                for e in edges { graph.add_edge(e).ok(); }
            }
        }

        Some(IndexState { graph, symbols: extractor.symbols, file_map })
    }

    // ── Semantic tokenizer ────────────────────────────────────────────────
    // Token-type indices (must match the legend order in ServerCapabilities):
    //   0 = COMMENT, 1 = KEYWORD, 2 = STRING, 3 = TYPE, 4 = FUNCTION, 5 = VARIABLE

    /// Tokenize Verilog/SystemVerilog source and return LSP delta-encoded
    /// semantic tokens.
    fn tokenize_semantic(content: &str) -> Vec<SemanticToken> {
        let keywords: HashSet<&str> = [
            "module", "endmodule", "input", "output", "inout", "wire", "reg", "logic",
            "always", "always_comb", "always_ff", "always_latch",
            "assign", "begin", "end", "if", "else", "case", "endcase", "for",
            "generate", "endgenerate", "function", "endfunction", "task", "endtask",
            "class", "endclass", "interface", "endinterface", "package", "endpackage",
            "typedef", "enum", "struct", "union", "parameter", "localparam", "import",
            "export", "virtual", "extends", "new", "this", "super", "return", "fork",
            "join", "wait", "repeat", "while", "forever", "initial", "posedge", "negedge",
            "edge", "or", "and", "nand", "nor", "xor", "xnor", "buf", "not", "integer",
            "real", "time", "string", "event", "chandle", "bit", "byte", "shortint",
            "int", "longint",
            "`ifdef", "`endif", "`else", "`elsif", "`define", "`include", "`ifndef",
            "`timescale", "`undef", "`line",
        ].iter().copied().collect();

        let chars: Vec<char> = content.chars().collect();
        let len = chars.len();
        let mut raw: Vec<RawToken> = Vec::new();

        let mut line = 0u32;
        let mut col = 0u32;
        let mut i = 0;

        while i < len {
            let ch = chars[i];

            // Track newlines
            if ch == '\n' {
                line += 1;
                col = 0;
                i += 1;
                continue;
            }
            if ch == '\r' {
                i += 1;
                col += 1;
                continue;
            }

            // Block comments: /* ... */
            if ch == '/' && i + 1 < len && chars[i + 1] == '*' {
                let start_line = line;
                let start_col = col;
                let start = i;
                i += 2;
                col += 2;
                while i + 1 < len {
                    if chars[i] == '*' && chars[i + 1] == '/' {
                        i += 2;
                        col += 2;
                        break;
                    }
                    if chars[i] == '\n' {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    i += 1;
                }
                raw.push(RawToken {
                    line: start_line,
                    col: start_col,
                    length: (i - start) as u32,
                    token_type: 0, // COMMENT
                });
                continue;
            }

            // Line comments: //
            if ch == '/' && i + 1 < len && chars[i + 1] == '/' {
                let start_line = line;
                let start_col = col;
                let start = i;
                while i < len && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
                raw.push(RawToken {
                    line: start_line,
                    col: start_col,
                    length: (i - start) as u32,
                    token_type: 0, // COMMENT
                });
                continue;
            }

            // String literals: "..."
            if ch == '"' {
                let start_line = line;
                let start_col = col;
                let start = i;
                i += 1;
                col += 1;
                while i < len && chars[i] != '"' && chars[i] != '\n' {
                    if chars[i] == '\\' && i + 1 < len {
                        i += 2;
                        col += 2;
                        continue;
                    }
                    i += 1;
                    col += 1;
                }
                if i < len && chars[i] == '"' {
                    i += 1;
                    col += 1;
                }
                raw.push(RawToken {
                    line: start_line,
                    col: start_col,
                    length: (i - start) as u32,
                    token_type: 2, // STRING
                });
                continue;
            }

            // Numeric literals (skip – not highlighted)
            if ch.is_ascii_digit() {
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_'
                    || chars[i] == '\'' || chars[i] == 'x' || chars[i] == 'z'
                    || chars[i] == 'h' || chars[i] == 'd' || chars[i] == 'o'
                    || chars[i] == 'b')
                {
                    i += 1;
                    col += 1;
                }
                continue;
            }

            // Identifiers / keywords / preprocessor directives
            if ch.is_ascii_alphabetic() || ch == '_' || ch == '`' || ch == '$' {
                let start_line = line;
                let start_col = col;
                let start = i;
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                    i += 1;
                    col += 1;
                }
                let word: String = chars[start..i].iter().collect();
                if keywords.contains(word.as_str()) {
                    raw.push(RawToken {
                        line: start_line,
                        col: start_col,
                        length: (i - start) as u32,
                        token_type: 1, // KEYWORD
                    });
                }
                continue;
            }

            // Skip everything else (operators, punctuation, etc.)
            i += 1;
            col += 1;
        }

        // Encode as relative-delta SemanticToken values
        let mut result: Vec<SemanticToken> = Vec::with_capacity(raw.len());
        let mut prev_line = 0u32;
        let mut prev_col = 0u32;

        for tok in &raw {
            let (dl, ds) = if tok.line == prev_line {
                (0u32, tok.col.saturating_sub(prev_col))
            } else {
                (tok.line.saturating_sub(prev_line), tok.col)
            };
            result.push(SemanticToken {
                delta_line: dl,
                delta_start: ds,
                length: tok.length,
                token_type: tok.token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = tok.line;
            prev_col = tok.col;
        }

        result
    }

    // ── Completion helpers ────────────────────────────────────────────────

    /// Scan source lines up to `line` to determine the current completion zone.
    fn determine_zone(content: &str, line: usize) -> CompletionZone {
        let mut module_depth: i32 = 0;
        let mut class_depth: i32 = 0;

        for (i, raw) in content.lines().enumerate() {
            if i > line {
                break;
            }
            let l = raw.to_lowercase();
            let words: Vec<&str> = l.split(|c: char| !c.is_alphanumeric() && c != '_').collect();

            for w in &words {
                match *w {
                    "module" => module_depth += 1,
                    "endmodule" => module_depth = (module_depth - 1).max(0),
                    "class" => class_depth += 1,
                    "endclass" => class_depth = (class_depth - 1).max(0),
                    _ => {}
                }
            }
        }

        if class_depth > 0 {
            return CompletionZone::ClassBody;
        }
        if module_depth > 0 {
            return CompletionZone::ModuleBody;
        }

        // One more heuristic: if any line up to the cursor contains `module`
        // but not `endmodule`, treat as module body.
        let lines: Vec<&str> = content.lines().collect();
        for idx in 0..=line.min(lines.len().saturating_sub(1)) {
            if let Some(l) = lines.get(idx) {
                let lower = l.to_lowercase();
                if lower.contains("module") && !lower.contains("endmodule") {
                    return CompletionZone::ModuleBody;
                }
            }
        }

        CompletionZone::ModuleLevel
    }

    fn module_level_completions() -> Vec<CompletionItem> {
        vec![
            Self::keyword_item("module"),
            Self::keyword_item("endmodule"),
            Self::keyword_item("input"),
            Self::keyword_item("output"),
            Self::keyword_item("inout"),
            Self::keyword_item("wire"),
            Self::keyword_item("reg"),
            Self::keyword_item("logic"),
            Self::keyword_item("assign"),
            Self::keyword_item("always_ff"),
            Self::keyword_item("always_comb"),
        ]
    }

    fn module_body_completions() -> Vec<CompletionItem> {
        vec![
            Self::keyword_item("endmodule"),
            Self::keyword_item("wire"),
            Self::keyword_item("reg"),
            Self::keyword_item("logic"),
            Self::keyword_item("assign"),
            Self::keyword_item("always_ff"),
            Self::keyword_item("always_comb"),
            Self::keyword_item("if"),
            Self::keyword_item("case"),
            Self::keyword_item("for"),
            Self::keyword_item("generate"),
            Self::keyword_item("input"),
            Self::keyword_item("output"),
        ]
    }

    fn class_body_completions() -> Vec<CompletionItem> {
        vec![
            Self::keyword_item("endclass"),
            Self::keyword_item("function"),
            Self::keyword_item("endfunction"),
            Self::keyword_item("task"),
            Self::keyword_item("endtask"),
            Self::keyword_item("virtual"),
            Self::keyword_item("new"),
            Self::keyword_item("this"),
            Self::keyword_item("super"),
            Self::keyword_item("return"),
            Self::keyword_item("extends"),
            Self::keyword_item("localparam"),
            Self::keyword_item("parameter"),
        ]
    }

    fn keyword_item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("SV keyword".to_string()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        }
    }

    // ── Hover helpers ─────────────────────────────────────────────────────

    /// Walk the graph (via Contains/Defines edges) looking for a node whose
    /// resolved name equals `symbol`.  Returns a human-readable markdown
    /// string on match.
    fn find_hover_info(
        graph: &InMemoryGraph,
        symbols: &SymbolTable,
        files: &HashMap<String, u64>,
        symbol: &str,
    ) -> Option<String> {
        for fid in files.values() {
            let mut visited = HashSet::new();
            let mut stack = vec![*fid];
            while let Some(nid) = stack.pop() {
                if !visited.insert(nid) {
                    continue;
                }
                if let Ok(Some(node)) = graph.get_node(nid) {
                    if let Some(info) = Self::format_node_info(&node, symbols, symbol) {
                        return Some(info);
                    }
                    if let Ok(edges) = graph.get_outgoing(nid) {
                        for e in &edges {
                            if e.edge_type == EdgeType::Contains || e.edge_type == EdgeType::Defines {
                                stack.push(e.target);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn format_node_info(node: &GraphNode, symbols: &SymbolTable, symbol: &str) -> Option<String> {
        let name = match &node.kind {
            NodeKind::Module { name } => symbols.resolve(*name),
            NodeKind::Class { name, .. } => symbols.resolve(*name),
            NodeKind::Package { name } => symbols.resolve(*name),
            NodeKind::Interface { name } => symbols.resolve(*name),
            NodeKind::ModulePort { name, .. } => symbols.resolve(*name),
            NodeKind::SignalDecl { name, .. } => symbols.resolve(*name),
            NodeKind::Function { name, .. } => symbols.resolve(*name),
            NodeKind::ModuleInstance { name, .. } => symbols.resolve(*name),
            NodeKind::Property { name } => symbols.resolve(*name),
            NodeKind::Method { name, .. } => symbols.resolve(*name),
            NodeKind::CoverGroup { name } => symbols.resolve(*name),
            NodeKind::CoverPoint { name } => symbols.resolve(*name),
            NodeKind::SequenceDecl { name } => symbols.resolve(*name),
            NodeKind::PropertyDecl { name } => symbols.resolve(*name),
            _ => None,
        }?;

        if name != symbol {
            return None;
        }

        Some(match &node.kind {
            NodeKind::Module { name: _ } => format!("module `{}`", name),
            NodeKind::Class { name: _, parent } => {
                if let Some(pid) = parent {
                    if let Some(pname) = symbols.resolve(*pid) {
                        format!("class `{}` extends `{}`", name, pname)
                    } else {
                        format!("class `{}`", name)
                    }
                } else {
                    format!("class `{}`", name)
                }
            }
            NodeKind::Package { name: _ } => format!("package `{}`", name),
            NodeKind::Interface { name: _ } => format!("interface `{}`", name),
            NodeKind::ModulePort { name: _, direction } => {
                let dir = match direction {
                    PortDirection::Input => "Input",
                    PortDirection::Output => "Output",
                    PortDirection::Inout => "Inout",
                    PortDirection::Ref => "Ref",
                };
                format!("port `{}` ({})", name, dir)
            }
            NodeKind::SignalDecl { name: _, kind } => {
                let sk = match kind {
                    SignalKind::Wire => "wire",
                    SignalKind::Reg => "reg",
                    SignalKind::Logic => "logic",
                    SignalKind::Integer => "integer",
                    SignalKind::Bit => "bit",
                };
                format!("signal `{}` ({})", name, sk)
            }
            NodeKind::Function { name: _, is_task } => {
                if *is_task {
                    format!("task `{}`", name)
                } else {
                    format!("function `{}`", name)
                }
            }
            NodeKind::ModuleInstance { name: _, module_type } => {
                if let Some(t) = symbols.resolve(*module_type) {
                    format!("instance `{}` : `{}`", name, t)
                } else {
                    format!("instance `{}`", name)
                }
            }
            _ => format!("symbol `{}`", name),
        })
    }

    /// Extract the word at the given (line, col) position from `content`.
    fn extract_word_at(content: &str, line: usize, col: usize) -> Option<String> {
        let line_str = content.lines().nth(line)?;
        let chars: Vec<char> = line_str.chars().collect();
        if col >= chars.len() {
            return None;
        }
        if !chars[col].is_alphanumeric() && chars[col] != '_' {
            return None;
        }
        let mut start = col;
        let mut end = col;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        Some(chars[start..end].iter().collect())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::COMMENT,
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::STRING,
                                SemanticTokenType::TYPE,
                                SemanticTokenType::FUNCTION,
                                SemanticTokenType::VARIABLE,
                            ],
                            token_modifiers: vec![],
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        range: None,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    }),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "hdl-graph-lsp".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "hdl-graph LSP server initialized. Building index...").await;

        let client = self.client.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            let index = Backend::build_index(".").await;
            if let Some(idx) = index {
                *state.write().await = Some(idx);
                client.log_message(MessageType::INFO, "Index built successfully.").await;
            } else {
                client.log_message(MessageType::WARNING, "Failed to build index.").await;
            }
        });
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let state = self.state.read().await;
        let state = match state.as_ref() {
            Some(s) => s,
            None => return Ok(None),
        };

        let uri = params.text_document_position_params.text_document.uri;
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let pos = params.text_document_position_params.position;
        let line = pos.line as usize;
        let col = pos.character as usize;

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };
        let symbol = match Self::extract_word_at(&content, line, col) {
            Some(s) => s,
            None => return Ok(None),
        };

        // Search for definitions across the whole graph
        for (file, fid) in &state.file_map {
            let mut visited = HashSet::new();
            let mut stack = vec![*fid];
            while let Some(nid) = stack.pop() {
                if !visited.insert(nid) {
                    continue;
                }
                if let Ok(Some(node)) = state.graph.get_node(nid) {
                    if node_matches_symbol(&node, &state.symbols, &symbol) {
                        let file_uri = match Url::from_file_path(
                            std::path::Path::new(".").join(file)
                        ) {
                            Ok(u) => u,
                            Err(_) => return Ok(None),
                        };
                        return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                            file_uri,
                            Range::new(Position::new(0, 0), Position::new(0, 0)),
                        ))));
                    }
                    if let Ok(edges) = state.graph.get_outgoing(nid) {
                        for e in &edges {
                            if e.edge_type == EdgeType::Contains || e.edge_type == EdgeType::Defines {
                                stack.push(e.target);
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    async fn references(&self, _params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        Ok(Some(vec![]))
    }

    // ── Semantic Tokens ─────────────────────────────────────────────────────
    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let data = Self::tokenize_semantic(&content);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    // ── Completions ─────────────────────────────────────────────────────────
    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
        let pos = params.text_document_position.position;
        let line = pos.line as usize;
        let col = pos.character as usize;

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let zone = Self::determine_zone(&content, line);

        let items = match zone {
            CompletionZone::ModuleLevel => Self::module_level_completions(),
            CompletionZone::ModuleBody => Self::module_body_completions(),
            CompletionZone::ClassBody => Self::class_body_completions(),
        };

        // Filter by prefix if the cursor is mid-word
        let line_str = content.lines().nth(line).unwrap_or("");
        let chars: Vec<char> = line_str.chars().collect();
        let prefix = if col > 0 && col <= chars.len() {
            let mut start = col.saturating_sub(1);
            while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
                start -= 1;
            }
            if start < col {
                Some(chars[start..col].iter().collect::<String>())
            } else {
                None
            }
        } else {
            None
        };

        let filtered: Vec<CompletionItem> = if let Some(p) = prefix {
            items.into_iter()
                .filter(|item| item.label.starts_with(&p))
                .collect()
        } else {
            items
        };

        Ok(Some(CompletionResponse::Array(filtered)))
    }

    // ── Hover ───────────────────────────────────────────────────────────────
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let state = self.state.read().await;
        let state = match state.as_ref() {
            Some(s) => s,
            None => return Ok(None),
        };

        let uri = params.text_document_position_params.text_document.uri;
        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };

        let pos = params.text_document_position_params.position;
        let line = pos.line as usize;
        let col = pos.character as usize;

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Ok(None),
        };

        let symbol = match Self::extract_word_at(&content, line, col) {
            Some(s) => s,
            None => return Ok(None),
        };

        if let Some(info) = Self::find_hover_info(&state.graph, &state.symbols, &state.file_map, &symbol) {
            let start_col = col.checked_sub(symbol.len()).unwrap_or(0);
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info,
                }),
                range: Some(Range::new(
                    Position::new(line as u32, start_col as u32),
                    Position::new(line as u32, col as u32),
                )),
            }));
        }

        Ok(None)
    }
}

fn node_matches_symbol(node: &GraphNode, symbols: &SymbolTable, symbol: &str) -> bool {
    match &node.kind {
        NodeKind::Module { name } => symbols.resolve(*name) == Some(symbol),
        NodeKind::Class { name, .. } => symbols.resolve(*name) == Some(symbol),
        NodeKind::Package { name } => symbols.resolve(*name) == Some(symbol),
        NodeKind::Interface { name } => symbols.resolve(*name) == Some(symbol),
        NodeKind::Function { name, .. } => symbols.resolve(*name) == Some(symbol),
        NodeKind::SignalDecl { name, .. } => symbols.resolve(*name) == Some(symbol),
        NodeKind::ModulePort { name, .. } => symbols.resolve(*name) == Some(symbol),
        NodeKind::ModuleInstance { name, .. } => symbols.resolve(*name) == Some(symbol),
        _ => false,
    }
}
