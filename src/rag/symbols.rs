//! Extract defines and imports from source (Python via tree-sitter). Parity with utils/ast_extract.py.
//! Rust: extract_rust_qualified_symbols, extract_calls. Python: extract_python (defines + imports). TypeScript/JavaScript: extract_ts_js, extract_calls_ts_js. generate_module_graph: walk Rust mod/use → Mermaid.

use std::collections::{HashSet, VecDeque};
use std::path::Path;
use tree_sitter::Node;
use tree_sitter_python::LANGUAGE;
use tree_sitter_typescript::{LANGUAGE_TSX, LANGUAGE_TYPESCRIPT};
use walkdir::WalkDir;

/// Defines (symbol names) and imports (module paths) from one source file. Used for symbol_index and chunk metadata.
#[derive(Debug, Default)]
/// SymbolExtraction.
pub struct SymbolExtraction {
    pub defines: Vec<String>,
    pub imports: Vec<String>,
}

impl SymbolExtraction {
    /// Serialize defines to JSON array string for chunk metadata.
    pub fn as_defines_json(&self) -> String {
        serde_json::to_string(&self.defines).unwrap_or_else(|_| "[]".to_string())
    }
    /// Serialize imports to JSON array string for chunk metadata.
    pub fn as_imports_json(&self) -> String {
        serde_json::to_string(&self.imports).unwrap_or_else(|_| "[]".to_string())
    }
}

/// Extracts text for node's byte range from source; UTF-8 lossy.
fn node_text(node: Node, source: &[u8]) -> String {
    let r = node.byte_range();
    source
        .get(r.start..r.end)
        .map(|s| String::from_utf8_lossy(s).to_string())
        .unwrap_or_default()
}

/// Pushes first component of dotted_name and aliased_import names from an import statement node into imports.
fn push_imports_from_import_statement(node: Node, source: &[u8], imports: &mut Vec<String>) {
    for i in 0..node.child_count() {
        let c = node.child(i).unwrap();
        if c.kind() == "dotted_name" {
            let name = node_text(c, source);
            let first = name.split('.').next().unwrap_or("").to_string();
            if !first.is_empty() && first != "import" {
                imports.push(first);
            }
        } else if c.kind() == "aliased_import" {
            if let Some(name_node) = c.child_by_field_name("name") {
                let name = node_text(name_node, source);
                let first = name.split('.').next().unwrap_or("").to_string();
                if !first.is_empty() {
                    imports.push(first);
                }
            }
        }
    }
}

/// Extract defines and imports from Python source.
pub fn extract_python(source: &str) -> SymbolExtraction {
    let mut defines = Vec::new();
    let mut imports = Vec::new();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&LANGUAGE.into()).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return SymbolExtraction { defines, imports },
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    /// walk.
    fn walk(node: Node, source: &[u8], defines: &mut Vec<String>, imports: &mut Vec<String>) {
        match node.kind() {
            "function_definition" | "class_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let s = node_text(name_node, source);
                    if !s.is_empty() {
                        defines.push(s);
                    }
                }
            }
            "decorated_definition" => {
                if let Some(inner) = node.child_by_field_name("definition") {
                    walk(inner, source, defines, imports);
                }
            }
            "import_statement" => push_imports_from_import_statement(node, source, imports),
            "import_from_statement" => {
                if let Some(module) = node.child_by_field_name("module_name") {
                    let name = node_text(module, source);
                    let first = name.split('.').next().unwrap_or("").to_string();
                    if !first.is_empty() {
                        imports.push(first);
                    }
                }
                for i in 0..node.child_count() {
                    let c = node.child(i).unwrap();
                    if c.kind() == "dotted_name" {
                        let name = node_text(c, source);
                        let first = name.split('.').next().unwrap_or("").to_string();
                        if !first.is_empty() {
                            imports.push(first);
                        }
                    }
                }
            }
            _ => {}
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk(c, source, defines, imports);
            }
        }
    }

    walk(root, bytes, &mut defines, &mut imports);

    defines.sort_unstable();
    defines.dedup();
    imports.retain(|s| !s.is_empty() && s != "import");
    imports.sort_unstable();
    imports.dedup();
    SymbolExtraction { defines, imports }
}

