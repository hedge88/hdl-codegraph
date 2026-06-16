mod assertion;
mod dpi;
mod generate;
mod class;
mod package;
mod interface;
mod uvm_tlm;
mod uvm_config;
mod uvm_factory;
pub mod incremental;

use tree_sitter::{Node, Tree};
use hdl_graph_core::*;

/// Describes the difference between two parses of the same file.
///
/// In the initial implementation this uses a full-rebuild strategy:
/// all old nodes/edges are removed and all new nodes/edges are added.
/// A future version can compute a fine-grained diff when node IDs
/// are made stable across parses.
#[derive(Debug, Default)]
pub struct ChangeSet {
    /// New nodes to add: (node_id, node) pairs.
    pub added_nodes: Vec<(u64, GraphNode)>,
    /// IDs of nodes to remove from the graph.
    pub removed_node_ids: Vec<u64>,
    /// New edges to add.
    pub added_edges: Vec<Edge>,
    /// Edges to remove, identified by (source, target).
    pub removed_edges: Vec<(u64, u64)>,
}

impl ChangeSet {
    /// Apply this changeset to a graph.
    ///
    /// The application order is: remove old edges → remove old nodes → add new nodes → add new edges.
    /// Removal errors (e.g. node already gone) are silently ignored.
    pub fn apply_to<G: Graph>(&self, graph: &mut G) -> CoreResult<()> {
        for &(source, target) in &self.removed_edges {
            let _ = graph.remove_edge(source, target);
        }
        for &id in &self.removed_node_ids {
            let _ = graph.remove_node(id);
        }
        for &(_, ref node) in &self.added_nodes {
            graph.add_node(node.clone())?;
        }
        for edge in &self.added_edges {
            graph.add_edge(edge.clone())?;
        }
        Ok(())
    }

    /// Return true when this changeset contains no actual changes.
    pub fn is_empty(&self) -> bool {
        self.added_nodes.is_empty()
            && self.removed_node_ids.is_empty()
            && self.added_edges.is_empty()
            && self.removed_edges.is_empty()
    }
}

pub struct GraphExtractor {
    pub symbols: SymbolTable,
    next_id: u64,
    current_file_id: u32,
}

impl GraphExtractor {
    pub fn new() -> Self {
        Self {
            symbols: SymbolTable::new(),
            next_id: 1,
            current_file_id: 0,
        }
    }

    /// Create a GraphNode with source position captured from a tree-sitter node.
    fn make_node(&self, ts_node: Node, id: u64, kind: NodeKind, scope_id: Option<NodeId>) -> GraphNode {
        let pos = ts_node.start_position();
        GraphNode {
            id,
            kind,
            scope_id,
            file_id: self.current_file_id,
            line: pos.row as u32 + 1, // tree-sitter is 0-indexed; store as 1-indexed
            col: pos.column as u32,
        }
    }

