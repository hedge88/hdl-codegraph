use tree_sitter::Node;
use hdl_graph_core::*;

impl super::GraphExtractor {
    // ------------------------------------------------------------------
    // Factory Registration
    // ------------------------------------------------------------------

    /// Extract factory registration from a data_declaration node containing
    /// `typedef uvm_component_registry #(TYPE, "TYPE") type_id;` or
    /// `typedef uvm_object_registry #(TYPE, "TYPE") type_id;`.
    ///
    /// These typedefs are produced by the preprocessor when expanding
    /// `uvm_component_utils`, `uvm_object_utils`, and similar macros.
    pub fn extract_factory_registration(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let decl_text = self.text(node, source);

        // Determine which registry type is used
        let base_type = if decl_text.contains("uvm_component_registry") {
            "uvm_component"
        } else if decl_text.contains("uvm_object_registry") {
            "uvm_object"
        } else {
            return;
        };

        // Extract the type parameter from #(TYPE, "TYPE")
        let type_name = match Self::parse_registry_type_name(decl_text) {
            Some(name) => name,
            None => return,
        };

        let type_sym = self.symbols.intern(&type_name);
        let base_sym = self.symbols.intern(base_type);
        let reg_id = self.next_id();
        nodes.push(self.make_node(node, reg_id, NodeKind::FactoryReg {
                type_name: type_sym,
                base_type: base_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: reg_id,
            edge_type: EdgeType::FactoryRegisters,
        });
    }

    // ------------------------------------------------------------------
    // Factory Create (type_id::create(...))
    // ------------------------------------------------------------------

    /// Check if a method_call node is a `type_id::create(...)` invocation.
    /// Returns the type name (the scope before `::type_id`) if it matches.
    pub fn is_factory_create(&self, node: Node, source: &[u8]) -> Option<String> {
        let mut method_name = String::new();
        let mut class_type_text = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "class_type" => {
                    class_type_text = self.text(child, source).to_string();
                }
                "method_call_body" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method_name = self.text(name_node, source).to_string();
                    }
                }
                _ => {}
            }
        }

        if method_name != "create" {
            return None;
        }

        // The receiver should end with "::type_id" (possibly followed by trailing colons).
        // Handle both "my_driver::type_id" and "my_driver::type_id::" patterns.
        let cleaned = class_type_text.trim_end_matches(':').trim().to_string();
        if cleaned == "type_id" {
            // Plain "type_id" with no scope prefix — cannot determine the type
            return None;
        }
        if let Some(prefix) = cleaned.strip_suffix("::type_id") {
            let trimmed = prefix.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }

        None
    }

    /// Extract a `type_id::create(...)` invocation.
    ///
    /// Creates a FactoryCreate node under the parent scope.
    pub fn extract_factory_create(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let type_name = match self.is_factory_create(node, source) {
            Some(name) => name,
            None => return,
        };

        let type_sym = self.symbols.intern(&type_name);
        let create_id = self.next_id();
        nodes.push(self.make_node(node, create_id, NodeKind::FactoryCreate {
                type_name: type_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: create_id,
            edge_type: EdgeType::Contains,
        });
    }

    // ------------------------------------------------------------------
    // Factory Override (set_type_override / set_inst_override)
    // ------------------------------------------------------------------

    /// Check if a method_call node is a `set_type_override(...)` or
    /// `set_inst_override(...)` invocation.
    /// Returns (original_type, override_type) if it matches.
    pub fn is_factory_override(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<(String, String)> {
        let method_name = {
            let mut cursor = node.walk();
            let mut name = String::new();
            for child in node.children(&mut cursor) {
                if child.kind() == "method_call_body" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        name = self.text(name_node, source).to_string();
                    }
                }
            }
            name
        };

        if method_name != "set_type_override" && method_name != "set_inst_override" {
            return None;
        }

        self.extract_override_args(node, source)
    }

    /// Extract a `set_type_override(...)` or `set_inst_override(...)` invocation.
    ///
    /// Creates a FactoryOverride node under the parent scope.
    pub fn extract_factory_override(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let (original_type, override_type) = match self.is_factory_override(node, source) {
            Some((o, r)) => (o, r),
            None => return,
        };

        let orig_sym = self.symbols.intern(&original_type);
        let over_sym = self.symbols.intern(&override_type);
        let ovr_id = self.next_id();
        nodes.push(self.make_node(node, ovr_id, NodeKind::FactoryOverride {
                original_type: orig_sym,
                override_type: over_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: ovr_id,
            edge_type: EdgeType::FactoryOverrides,
        });
    }

    // ------------------------------------------------------------------
    // Helper methods
    // ------------------------------------------------------------------

    /// Parse the first type parameter from `#(TYPE, "TYPE")` or `#(TYPE, ...)`.
    fn parse_registry_type_name(decl_text: &str) -> Option<String> {
        let paren_start = decl_text.find("#(")?;
        let after_paren = &decl_text[paren_start + 2..];

        let mut depth: i32 = 1;
        let mut end = 0;
        for (i, ch) in after_paren.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            return None;
        }

        let params = &after_paren[..end];

        // First parameter: split by commas at depth 0
        let mut depth2: i32 = 0;
        let first_param: String = params
            .chars()
            .take_while(|&ch| {
                match ch {
                    '(' | '[' | '{' => {
                        depth2 += 1;
                        true
                    }
                    ')' | ']' | '}' => {
                        depth2 -= 1;
                        true
                    }
                    ',' if depth2 == 0 => false,
                    _ => true,
                }
            })
            .collect();

        let trimmed = first_param.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    }

    /// Extract the original_type and override_type arguments from a
    /// set_type_override / set_inst_override method_call node.
    fn extract_override_args(
        &self,
        node: Node,
        source: &[u8],
    ) -> Option<(String, String)> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "method_call_body" {
                let args = self.collect_args(child, source);
                return Self::clean_override_args(&args);
            }
        }
        None
    }

    /// Collect argument text from a method_call_body's list_of_arguments.
    fn collect_args(&self, method_call_body: Node, source: &[u8]) -> Vec<String> {
        let mut args = Vec::new();
        let mut mc = method_call_body.walk();
        for child in method_call_body.children(&mut mc) {
            if child.kind() == "list_of_arguments" {
                let mut ac = child.walk();
                for arg_child in child.children(&mut ac) {
                    let kind = arg_child.kind();
                    // Skip punctuation-only children
                    if kind == "," || kind == "(" || kind == ")" || kind == ";" {
                        continue;
                    }
                    args.push(self.text(arg_child, source).to_string());
                }
            }
        }
        args
    }

    /// Given collected argument strings, extract original_type and
    /// override_type.  Handles patterns like:
    ///   - `my_driver::get_type()` → `my_driver`
    ///   - `"my_driver"` → `my_driver` (strip quotes)
    fn clean_override_args(args: &[String]) -> Option<(String, String)> {
        if args.len() < 2 {
            return None;
        }

        let first = Self::clean_arg(&args[0]);
        let second = Self::clean_arg(&args[1]);

        if first.is_empty() || second.is_empty() {
            return None;
        }

        Some((first, second))
    }

    /// Handle a `type_id::create(...)` call parsed as a tf_call node.
    /// Extracts the type name from the scope fragments and delegates to
    /// extract_factory_create.
    pub fn extract_factory_create_via_tf(
        &mut self,
        _node: Node,
        _source: &[u8],
        scope_fragments: &[String],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // The type name is everything before the final "type_id" fragment.
        let type_name = if scope_fragments.len() >= 2 {
            // e.g. ["driver_type", "type_id"] → "driver_type"
            scope_fragments[0..scope_fragments.len() - 1].join("::")
        } else if scope_fragments.len() == 1 {
            // e.g. ["type_id"] → no explicit type prefix
            return;
        } else {
            return;
        };

        if type_name.is_empty() {
            return;
        }

        let type_sym = self.symbols.intern(&type_name);
        let create_id = self.next_id();
        nodes.push(self.make_node(_node, create_id, NodeKind::FactoryCreate {
                type_name: type_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: create_id,
            edge_type: EdgeType::Contains,
        });
    }

    /// Handle a set_type_override / set_inst_override call parsed as a tf_call.
    /// tf_call nodes have list_of_arguments as a direct child (no method_call_body).
    ///
    /// Extracts original_type and override_type from the arguments,
    /// optionally using scope_fragments for the original type.
    pub fn extract_factory_override_via_tf(
        &mut self,
        node: Node,
        source: &[u8],
        scope_fragments: &[String],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let args = Self::collect_tf_call_args(node, source);

        // Try scope fragments first for original_type
        if scope_fragments.len() >= 2
            && scope_fragments.last().map(|s| s.as_str()) == Some("type_id")
        {
            let original_type = scope_fragments[..scope_fragments.len() - 1].join("::");
            let override_type = if !args.is_empty() {
                Self::clean_arg(&args[0])
            } else {
                String::new()
            };
            if !original_type.is_empty() && !override_type.is_empty() {
                let orig_sym = self.symbols.intern(&original_type);
                let over_sym = self.symbols.intern(&override_type);
                let ovr_id = self.next_id();
                nodes.push(self.make_node(node, ovr_id, NodeKind::FactoryOverride {
                        original_type: orig_sym,
                        override_type: over_sym,
                    }, None));
                edges.push(Edge {
                    source: parent_id,
                    target: ovr_id,
                    edge_type: EdgeType::FactoryOverrides,
                });
                return;
            }
        }

        // Fall back: extract both from arguments
        let (original_type, override_type) = match Self::clean_override_args(&args) {
            Some(pair) => pair,
            None => return,
        };

        if original_type.is_empty() || override_type.is_empty() {
            return;
        }

        let orig_sym = self.symbols.intern(&original_type);
        let over_sym = self.symbols.intern(&override_type);
        let ovr_id = self.next_id();
        nodes.push(self.make_node(node, ovr_id, NodeKind::FactoryOverride {
                original_type: orig_sym,
                override_type: over_sym,
            }, None));
        edges.push(Edge {
            source: parent_id,
            target: ovr_id,
            edge_type: EdgeType::FactoryOverrides,
        });
    }

    /// Collect argument text from a tf_call's direct list_of_arguments child.
    fn collect_tf_call_args(node: Node, source: &[u8]) -> Vec<String> {
        let mut args = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "list_of_arguments" {
                let mut ac = child.walk();
                for arg_child in child.children(&mut ac) {
                    let kind = arg_child.kind();
                    if kind == "," || kind == "(" || kind == ")" || kind == ";" {
                        continue;
                    }
                    if let Ok(text) = arg_child.utf8_text(source) {
                        args.push(text.to_string());
                    }
                }
            }
        }
        args
    }

    /// Clean a single argument: strip `::get_type()` suffix and/or quotes.
    fn clean_arg(arg: &str) -> String {
        let trimmed = arg.trim();
        if trimmed.is_empty() {
            return String::new();
        }

        // Handle `T::get_type()` → T
        if let Some(pos) = trimmed.find("::get_type") {
            let before = &trimmed[..pos];
            return before.trim().to_string();
        }

        // Strip surrounding quotes
        let stripped = trimmed
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(trimmed);

        stripped.trim().to_string()
    }
}