/// Extract defines and imports from Rust source. Defines are qualified (e.g. mod::Struct::method); imports are crate/mod paths.
pub fn extract_rust(source: &str) -> SymbolExtraction {
    let mut defines = Vec::new();
    let mut imports = Vec::new();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return SymbolExtraction { defines, imports },
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    let mut context_stack = Vec::new();
    /// qualified_name.
    fn qualified_name(stack: &[String], name: &str) -> String {
        if stack.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", stack.join("::"), name)
        }
    }

    /// Extract imported names from a Rust use_declaration (e.g. use std::collections::HashMap; or use a::b::{X, Y};).
    fn push_rust_use_imports(node: Node, source: &[u8], imports: &mut Vec<String>) {
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                match c.kind() {
                    "scoped_identifier" | "identifier" | "path" => {
                        let path_str = node_text(c, source).trim().to_string();
                        if !path_str.is_empty() {
                            imports.push(path_str.clone());
                            if let Some(last) =
                                path_str.split("::").filter(|s| !s.is_empty()).last()
                            {
                                imports.push(last.to_string());
                            }
                        }
                    }
                    "use_list" => {
                        for j in 0..c.child_count() {
                            if let Some(item) = c.child(j) {
                                let text = node_text(item, source).trim().to_string();
                                if !text.is_empty() && text != "{" && text != "}" {
                                    imports.push(text.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    /// walk_rs.
    fn walk_rs(
        node: Node,
        source: &[u8],
        defines: &mut Vec<String>,
        _imports: &mut Vec<String>,
        context_stack: &mut Vec<String>,
    ) {
        match node.kind() {
            "mod_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source);
                    if !name.is_empty() {
                        let q = qualified_name(context_stack, &name);
                        defines.push(q);
                        context_stack.push(name);
                        for i in 0..node.child_count() {
                            if let Some(c) = node.child(i) {
                                walk_rs(c, source, defines, _imports, context_stack);
                            }
                        }
                        context_stack.pop();
                        return;
                    }
                }
            }
            "impl_item" => {
                if let Some(type_node) = node.child_by_field_name("type") {
                    let type_name = node_text(type_node, source);
                    if !type_name.is_empty() {
                        context_stack.push(type_name);
                        for i in 0..node.child_count() {
                            if let Some(c) = node.child(i) {
                                walk_rs(c, source, defines, _imports, context_stack);
                            }
                        }
                        context_stack.pop();
                        return;
                    }
                }
            }
            "function_item" | "struct_item" | "enum_item" | "trait_item" | "type_item" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source);
                    if !name.is_empty() {
                        defines.push(qualified_name(context_stack, &name));
                    }
                }
            }
            "use_declaration" => {
                push_rust_use_imports(node, source, _imports);
            }
            _ => {}
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk_rs(c, source, defines, _imports, context_stack);
            }
        }
    }

    walk_rs(root, bytes, &mut defines, &mut imports, &mut context_stack);

    defines.sort_unstable();
    defines.dedup();
    imports.retain(|s| !s.is_empty() && s != "import");
    imports.sort_unstable();
    imports.dedup();
    SymbolExtraction { defines, imports }
}