    /// Extract graph nodes/edges from a parsed tree.
    /// source_bytes must be the original source text (needed for utf8_text).
    pub fn extract(
        &mut self,
        tree: &Tree,
        source: &[u8],
        file_id: u64,
    ) -> (Vec<GraphNode>, Vec<Edge>) {
        self.current_file_id = file_id as u32;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let root = tree.root_node();

        // Root SourceFile node
        let root_id = self.next_id();
        nodes.push(self.make_node(root, root_id, NodeKind::SourceFile, None));

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "module_declaration" => {
                    if let Some(mod_id) =
                        self.extract_module(child, source, &mut nodes, &mut edges)
                    {
                        edges.push(Edge {
                            source: root_id,
                            target: mod_id,
                            edge_type: EdgeType::Contains,
                        });
                    }
                }
                "config_declaration" => {
                    self.extract_config_declaration(child, source, root_id, &mut nodes, &mut edges);
                }
                "bind_directive" => {
                    self.extract_bind_directive(child, source, root_id, &mut nodes, &mut edges);
                }
                "package_declaration" => {
                    self.extract_package(child, source, root_id, &mut nodes, &mut edges);
                }
                "interface_declaration" => {
                    self.extract_interface(child, source, root_id, &mut nodes, &mut edges);
                }
                "class_declaration" => {
                    if let Some(cls_id) = self.extract_class(child, source, root_id, &mut nodes, &mut edges) {
                        edges.push(Edge { source: root_id, target: cls_id, edge_type: EdgeType::Contains });
                    }
                }
                _ => {}
            }
        }

        (nodes, edges)
    }

    /// Compute the changeset between a previously-extracted file and freshly parsed content.
    ///
    /// For Phase 1 this performs a full rebuild: all nodes/edges from the old parse are
    /// marked for removal and all nodes/edges from the new parse are returned as additions.
    ///
    /// `old_node_ids` must be the node IDs that were previously added to the graph for this file,
    /// and `old_edges` must be the corresponding edges. These are typically stored by the caller
    /// during the initial index pass.
    // (extract_changeset is defined in incremental.rs)

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn text<'a>(&self, node: Node<'a>, source: &'a [u8]) -> &'a str {
        node.utf8_text(source).unwrap_or("")
    }

    fn extract_module(
        &mut self,
        node: Node,
        source: &[u8],
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) -> Option<u64> {
        let mut module_name = String::new();
        let module_id = self.next_id();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "module_header" | "module_nonansi_header" | "module_ansi_header" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        module_name = self.text(name_node, source).to_string();
                    }
                    let mut hc = child.walk();
                    for hchild in child.children(&mut hc) {
                        if hchild.kind() == "list_of_ports"
                            || hchild.kind() == "list_of_port_declarations"
                        {
                            self.extract_ports(hchild, source, module_id, nodes, edges);
                        }
                    }
                }
                // Module body items — grammar inlines them as direct children of module_declaration
                "net_declaration" | "data_declaration" => {
                    if self.detect_tlm_port(child, source).is_some() {
                        self.extract_tlm_port(child, source, module_id, nodes, edges);
                    } else {
                        self.extract_signal(child, source, module_id, nodes, edges);
                    }
                    self.extract_factory_registration(child, source, module_id, nodes, edges);
                }
                "always_construct" => {
                    self.extract_always(child, source, module_id, nodes, edges);
                    self.extract_method_calls_in_subtree(child, source, module_id, nodes, edges);
                }
                "initial_construct" | "final_construct" => {
                    self.extract_method_calls_in_subtree(child, source, module_id, nodes, edges);
                }
                "module_instantiation" | "udp_instantiation" | "gate_instantiation" | "interface_instantiation" => {
                    self.extract_instance(child, source, module_id, nodes, edges);
                }
                "continuous_assign" => {
                    let assign_id = self.next_id();
                    nodes.push(self.make_node(child, assign_id, NodeKind::Assignment, None));
                    edges.push(Edge {
                        source: module_id,
                        target: assign_id,
                        edge_type: EdgeType::Contains,
                    });
                }
                "function_declaration" | "task_declaration" => {
                    let func_name = child.child_by_field_name("name")
                        .map(|n| self.text(n, source).to_string())
                        .or_else(|| {
                            // Try nested path: function_body_declaration → name
                            let mut c = child.walk();
                            for gc in child.children(&mut c) {
                                if gc.kind() == "function_body_declaration" || gc.kind() == "function_prototype" {
                                    if let Some(nn) = gc.child_by_field_name("name") {
                                        return Some(self.text(nn, source).to_string());
                                    }
                                }
                            }
                            None
                        })
                        .or_else(|| {
                            // Try task_body_declaration
                            let mut c = child.walk();
                            for gc in child.children(&mut c) {
                                if gc.kind() == "task_body_declaration" || gc.kind() == "task_prototype" {
                                    if let Some(nn) = gc.child_by_field_name("name") {
                                        return Some(self.text(nn, source).to_string());
                                    }
                                }
                            }
                            None
                        })
                        .unwrap_or_default();

                    if !func_name.is_empty() {
                        let name_sym = self.symbols.intern(&func_name);
                        let func_id = self.next_id();
                        let is_task = child.kind() == "task_declaration";
                        nodes.push(self.make_node(child, func_id, NodeKind::Function { name: name_sym, is_task }, None));
                        edges.push(Edge { source: module_id, target: func_id, edge_type: EdgeType::Contains });
                        // Scan function/task body for factory calls and other method calls
                        self.extract_method_calls_in_subtree(child, source, func_id, nodes, edges);
                    }
                }
                // Generate constructs
                "loop_generate_construct" => {
                    self.extract_generate_for(child, source, module_id, nodes, edges);
                }
                "conditional_generate_construct" => {
                    self.extract_generate_if(child, source, module_id, nodes, edges);
                }
                "generate_region" => {
                    self.extract_generate_region(child, source, module_id, nodes, edges);
                }
                // Assertion constructs
                "concurrent_assertion_item" => {
                    self.extract_concurrent_assertion_item(child, source, module_id, nodes, edges);
                }
                "covergroup_declaration" => {
                    self.extract_covergroup_declaration(child, source, module_id, nodes, edges);
                }
                "property_declaration" => {
                    self.extract_property_declaration(child, source, module_id, nodes, edges);
                }
                "sequence_declaration" => {
                    self.extract_sequence_declaration(child, source, module_id, nodes, edges);
                }
                // DPI-C / Bind / Config
                "dpi_import_export" => {
                    self.extract_dpi_import(child, source, module_id, nodes, edges);
                }
                "bind_directive" => {
                    self.extract_bind_directive(child, source, module_id, nodes, edges);
                }
                "config_declaration" => {
                    self.extract_config_declaration(child, source, module_id, nodes, edges);
                }
                "module_item" => {
                    self.extract_module_body(child, source, module_id, nodes, edges);
                }
                // OOP and interface constructs inside modules
                "class_declaration" => {
                    self.extract_class(child, source, module_id, nodes, edges);
                }
                "interface_declaration" => {
                    self.extract_interface(child, source, module_id, nodes, edges);
                }
                "module_keyword" | "simple_identifier" | "end_of_module_identifier" => {}
                _ => {}
            }
        }

        if module_name.is_empty() {
            return None;
        }

        let name_sym = self.symbols.intern(&module_name);
        nodes.push(self.make_node(node, module_id, NodeKind::Module { name: name_sym }, None));

        Some(module_id)
    }

    fn extract_ports(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "port_declaration" | "ansi_port_declaration" => {
                    self.extract_port(child, source, module_id, nodes, edges);
                }
                _ => {}
            }
        }
    }

    fn extract_port(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut direction = PortDirection::Inout;
        let mut port_name = String::new();

        // For ansi_port_declaration, use field name
        if node.kind() == "ansi_port_declaration" {
            if let Some(name_node) = node.child_by_field_name("port_name") {
                port_name = self.text(name_node, source).to_string();
            }
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "port_direction" => {
                        let mut dc = child.walk();
                        for dchild in child.children(&mut dc) {
                            let t = self.text(dchild, source);
                            if t == "input" { direction = PortDirection::Input; }
                            else if t == "output" { direction = PortDirection::Output; }
                            else if t == "inout" { direction = PortDirection::Inout; }
                        }
                    }
                    _ => {}
                }
            }
        } else {
            // For non-ANSI port_declaration
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "input" => direction = PortDirection::Input,
                    "output" => direction = PortDirection::Output,
                    "inout" => direction = PortDirection::Inout,
                    "port_direction" => {
                        let mut dc = child.walk();
                        for dchild in child.children(&mut dc) {
                            let t = self.text(dchild, source);
                            if t == "input" { direction = PortDirection::Input; }
                            else if t == "output" { direction = PortDirection::Output; }
                            else if t == "inout" { direction = PortDirection::Inout; }
                        }
                    }
                    "simple_identifier" => {
                        port_name = self.text(child, source).to_string();
                    }
                    _ => {}
                }
            }
        }

        if !port_name.is_empty() {
            let name_sym = self.symbols.intern(&port_name);
            let port_id = self.next_id();
            nodes.push(self.make_node(node, port_id, NodeKind::ModulePort {
                    name: name_sym,
                    direction,
                }, None));
            edges.push(Edge {
                source: module_id,
                target: port_id,
                edge_type: EdgeType::Defines,
            });
        }
    }

    fn extract_module_body(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "net_declaration" | "data_declaration" => {
                    if self.detect_tlm_port(child, source).is_some() {
                        self.extract_tlm_port(child, source, module_id, nodes, edges);
                    } else {
                        self.extract_signal(child, source, module_id, nodes, edges);
                    }
                    self.extract_factory_registration(child, source, module_id, nodes, edges);
                }
                "always_construct" => {
                    self.extract_always(child, source, module_id, nodes, edges);
                    self.extract_method_calls_in_subtree(child, source, module_id, nodes, edges);
                }
                "initial_construct" | "final_construct" => {
                    self.extract_method_calls_in_subtree(child, source, module_id, nodes, edges);
                }
                "module_instantiation" | "udp_instantiation" => {
                    self.extract_instance(child, source, module_id, nodes, edges);
                }
                "continuous_assign" => {
                    let assign_id = self.next_id();
                    nodes.push(self.make_node(child, assign_id, NodeKind::Assignment, None));
                    edges.push(Edge {
                        source: module_id,
                        target: assign_id,
                        edge_type: EdgeType::Contains,
                    });
                }
                // Generate constructs
                "loop_generate_construct" => {
                    self.extract_generate_for(child, source, module_id, nodes, edges);
                }
                "conditional_generate_construct" => {
                    self.extract_generate_if(child, source, module_id, nodes, edges);
                }
                "generate_region" => {
                    self.extract_generate_region(child, source, module_id, nodes, edges);
                }
                // Assertion constructs
                "concurrent_assertion_item" => {
                    self.extract_concurrent_assertion_item(child, source, module_id, nodes, edges);
                }
                "covergroup_declaration" => {
                    self.extract_covergroup_declaration(child, source, module_id, nodes, edges);
                }
                "property_declaration" => {
                    self.extract_property_declaration(child, source, module_id, nodes, edges);
                }
                "sequence_declaration" => {
                    self.extract_sequence_declaration(child, source, module_id, nodes, edges);
                }
                // DPI-C / Bind / Config
                "dpi_import_export" => {
                    self.extract_dpi_import(child, source, module_id, nodes, edges);
                }
                "bind_directive" => {
                    self.extract_bind_directive(child, source, module_id, nodes, edges);
                }
                "config_declaration" => {
                    self.extract_config_declaration(child, source, module_id, nodes, edges);
                }
                // OOP and interface constructs inside module items
                "class_declaration" => {
                    self.extract_class(child, source, module_id, nodes, edges);
                }
                "interface_declaration" => {
                    self.extract_interface(child, source, module_id, nodes, edges);
                }
                _ => {}
            }
        }
    }

    fn extract_signal(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let kind = self.detect_signal_kind(node, source);
        for name in &self.collect_signal_names(node, source) {
            let name_sym = self.symbols.intern(name);
            let sig_id = self.next_id();
            nodes.push(self.make_node(node, sig_id, NodeKind::SignalDecl {
                    name: name_sym,
                    kind: kind.clone(),
                }, None));
            edges.push(Edge {
                source: module_id,
                target: sig_id,
                edge_type: EdgeType::Defines,
            });
        }
    }

    /// Walk the subtree looking for the signal type keyword.
    fn detect_signal_kind(&self, node: Node, source: &[u8]) -> SignalKind {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let t = self.text(child, source);
            if t == "wire" {
                return SignalKind::Wire;
            }
            if t == "reg" {
                return SignalKind::Reg;
            }
            if t == "logic" || t == "bit" || t == "integer" {
                return match t {
                    "bit" | "integer" => SignalKind::Integer,
                    _ => SignalKind::Logic,
                };
            }
            if child.kind() == "net_type" || child.kind() == "integer_vector_type"
                || child.kind() == "integer_atom_type"
            {
                let inner = self.detect_signal_kind(child, source);
                if !matches!(inner, SignalKind::Logic) {
                    return inner;
                }
            }
        }
        SignalKind::Logic
    }

    /// Recursively collect user-defined signal names from a declaration.
    /// Skips type/dimension nodes to avoid false positives.
    fn collect_signal_names(&self, node: Node, source: &[u8]) -> Vec<String> {
        let mut names = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "simple_identifier" | "escaped_identifier" => {
                    names.push(self.text(child, source).to_string());
                }
                // Sub-trees that may contain type/expression identifiers — skip
                "data_type_or_implicit" | "implicit_data_type" | "data_type"
                | "net_type" | "integer_vector_type" | "integer_atom_type"
                | "non_integer_type" | "signing" | "delay3" | "delay_control"
                | "delay_value" | "drive_strength" | "charge_strength"
                | "unpacked_dimension" | "constant_expression" | "constant_mintypmax_expression"
                | "parameter_value_assignment" | "ordered_parameter_assignment"
                | "named_parameter_assignment" => {}
                // Everything else — recurse
                _ => {
                    names.extend(self.collect_signal_names(child, source));
                }
            }
        }
        names
    }

    fn extract_always(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut kind = AlwaysKind::Combinational;
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "always_keyword" => {
                    let t = self.text(child, source);
                    if t == "always_ff" {
                        kind = AlwaysKind::Sequential;
                    } else if t == "always_latch" {
                        kind = AlwaysKind::Latch;
                    }
                }
                _ => {}
            }
        }
        let al_id = self.next_id();
        nodes.push(self.make_node(node, al_id, NodeKind::AlwaysBlock { kind }, None));
        edges.push(Edge {
            source: module_id,
            target: al_id,
            edge_type: EdgeType::Contains,
        });
    }

    fn extract_instance(
        &mut self,
        node: Node,
        source: &[u8],
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // Module type is in the instance_type field
        let module_type = node.child_by_field_name("instance_type")
            .map(|n| self.text(n, source).to_string())
            .unwrap_or_default();

        if module_type.is_empty() {
            return;
        }

        // Instance name is inside hierarchical_instance → name_of_instance
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "hierarchical_instance" {
                self.extract_hierarchical_instance(child, source, &module_type, module_id, nodes, edges);
            }
        }
    }

    fn extract_hierarchical_instance(
        &mut self,
        node: Node,
        source: &[u8],
        module_type: &str,
        module_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "name_of_instance" {
                if let Some(name_node) = child.child_by_field_name("instance_name") {
                    let instance_name = self.text(name_node, source);
                    let type_sym = self.symbols.intern(module_type);
                    let name_sym = self.symbols.intern(instance_name);
                    let inst_id = self.next_id();
                    nodes.push(self.make_node(node, inst_id, NodeKind::ModuleInstance { name: name_sym, module_type: type_sym }, None));
                    edges.push(Edge { source: module_id, target: inst_id, edge_type: EdgeType::Contains });
                }
            }
        }
    }

    /// Recursively walk a subtree looking for `method_call` and `tf_call` nodes.
    /// When found, dispatches to the appropriate handler
    /// (tlm_connect, config_db_set, config_db_get, factory, etc.).
    fn extract_method_calls_in_subtree(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "method_call" => {
                    self.extract_method_call(child, source, parent_id, nodes, edges);
                }
                "tf_call" => {
                    self.extract_tf_call(child, source, parent_id, nodes, edges);
                }
                // Recurse into child nodes that can contain method calls
                _ => {
                    self.extract_method_calls_in_subtree(child, source, parent_id, nodes, edges);
                }
            }
        }
    }

    /// Dispatch a method_call node (dot syntax: obj.method()) to the appropriate handler.
    fn extract_method_call(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        let mut method_name = String::new();

        {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "method_call_body" {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        method_name = self.text(name_node, source).to_string();
                    }
                }
            }
        }

        let method_str: &str = &method_name;
        match method_str {
            "connect" => {
                self.extract_tlm_connect(node, source, parent_id, nodes, edges);
            }
            "set" => {
                self.extract_config_db_set(node, source, parent_id, nodes, edges);
            }
            "get" => {
                self.extract_config_db_get(node, source, parent_id, nodes, edges);
            }
            "create" => {
                self.extract_factory_create(node, source, parent_id, nodes, edges);
            }
            "set_type_override" | "set_inst_override" => {
                self.extract_factory_override(node, source, parent_id, nodes, edges);
            }
            _ => {}
        }
    }

    /// Dispatch a tf_call node (scope-resolution syntax: Class::method()) to
    /// the appropriate handler.
    fn extract_tf_call(
        &mut self,
        node: Node,
        source: &[u8],
        parent_id: u64,
        nodes: &mut Vec<GraphNode>,
        edges: &mut Vec<Edge>,
    ) {
        // A tf_call typically has one or more package_scope children (for ::
        // prefixes) and a simple_identifier (for the method name).
        // Collect ALL scope fragments, then the last identifier is the method.
        let (scope_fragments, method_name) = self.extract_tf_call_name(node, source);
        if method_name.is_empty() {
            return;
        }

        match method_name.as_str() {
            "create" => {
                // Only handle when it looks like type_id::create(...)
                if scope_fragments.last().map(|s| s.as_str()) == Some("type_id") {
                    self.extract_factory_create_via_tf(node, source, &scope_fragments, parent_id, nodes, edges);
                }
            }
            "set_type_override" | "set_inst_override" => {
                self.extract_factory_override_via_tf(node, source, &scope_fragments, parent_id, nodes, edges);
            }
            "set" => {
                self.extract_config_db_set(node, source, parent_id, nodes, edges);
            }
            "get" => {
                self.extract_config_db_get(node, source, parent_id, nodes, edges);
            }
            _ => {}
        }
    }

    /// Extract the scope fragments and method name from a tf_call node.
    /// For `type_id::create(...)`, returns (["type_id"], "create").
    /// For `driver_type::type_id::create(...)`, returns (["driver_type", "type_id"], "create").
    /// For `uvm_config_db#(int)::set(...)`, returns (["uvm_config_db#(int)"], "set").
    fn extract_tf_call_name(
        &self,
        node: Node,
        source: &[u8],
    ) -> (Vec<String>, String) {
        let mut scope: Vec<String> = Vec::new();
        let mut method_name = String::new();

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "package_scope" => {
                    // package_scope text includes the trailing "::" — strip it
                    let raw = self.text(child, source);
                    let cleaned = raw.trim_end_matches(':').trim().to_string();
                    if !cleaned.is_empty() {
                        scope.push(cleaned);
                    }
                }
                "simple_identifier" | "escaped_identifier" => {
                    // In tf_call, the last identifier child is the method name.
                    // If we see an identifier here, it's the target of the
                    // preceding package_scope (e.g. driver_type::type_id →
                    // package_scope="driver_type::" then identifier="type_id").
                    // But it could also be the method name at the end.
                    // We'll push identifiers as scope parts, then the final
                    // identifier becomes method_name.
                    method_name = self.text(child, source).to_string();
                }
                _ => {}
            }
        }

        (scope, method_name)
    }
}