/// Extract defines and imports from TypeScript/JavaScript source. Defines: function, class, method, interface, type_alias. Imports: module path first segment and named import identifiers.
fn extract_ts_js_impl(source: &str, language: tree_sitter::Language) -> SymbolExtraction {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return SymbolExtraction::default(),
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    let mut defines = Vec::new();
    let mut imports = Vec::new();

    fn walk_ts_js(node: Node, source: &[u8], defines: &mut Vec<String>, imports: &mut Vec<String>) {
        match node.kind() {
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "method_definition" => {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let s = node_text(name_node, source).trim().to_string();
                    if !s.is_empty() {
                        defines.push(s);
                    }
                }
            }
            "import_statement" | "import_declaration" => {
                if let Some(src_node) = node.child_by_field_name("source") {
                    let raw = node_text(src_node, source);
                    let module = raw
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .split('/')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    if !module.is_empty() {
                        imports.push(module);
                    }
                }
                for i in 0..node.child_count() {
                    let c = node.child(i).unwrap();
                    if c.kind() == "import_clause" || c.kind() == "named_imports" {
                        for j in 0..c.child_count() {
                            let spec = c.child(j).unwrap();
                            if spec.kind() == "import_specifier" {
                                if let Some(name_node) = spec.child_by_field_name("name") {
                                    let s = node_text(name_node, source).trim().to_string();
                                    if !s.is_empty() {
                                        imports.push(s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "export_statement" | "export_default_declaration" => {
                if let Some(decl) = node.child_by_field_name("declaration") {
                    let kind = decl.kind();
                    if kind == "function_declaration"
                        || kind == "generator_function_declaration"
                        || kind == "class_declaration"
                        || kind == "interface_declaration"
                        || kind == "type_alias_declaration"
                    {
                        if let Some(name_node) = decl.child_by_field_name("name") {
                            let s = node_text(name_node, source).trim().to_string();
                            if !s.is_empty() {
                                defines.push(s);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk_ts_js(c, source, defines, imports);
            }
        }
    }

    walk_ts_js(root, bytes, &mut defines, &mut imports);
    defines.sort_unstable();
    defines.dedup();
    imports.retain(|s| !s.is_empty());
    imports.sort_unstable();
    imports.dedup();
    SymbolExtraction { defines, imports }
}

/// Extract defines and imports from TypeScript/JavaScript by extension (ts/tsx/js/jsx).
pub fn extract_ts_js(source: &str, ext: &str) -> SymbolExtraction {
    let ext = ext.to_lowercase();
    let language = ts_js_language_for_ext(&ext);
    extract_ts_js_impl(source, language)
}

/// By language (extension): Python, Rust, and TS/JS implemented; ext "py" | "rs" | "ts" | "tsx" | "js" | "jsx" (case-insensitive); other ext return default empty.
pub fn extract_symbols(source: &str, ext: &str) -> SymbolExtraction {
    match ext.to_lowercase().as_str() {
        "py" => extract_python(source),
        "rs" => extract_rust(source),
        "ts" | "tsx" | "js" | "jsx" => extract_ts_js(source, ext),
        _ => SymbolExtraction::default(),
    }
}

/// Extract called function names from Python source (call nodes).
/// - Direct call `foo()` -> "foo"
/// - Attribute call `obj.method()` -> "method" (property_identifier / attribute field)
///
/// Rust impl stubbed for now (complex).
pub fn extract_calls_python(source: &str) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&LANGUAGE.into()).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    /// walk_calls.
    fn walk_calls(node: Node, source: &[u8], out: &mut Vec<String>) {
        if node.kind() == "call" {
            if let Some(func_node) = node.child_by_field_name("function") {
                let name = match func_node.kind() {
                    "identifier" | "keyword_identifier" => node_text(func_node, source),
                    "attribute" => {
                        if let Some(attr_node) = func_node.child_by_field_name("attribute") {
                            node_text(attr_node, source)
                        } else {
                            String::new()
                        }
                    }
                    _ => String::new(),
                };
                if !name.is_empty() {
                    out.push(name);
                }
            }
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk_calls(c, source, out);
            }
        }
    }

    walk_calls(root, bytes, &mut out);
    out.sort_unstable();
    out.dedup();
    out
}

/// Resolve tree-sitter language for TS/JS by extension: .ts or .js -> TYPESCRIPT, .tsx or .jsx -> TSX.
fn ts_js_language_for_ext(ext: &str) -> tree_sitter::Language {
    match ext {
        "tsx" | "jsx" => LANGUAGE_TSX.into(),
        _ => LANGUAGE_TYPESCRIPT.into(),
    }
}

/// Extract called function/method names from TypeScript/JavaScript source (call_expression nodes).
/// - Direct call `foo()` -> "foo"
/// - Member call `obj.method()` -> "method" (property from member_expression)
fn extract_calls_ts_js_impl(source: &str, language: tree_sitter::Language) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    let mut out = Vec::new();

    fn walk_call_expressions(node: Node, source: &[u8], out: &mut Vec<String>) {
        if node.kind() == "call_expression" {
            if let Some(func_node) = node.child_by_field_name("function") {
                let name = ts_js_callee_name(func_node, source);
                if !name.is_empty() {
                    out.push(name);
                }
            }
            // Do not return early — recurse into arguments to capture nested calls (e.g. foo(bar())).
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk_call_expressions(c, source, out);
            }
        }
    }

    fn ts_js_callee_name(node: Node, source: &[u8]) -> String {
        match node.kind() {
            "identifier" | "property_identifier" => node_text(node, source).trim().to_string(),
            "member_expression" => node
                .child_by_field_name("property")
                .map(|n| node_text(n, source).trim().to_string())
                .unwrap_or_default(),
            _ => {
                let mut fallback = String::new();
                for i in 0..node.child_count() {
                    if let Some(c) = node.child(i) {
                        let s = ts_js_callee_name(c, source);
                        if !s.is_empty() {
                            fallback = s;
                        }
                    }
                }
                fallback
            }
        }
    }

    walk_call_expressions(root, bytes, &mut out);
    out.sort_unstable();
    out.dedup();
    out
}

/// Extract called function/method names from TypeScript/JavaScript by extension (ts/tsx/js/jsx).
pub fn extract_calls_ts_js(source: &str, ext: &str) -> Vec<String> {
    let ext = ext.to_lowercase();
    let language = ts_js_language_for_ext(&ext);
    extract_calls_ts_js_impl(source, language)
}

/// By extension: ext "py" uses extract_calls_python; "ts"/"tsx"/"js"/"jsx" use extract_calls_ts_js; any other ext returns empty.
pub fn extract_calls(source: &str, ext: &str) -> Vec<String> {
    match ext.to_lowercase().as_str() {
        "py" => extract_calls_python(source),
        "ts" | "tsx" | "js" | "jsx" => extract_calls_ts_js(source, ext),
        _ => Vec::new(),
    }
}

/// True if mod_item is inline (`mod foo { ... }`); false for file reference (`mod foo;`).
fn mod_item_has_inline_body(node: Node) -> bool {
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            let k = c.kind();
            if k == "declaration_list" || k == "block" || k == "{" {
                return true;
            }
        }
    }
    false
}

/// Collect mod_item names (file refs only) and use crate::... paths (first two segments) from Rust source.
fn extract_rust_mod_and_use(source: &str) -> (Vec<String>, Vec<String>) {
    let mut mod_names = Vec::new();
    let mut use_crate_paths = Vec::new();
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok();
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return (mod_names, use_crate_paths),
    };
    let root = tree.root_node();
    let bytes = source.as_bytes();
    /// walk_mod_use.
    fn walk_mod_use(
        node: Node,
        source: &[u8],
        mod_names: &mut Vec<String>,
        use_paths: &mut Vec<String>,
    ) {
        match node.kind() {
            "mod_item" => {
                if mod_item_has_inline_body(node) {
                    return;
                }
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source);
                    if !name.is_empty() {
                        mod_names.push(name);
                    }
                }
            }
            "use_declaration" => {
                for i in 0..node.child_count() {
                    if let Some(c) = node.child(i) {
                        let kind = c.kind();
                        if kind == "scoped_identifier" || kind == "identifier" || kind == "path" {
                            let path_str = node_text(c, source).trim().to_string();
                            if path_str.starts_with("crate::") {
                                let after = path_str.trim_start_matches("crate::");
                                let segments: Vec<&str> = after
                                    .split("::")
                                    .filter(|s| !s.is_empty())
                                    .take(2)
                                    .collect();
                                if !segments.is_empty() {
                                    use_paths.push(segments.join("::"));
                                }
                            }
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
        for i in 0..node.child_count() {
            if let Some(c) = node.child(i) {
                walk_mod_use(c, source, mod_names, use_paths);
            }
        }
    }
    walk_mod_use(root, bytes, &mut mod_names, &mut use_crate_paths);
    (mod_names, use_crate_paths)
}

/// Build a Mermaid diagram string of module dependencies under workspace_src_root (e.g. src/).
/// Nodes = file-based modules (path with .rs stripped, / and \ -> ::). Edges = mod parent–child and use crate::... deps.
pub fn generate_module_graph(
    workspace_src_root: &Path,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let root = workspace_src_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_src_root.to_path_buf());
    let mut nodes: HashSet<String> = HashSet::new();
    let mut edges: Vec<(String, String)> = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "rs" {
            continue;
        }
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let is_mod_rs = path.file_name().and_then(|n| n.to_str()) == Some("mod.rs");
        let module_path = if is_mod_rs {
            rel.parent()
                .map(|p| {
                    p.to_string_lossy()
                        .replace([std::path::MAIN_SEPARATOR, '/'], "::")
                })
                .unwrap_or_default()
        } else {
            rel.with_extension("")
                .to_string_lossy()
                .replace([std::path::MAIN_SEPARATOR, '/'], "::")
        };
        if module_path.is_empty() {
            continue;
        }
        nodes.insert(module_path.clone());
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (child_mods, use_deps) = extract_rust_mod_and_use(&content);
        for child in &child_mods {
            let child_full =
                if module_path.is_empty() || module_path == "lib" || module_path == "main" {
                    child.clone()
                } else {
                    format!("{}::{}", module_path, child)
                };
            nodes.insert(child_full.clone());
            edges.push((module_path.clone(), child_full));
        }
        for dep in &use_deps {
            nodes.insert(dep.clone());
            edges.push((module_path.clone(), dep.clone()));
        }
    }

    // Mermaid: node IDs must be safe (no :: or special chars for box)
    /// mermaid_id.
    fn mermaid_id(s: &str) -> String {
        s.replace("::", "_")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
    }
    let mut lines = vec!["graph TD".to_string()];
    for n in &nodes {
        let id = mermaid_id(n);
        if id.is_empty() {
            continue;
        }
        lines.push(format!("    {}[\"{}\"]", id, n.replace('"', "\\\"")));
    }
    for (a, b) in &edges {
        let id_a = mermaid_id(a);
        let id_b = mermaid_id(b);
        if id_a.is_empty() || id_b.is_empty() {
            continue;
        }
        lines.push(format!("    {} --> {}", id_a, id_b));
    }
    Ok(lines.join("\n"))
}

/// Compute which modules are reachable from lib.rs or main.rs (and bin/*) and which are not.
/// Returns (reachable_set, unreachable_list). Unreachable modules are "phantom" (not wired into the crate).
pub fn modules_reachable_from_root(
    workspace_src_root: &Path,
) -> Result<(HashSet<String>, Vec<String>), Box<dyn std::error::Error + Send + Sync>> {
    let root = workspace_src_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_src_root.to_path_buf());
    let mut nodes: HashSet<String> = HashSet::new();
    let mut mod_edges: Vec<(String, String)> = Vec::new();

    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "rs" {
            continue;
        }
        let rel = path.strip_prefix(&root).unwrap_or(path);
        let is_mod_rs = path.file_name().and_then(|n| n.to_str()) == Some("mod.rs");
        let module_path = if is_mod_rs {
            rel.parent()
                .map(|p| {
                    p.to_string_lossy()
                        .replace([std::path::MAIN_SEPARATOR, '/'], "::")
                })
                .unwrap_or_default()
        } else {
            rel.with_extension("")
                .to_string_lossy()
                .replace([std::path::MAIN_SEPARATOR, '/'], "::")
        };
        if module_path.is_empty() {
            continue;
        }
        nodes.insert(module_path.clone());
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let (child_mods, _use_deps) = extract_rust_mod_and_use(&content);
        for child in &child_mods {
            let child_full = if module_path == "lib"
                || module_path == "main"
                || module_path.starts_with("bin::")
            {
                child.clone()
            } else {
                format!("{}::{}", module_path, child)
            };
            nodes.insert(child_full.clone());
            mod_edges.push((module_path.clone(), child_full));
        }
    }

    let roots: Vec<String> = nodes
        .iter()
        .filter(|n| n.as_str() == "lib" || n.as_str() == "main" || n.starts_with("bin::"))
        .cloned()
        .collect();

    let mut reachable: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = roots.into_iter().collect();
    for r in &queue {
        reachable.insert(r.clone());
    }
    while let Some(n) = queue.pop_front() {
        for (parent, child) in &mod_edges {
            if parent == &n && !reachable.contains(child) {
                reachable.insert(child.clone());
                queue.push_back(child.clone());
            }
        }
    }

    let unreachable: Vec<String> = nodes
        .iter()
        .filter(|n| !reachable.contains(*n))
        .cloned()
        .collect();
    Ok((reachable, unreachable))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// extract_calls_direct_identifier.
    fn extract_calls_direct_identifier() {
        let src = r#"def foo():
    bar()
    baz()
"#;
        let calls = extract_calls_python(src);
        assert!(calls.contains(&"bar".to_string()));
        assert!(calls.contains(&"baz".to_string()));
        assert_eq!(calls.len(), 2);
    }

    #[test]
    /// extract_calls_attribute.
    fn extract_calls_attribute() {
        let src = r#"def run():
    obj.method()
    x.append(1)
"#;
        let calls = extract_calls_python(src);
        assert!(calls.contains(&"method".to_string()));
        assert!(calls.contains(&"append".to_string()));
    }

    #[test]
    /// extract_calls_non_py_returns_empty.
    fn extract_calls_non_py_returns_empty() {
        let empty = extract_calls("fn main() { foo(); }", "rs");
        assert!(empty.is_empty());
    }

    #[test]
    /// extract_calls_ts_js_direct_and_member.
    fn extract_calls_ts_js_direct_and_member() {
        let src = r#"
function run() {
  foo();
  bar();
  obj.method();
  x.append(1);
}
"#;
        let calls_ts = extract_calls(src, "ts");
        assert!(calls_ts.contains(&"foo".to_string()), "ts: {:?}", calls_ts);
        assert!(calls_ts.contains(&"bar".to_string()), "ts: {:?}", calls_ts);
        assert!(
            calls_ts.contains(&"method".to_string()),
            "ts: {:?}",
            calls_ts
        );
        assert!(
            calls_ts.contains(&"append".to_string()),
            "ts: {:?}",
            calls_ts
        );
        let calls_js = extract_calls(src, "js");
        assert!(calls_js.contains(&"foo".to_string()), "js: {:?}", calls_js);
        assert!(
            calls_js.contains(&"method".to_string()),
            "js: {:?}",
            calls_js
        );
    }

    #[test]
    /// extract_calls_tsx_jsx_same_as_ts_js.
    fn extract_calls_tsx_jsx_same_as_ts_js() {
        let src = "function f() { helper(); }";
        assert!(extract_calls(src, "tsx").contains(&"helper".to_string()));
        assert!(extract_calls(src, "jsx").contains(&"helper".to_string()));
    }

    #[test]
    /// extract_ts_js_defines_and_imports.
    fn extract_ts_js_defines_and_imports() {
        let src = r#"
import { useState } from "react";
import lodash from "lodash";

export function greet() {
  return "hi";
}
class Foo {
  bar() {}
}
"#;
        let out = extract_ts_js(src, "ts");
        assert!(
            out.defines.iter().any(|s| s == "greet"),
            "defines: {:?}",
            out.defines
        );
        assert!(
            out.defines.iter().any(|s| s == "Foo"),
            "defines: {:?}",
            out.defines
        );
        assert!(
            out.defines.iter().any(|s| s == "bar"),
            "defines: {:?}",
            out.defines
        );
        assert!(
            out.imports
                .iter()
                .any(|s| s == "react" || s == "lodash" || s == "useState"),
            "imports: {:?}",
            out.imports
        );
    }

    #[test]
    /// extract_symbols_ts_js_dispatches.
    fn extract_symbols_ts_js_dispatches() {
        let src = "export function f() { g(); }";
        let out = extract_symbols(src, "ts");
        assert!(
            !out.defines.is_empty(),
            "extract_symbols(ts) should return defines"
        );
        assert!(out.defines.contains(&"f".to_string()));
        let out_js = extract_symbols(src, "js");
        assert!(out_js.defines.contains(&"f".to_string()));
    }

    #[test]
    /// extract_python_empty_returns_empty.
    fn extract_python_empty_returns_empty() {
        let out = extract_python("");
        assert!(out.defines.is_empty());
        assert!(out.imports.is_empty());
    }

    #[test]
    /// extract_python_defines_and_imports.
    fn extract_python_defines_and_imports() {
        let src = r#"
import os
from json import loads

def hello():
    pass

class Foo:
    def bar(self):
        pass
"#;
        let out = extract_python(src);
        assert!(out.defines.iter().any(|s| s == "hello"));
        assert!(out.defines.iter().any(|s| s == "Foo"));
        assert!(out.defines.iter().any(|s| s == "bar"));
        assert!(out.imports.iter().any(|s| s == "os"));
        assert!(out.imports.iter().any(|s| s == "json"));
        assert!(out.imports.iter().any(|s| s == "loads"));
    }

    #[test]
    /// extract_rust_empty_source_returns_empty.
    fn extract_rust_empty_source_returns_empty() {
        let out = extract_rust("");
        assert!(out.defines.is_empty() && out.imports.is_empty());
    }

    #[test]
    /// extract_rust_qualified_symbols.
    fn extract_rust_qualified_symbols() {
        let src = r#"
/// foo.
fn foo() {}

impl DatasetCollector {
/// new.
    fn new() -> Self {}
/// collect.
    fn collect(&self) {}
}
"#;
        let out = extract_rust(src);
        assert!(
            out.defines.iter().any(|s| s == "foo"),
            "top-level fn should be unqualified"
        );
        assert!(
            out.defines.iter().any(|s| s == "DatasetCollector::new"),
            "impl method should be qualified"
        );
        assert!(
            out.defines.iter().any(|s| s == "DatasetCollector::collect"),
            "impl method should be qualified"
        );
    }

    #[test]
    /// extract_rust_collects_use_imports.
    fn extract_rust_collects_use_imports() {
        let src = "use std::collections::HashMap;\nfn foo() {}";
        let out = extract_rust(src);
        assert!(
            out.imports
                .iter()
                .any(|s| s == "HashMap" || s.contains("HashMap")),
            "use std::collections::HashMap should yield HashMap in imports: {:?}",
            out.imports
        );
    }

    #[test]
    /// generate_module_graph_produces_mermaid.
    fn generate_module_graph_produces_mermaid() {
        let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let out = generate_module_graph(&src_dir).expect("generate_module_graph should succeed");
        assert!(
            out.contains("graph TD"),
            "output should be Mermaid graph TD"
        );
        assert!(out.contains("rag"), "output should include rag module");
    }

    #[test]
    /// generate_module_graph_minimal_tree.
    fn generate_module_graph_minimal_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        std::fs::write(root.join("lib.rs"), "mod a;").unwrap();
        std::fs::write(root.join("a.rs"), "use crate::b;").unwrap();
        std::fs::write(root.join("b.rs"), "").unwrap();
        let out = generate_module_graph(root).expect("minimal tree should succeed");
        assert!(out.contains("graph TD"), "output should be Mermaid");
        assert!(
            out.contains("lib") || out.contains("a") || out.contains("b"),
            "should list modules"
        );
    }
}
