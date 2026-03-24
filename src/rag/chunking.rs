//! Chunking: Python AST-based (tree-sitter) + generic recursive split. Function/class boundaries;
//! method chunks get class name and module imports prepended for context preservation.
//! Entry: [`chunk_file`] dispatches by extension (.py → Python, .rs → Rust, .ts/.tsx/.js/.jsx → TypeScript/JavaScript, else generic).
//! Python, Rust, and TS/JS use semantic boundaries; other extensions use generic size/overlap chunking.

use std::path::Path;
use tree_sitter::Node;
use tree_sitter_python::LANGUAGE;
use tree_sitter_typescript::{LANGUAGE_TSX, LANGUAGE_TYPESCRIPT};

use crate::rag::symbols;

/// Target chunk size in characters for generic (non-semantic) recursive split. Sized for ~256 tokens at
/// 4 chars/token average. Smaller chunks improve retrieval precision; larger improve context coherence.
/// Re-verify periodically for embedder and token budgets; see docs/setup/RAG_OPERATIONS.md § Chunk size and overlap.
const CHUNK_SIZE: usize = 1024;
/// Overlap between adjacent chunks to preserve cross-boundary context.
/// 128 chars (~32 tokens) matches typical sentence length; ensures boundary-spanning concepts appear in adjacent chunks.
const CHUNK_OVERLAP: usize = 128;

/// Average line length (chars) above which we bypass Tree-sitter to avoid parser memory/stack issues on minified or generated code. Lines are split by `\n`.
const AVG_LINE_LENGTH_THRESHOLD: usize = 2000;
/// Single-line length (chars) above which we bypass Tree-sitter regardless of average. Catches one huge line that could exhaust the parser.
const MAX_LINE_LENGTH: usize = 50_000;
/// Min content length (chars) to treat as "single huge line" when there is 0 or 1 newline; below this we do not bypass for minification.
const MIN_CONTENT_LEN_SINGLE_LINE_BYPASS: usize = 10_000;

/// Pre-flight: true if content looks minified or has excessively long lines and should bypass Tree-sitter in favor of generic chunking.
/// Uses average line length > [`AVG_LINE_LENGTH_THRESHOLD`], any line > [`MAX_LINE_LENGTH`], or single-line content longer than [`MIN_CONTENT_LEN_SINGLE_LINE_BYPASS`].
/// Rationale: avoids stack exhaustion and parsing hangs on adversarial or accidental inputs (see docs/NEXT_PHASE_PROPOSAL.md Option B).
pub fn should_bypass_treesitter(content: &str) -> bool {
    if content.is_empty() {
        return false;
    }
    let lines: Vec<&str> = content.lines().collect();
    let line_count = lines.len().max(1);
    let total_chars = content.chars().count();

    // Single or no newline and very long content -> treat as minified
    if line_count <= 1 && total_chars >= MIN_CONTENT_LEN_SINGLE_LINE_BYPASS {
        return true;
    }
    // Any line exceeds max line length
    for line in &lines {
        if line.chars().count() > MAX_LINE_LENGTH {
            return true;
        }
    }
    // Average line length above threshold
    let avg = total_chars / line_count;
    avg > AVG_LINE_LENGTH_THRESHOLD
}

/// One chunk: text, type_ (function/class/raw), name, defines, calls. Used for ingest and symbol_index.
#[derive(Debug, Clone)]
/// Chunk.
pub struct Chunk {
    pub text: String,
    pub type_: String,
    pub name: String,
    /// Symbols defined in this chunk (e.g. function/class/method name) for exact symbol_index mapping.
    pub defines: Vec<String>,
    /// Called function/method names extracted from this chunk (call nodes).
    pub calls: Vec<String>,
}

/// Returns the slice of source covered by node's byte range (clamped to source length).
fn node_source<'a>(node: Node, source: &'a str) -> &'a str {
    let r = node.byte_range();
    let start = r.start.min(source.len());
    let end = r.end.min(source.len());
    &source[start..end]
}

/// Returns the first child of `node` with kind "block" (e.g. function/class body), or None if none.
fn block_body(node: Node) -> Option<Node> {
    for i in 0..node.child_count() {
        let c = node.child(i)?;
        if c.kind() == "block" {
            return Some(c);
        }
    }
    None
}

/// Chunk type when parsing fails (e.g. Python AST error); full content in one chunk.
const RAW_CHUNK_TYPE: &str = "raw";
/// Chunk name used for raw fallback chunks (e.g. when parse fails).
const RAW_CHUNK_NAME: &str = "file_content";
/// Chunk type for section-based doc chunks (markdown heading hierarchy). Used by get_doc_outline.
pub const SECTION_CHUNK_TYPE: &str = "section";

/// Single chunk with full content when parsing fails (e.g. Python parse error). Used by chunk_python and chunk_rust. Chunk type_ and name are RAW_CHUNK_TYPE and RAW_CHUNK_NAME.
fn raw_fallback_chunk(content: &str) -> Chunk {
    Chunk {
        text: content.to_string(),
        type_: RAW_CHUNK_TYPE.to_string(),
        name: RAW_CHUNK_NAME.to_string(),
        defines: Vec::new(),
        calls: Vec::new(),
    }
}

/// Build context prefix: optional class line and import block for embedding.
fn context_prefix(class_name: Option<&str>, import_block: &str) -> String {
    let mut parts = Vec::new();
    if let Some(cn) = class_name {
        if !cn.is_empty() {
            parts.push(format!("Class {}:", cn));
        }
    }
    if !import_block.is_empty() {
        parts.push(import_block.to_string());
    }
    if parts.is_empty() {
        String::new()
    } else {
        parts.join("\n\n")
    }
}

/// Python: semantic chunks (classes, top-level functions, methods). Methods get class name + imports prepended.
pub fn chunk_python(content: &str, file_path: &str) -> Vec<Chunk> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&LANGUAGE.into()).ok();
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            tracing::warn!(path = %file_path, "tree-sitter parse failed; falling back to raw chunker");
            return vec![raw_fallback_chunk(content)];
        }
    };
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let root = tree.root_node();
    let mut import_segments = Vec::new();
    for i in 0..root.child_count() {
        let c = root.child(i).unwrap();
        if c.kind() == "import_statement" || c.kind() == "import_from_statement" {
            import_segments.push(node_source(c, content).to_string());
        }
    }
    let import_block = import_segments.join("\n");
    let mut chunks = Vec::new();
    for i in 0..root.child_count() {
        let c = root.child(i).unwrap();
        if c.kind() == "function_definition" || c.kind() == "async_function_definition" {
            let segment = node_source(c, content).to_string();
            let name = c
                .child_by_field_name("name")
                .map(|n| node_source(n, content).to_string())
                .unwrap_or_else(|| "lambda".to_string());
            let prefix = context_prefix(None, &import_block);
            let full_text = if prefix.is_empty() {
                segment
            } else {
                format!("{}\n\n{}", prefix, segment)
            };
            let segment_for_calls = node_source(c, content);
            let calls = symbols::extract_calls(segment_for_calls, &ext);
            chunks.push(Chunk {
                text: full_text,
                type_: "function".to_string(),
                name: name.clone(),
                defines: vec![name],
                calls,
            });
        } else if c.kind() == "class_definition" {
            let class_name = c
                .child_by_field_name("name")
                .map(|n| node_source(n, content).to_string())
                .unwrap_or_else(|| "Anonymous".to_string());
            let segment = node_source(c, content).to_string();
            let calls = symbols::extract_calls(&segment, &ext);
            chunks.push(Chunk {
                text: segment.clone(),
                type_: "class".to_string(),
                name: class_name.clone(),
                defines: vec![class_name.clone()],
                calls,
            });
            if let Some(block) = block_body(c) {
                for j in 0..block.child_count() {
                    let m = block.child(j).unwrap();
                    let func = match () {
                        _ if m.kind() == "function_definition"
                            || m.kind() == "async_function_definition" =>
                        {
                            m
                        }
                        _ if m.kind() == "decorated_definition" => {
                            match m.child_by_field_name("definition") {
                                Some(def)
                                    if def.kind() == "function_definition"
                                        || def.kind() == "async_function_definition" =>
                                {
                                    def
                                }
                                _ => continue,
                            }
                        }
                        _ => continue,
                    };
                    let method_src = node_source(func, content).to_string();
                    let method_name = func
                        .child_by_field_name("name")
                        .map(|n| node_source(n, content).to_string())
                        .unwrap_or_else(|| "lambda".to_string());
                    let calls = symbols::extract_calls(&method_src, &ext);
                    let prefix = context_prefix(Some(&class_name), &import_block);
                    let full_text = if prefix.is_empty() {
                        method_src
                    } else {
                        format!("{}\n\n{}", prefix, method_src)
                    };
                    chunks.push(Chunk {
                        text: full_text,
                        type_: "function".to_string(),
                        name: method_name.clone(),
                        defines: vec![method_name],
                        calls,
                    });
                }
            }
        }
    }
    if chunks.is_empty() {
        chunks.push(Chunk {
            text: content.to_string(),
            type_: "script".to_string(),
            name: "__main__".to_string(),
            defines: Vec::new(),
            calls: Vec::new(),
        });
    }
    chunks
}

/// Collect callee names from a Rust AST node (call_expression: function identifier or method name from field_expression).
fn extract_rust_calls_from_node(node: Node, content: &str, out: &mut Vec<String>) {
    if node.kind() == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            let name = rust_callee_name(func_node, content);
            if !name.is_empty() {
                out.push(name);
            }
        }
        return;
    }
    for i in 0..node.child_count() {
        if let Some(c) = node.child(i) {
            extract_rust_calls_from_node(c, content, out);
        }
    }
}

/// Return a single callee name: identifier text, or method name from field_expression (a.b() -> b).
fn rust_callee_name(node: Node, content: &str) -> String {
    match node.kind() {
        "identifier" => node_source(node, content).trim().to_string(),
        "field_expression" => node
            .child_by_field_name("field")
            .map(|n| node_source(n, content).trim().to_string())
            .unwrap_or_default(),
        "path" | "scoped_identifier" => node
            .child(node.child_count().saturating_sub(1))
            .map(|n| node_source(n, content).trim().to_string())
            .unwrap_or_default(),
        _ => {
            let mut fallback = String::new();
            for i in 0..node.child_count() {
                if let Some(c) = node.child(i) {
                    let s = rust_callee_name(c, content);
                    if !s.is_empty() {
                        fallback = s;
                    }
                }
            }
            fallback
        }
    }
}

/// Rust: semantic chunks (functions, structs, impl blocks).
pub fn chunk_rust(content: &str, file_path: &str) -> Vec<Chunk> {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).ok();
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            tracing::warn!(path = %file_path, "tree-sitter parse failed; falling back to raw chunker");
            return vec![raw_fallback_chunk(content)];
        }
    };
    let root = tree.root_node();
    let mut chunks = Vec::new();

    // Helper to get text
    let get_text = |n: Node| node_source(n, content).to_string();

    for i in 0..root.child_count() {
        let c = root.child(i).unwrap();
        match c.kind() {
            "function_item" => {
                let name = c
                    .child_by_field_name("name")
                    .map(get_text)
                    .unwrap_or_else(|| "fn".to_string());
                let text = get_text(c);
                let mut calls = Vec::new();
                extract_rust_calls_from_node(c, content, &mut calls);
                chunks.push(Chunk {
                    text,
                    type_: "function".to_string(),
                    name: name.clone(),
                    defines: vec![name],
                    calls,
                });
            }
            "struct_item" | "enum_item" | "trait_item" => {
                let name = c
                    .child_by_field_name("name")
                    .map(get_text)
                    .unwrap_or_else(|| "type".to_string());
                let text = get_text(c);
                let mut calls = Vec::new();
                extract_rust_calls_from_node(c, content, &mut calls);
                chunks.push(Chunk {
                    text,
                    type_: "type".to_string(),
                    name: name.clone(),
                    defines: vec![name],
                    calls,
                });
            }
            "impl_item" => {
                // impl Foo { fn bar() {} }
                // We want to chunk the individual methods, or the whole impl?
                // Ideally individual methods with context.
                let type_name = c
                    .child_by_field_name("type")
                    .map(get_text)
                    .unwrap_or_else(|| "impl".to_string());

                // If it has a body (declaration_list)
                let body = c.child_by_field_name("body");
                if let Some(body_node) = body {
                    for j in 0..body_node.child_count() {
                        let m = body_node.child(j).unwrap();
                        if m.kind() == "function_item" {
                            let method_name = m
                                .child_by_field_name("name")
                                .map(get_text)
                                .unwrap_or_else(|| "fn".to_string());
                            let method_text = get_text(m);
                            let full_text = format!("impl {} {{\n{}\n}}", type_name, method_text);
                            let qualified_name = format!("{}::{}", type_name, method_name);
                            let mut calls = Vec::new();
                            extract_rust_calls_from_node(m, content, &mut calls);
                            chunks.push(Chunk {
                                text: full_text,
                                type_: "method".to_string(),
                                name: method_name.clone(),
                                defines: vec![qualified_name, method_name],
                                calls,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if chunks.is_empty() {
        // If no semantic chunks found (e.g. mostly comments or macros), fallback to generic?
        // Or just return the whole file?
        // Let's use generic as fallback if empty, or just return main file.
        // For now, consistent with Python: return file content if empty.
        if !content.trim().is_empty() {
            return chunk_generic(content, CHUNK_SIZE, CHUNK_OVERLAP); // CHUNK_OVERLAP = overlap chars between consecutive chunks
        }
    }
    chunks
}

/// Resolve tree-sitter language for TS/JS by file extension: .ts or .js -> TYPESCRIPT, .tsx or .jsx -> TSX.
fn ts_js_language_for_ext(ext: &str) -> tree_sitter::Language {
    match ext {
        "tsx" | "jsx" => LANGUAGE_TSX.into(),
        _ => LANGUAGE_TYPESCRIPT.into(),
    }
}

/// Returns the class_body node of a class_declaration (field "body"). TypeScript/JavaScript use class_body, not "block".
fn ts_js_class_body(node: Node) -> Option<Node> {
    node.child_by_field_name("body")
}

/// Extract name from a TS/JS declaration node (function_declaration, class_declaration, or method_definition "name" field).
fn ts_js_node_name(node: Node, content: &str) -> String {
    node.child_by_field_name("name")
        .map(|n| node_source(n, content).trim().to_string())
        .unwrap_or_else(|| "anonymous".to_string())
}

/// Extract name from export_statement / export_default_declaration inner declaration (lexical_declaration has declarator with "name").
fn ts_js_export_declaration_name(decl: Node, content: &str) -> String {
    match decl.kind() {
        "function_declaration"
        | "generator_function_declaration"
        | "class_declaration"
        | "interface_declaration"
        | "type_alias_declaration" => ts_js_node_name(decl, content),
        "lexical_declaration" => {
            // const x = ... or let y = ...; first declarator's name
            if let Some(vars) = decl.child_by_field_name("declarator") {
                return ts_js_node_name(vars, content);
            }
            for i in 0..decl.child_count() {
                let c = decl.child(i).unwrap();
                if c.kind() == "variable_declarator" {
                    if let Some(name_node) = c.child_by_field_name("name") {
                        return node_source(name_node, content).trim().to_string();
                    }
                }
            }
            "constant".to_string()
        }
        _ => "export".to_string(),
    }
}

/// Internal: semantic chunking for TypeScript/JavaScript using the given language (TYPESCRIPT or TSX).
fn chunk_ts_js_impl(content: &str, file_path: &str, language: tree_sitter::Language) -> Vec<Chunk> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if should_bypass_treesitter(content) {
        return chunk_generic(content, CHUNK_SIZE, CHUNK_OVERLAP);
    }
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).ok();
    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => {
            tracing::warn!(path = %file_path, "tree-sitter parse failed; falling back to raw chunker");
            return vec![raw_fallback_chunk(content)];
        }
    };
    let root = tree.root_node();
    let mut import_segments = Vec::new();
    for i in 0..root.child_count() {
        let c = root.child(i).unwrap();
        if c.kind() == "import_statement" || c.kind() == "import_declaration" {
            import_segments.push(node_source(c, content).to_string());
        }
    }
    let import_block = import_segments.join("\n");
    let mut chunks = Vec::new();
    let get_text = |n: Node| node_source(n, content).to_string();

    for i in 0..root.child_count() {
        let c = root.child(i).unwrap();
        let kind = c.kind();

        // Top-level function (including async; grammar may use same function_declaration with modifier)
        if kind == "function_declaration" || kind == "generator_function_declaration" {
            let name = ts_js_node_name(c, content);
            let segment = get_text(c);
            let calls = symbols::extract_calls(&segment, &ext);
            let prefix = context_prefix(None, &import_block);
            let full_text = if prefix.is_empty() {
                segment
            } else {
                format!("{}\n\n{}", prefix, segment)
            };
            chunks.push(Chunk {
                text: full_text,
                type_: "function".to_string(),
                name: name.clone(),
                defines: vec![name],
                calls,
            });
            continue;
        }

        // Class: one chunk for the class, then method chunks with context
        if kind == "class_declaration" {
            let class_name = ts_js_node_name(c, content);
            let segment = get_text(c);
            let calls = symbols::extract_calls(&segment, &ext);
            chunks.push(Chunk {
                text: segment.clone(),
                type_: "class".to_string(),
                name: class_name.clone(),
                defines: vec![class_name.clone()],
                calls,
            });
            if let Some(body_node) = ts_js_class_body(c) {
                for j in 0..body_node.child_count() {
                    let m = body_node.child(j).unwrap();
                    if m.kind() == "method_definition" {
                        let method_name = ts_js_node_name(m, content);
                        let method_src = get_text(m);
                        let calls = symbols::extract_calls(&method_src, &ext);
                        let prefix = context_prefix(Some(&class_name), &import_block);
                        let full_text = if prefix.is_empty() {
                            method_src.clone()
                        } else {
                            format!("{}\n\n{}", prefix, method_src)
                        };
                        chunks.push(Chunk {
                            text: full_text,
                            type_: "function".to_string(),
                            name: method_name.clone(),
                            defines: vec![method_name],
                            calls,
                        });
                    }
                }
            }
            continue;
        }

        // Export: unwrap declaration and chunk if it's a declaration we care about
        if kind == "export_statement" {
            if let Some(decl) = c.child_by_field_name("declaration") {
                let decl_kind = decl.kind();
                let name = ts_js_export_declaration_name(decl, content);
                let segment = get_text(decl);
                let type_ = match decl_kind {
                    "interface_declaration" => "interface",
                    "type_alias_declaration" => "type",
                    "lexical_declaration" => "constant",
                    "function_declaration" | "generator_function_declaration" => "function",
                    "class_declaration" => "class",
                    _ => "export",
                };
                let calls = symbols::extract_calls(&segment, &ext);
                let prefix = context_prefix(None, &import_block);
                let full_text = if prefix.is_empty() {
                    segment.clone()
                } else {
                    format!("{}\n\n{}", prefix, segment)
                };
                chunks.push(Chunk {
                    text: full_text,
                    type_: type_.to_string(),
                    name: name.clone(),
                    defines: vec![name],
                    calls,
                });
            }
            continue;
        }

        if kind == "export_default_declaration" {
            if let Some(decl) = c.child_by_field_name("declaration") {
                let decl_kind = decl.kind();
                let name = ts_js_export_declaration_name(decl, content);
                let segment = get_text(decl);
                let type_ = match decl_kind {
                    "interface_declaration" => "interface",
                    "type_alias_declaration" => "type",
                    "lexical_declaration" => "constant",
                    "function_declaration" | "generator_function_declaration" => "function",
                    "class_declaration" => "class",
                    _ => "export",
                };
                let calls = symbols::extract_calls(&segment, &ext);
                let prefix = context_prefix(None, &import_block);
                let full_text = if prefix.is_empty() {
                    segment.clone()
                } else {
                    format!("{}\n\n{}", prefix, segment)
                };
                chunks.push(Chunk {
                    text: full_text,
                    type_: type_.to_string(),
                    name: name.clone(),
                    defines: vec![name],
                    calls,
                });
            }
            continue;
        }

        // Standalone interface or type alias (not inside export)
        if kind == "interface_declaration" {
            let name = ts_js_node_name(c, content);
            let segment = get_text(c);
            let calls = symbols::extract_calls(&segment, &ext);
            let prefix = context_prefix(None, &import_block);
            let full_text = if prefix.is_empty() {
                segment
            } else {
                format!("{}\n\n{}", prefix, segment)
            };
            chunks.push(Chunk {
                text: full_text,
                type_: "interface".to_string(),
                name: name.clone(),
                defines: vec![name],
                calls,
            });
            continue;
        }
        if kind == "type_alias_declaration" {
            let name = ts_js_node_name(c, content);
            let segment = get_text(c);
            let calls = symbols::extract_calls(&segment, &ext);
            let prefix = context_prefix(None, &import_block);
            let full_text = if prefix.is_empty() {
                segment
            } else {
                format!("{}\n\n{}", prefix, segment)
            };
            chunks.push(Chunk {
                text: full_text,
                type_: "type".to_string(),
                name: name.clone(),
                defines: vec![name],
                calls,
            });
        }
    }

    if chunks.is_empty() && !content.trim().is_empty() {
        chunks.push(Chunk {
            text: content.to_string(),
            type_: "script".to_string(),
            name: "file_content".to_string(),
            defines: Vec::new(),
            calls: Vec::new(),
        });
    }
    chunks
}

/// TypeScript: semantic chunks for .ts and .tsx (uses LANGUAGE_TYPESCRIPT or LANGUAGE_TSX by extension).
pub fn chunk_typescript(content: &str, file_path: &str) -> Vec<Chunk> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("ts")
        .to_lowercase();
    let language = ts_js_language_for_ext(&ext);
    chunk_ts_js_impl(content, file_path, language)
}

/// JavaScript: semantic chunks for .js and .jsx (uses LANGUAGE_TYPESCRIPT or LANGUAGE_TSX by extension).
pub fn chunk_javascript(content: &str, file_path: &str) -> Vec<Chunk> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("js")
        .to_lowercase();
    let language = ts_js_language_for_ext(&ext);
    chunk_ts_js_impl(content, file_path, language)
}

/// True if content has at least one ATX or setext heading. Used to choose section-based vs generic chunking for docs.
fn has_markdown_headings(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with('#')
            && t.len() > 1
            && t.as_bytes()
                .get(1)
                .map(|&b| b == b' ' || b == b'#')
                .unwrap_or(false)
        {
            return true;
        }
        if i > 0 && (t == "===" || t == "---") && !lines[i - 1].trim().is_empty() {
            return true;
        }
    }
    false
}

/// Snap a byte index to the nearest valid UTF-8 char boundary in `content`. `forward`: if true, snap to next boundary (>= idx); if false, snap to previous (<= idx). Avoids panics when section boundaries fall inside multi-byte characters.
fn snap_to_char_boundary(content: &str, idx: usize, forward: bool) -> usize {
    let len = content.len();
    if idx >= len {
        return len;
    }
    if content.is_char_boundary(idx) {
        return idx;
    }
    if forward {
        let mut i = idx;
        while i < len && !content.is_char_boundary(i) {
            i += 1;
        }
        i
    } else {
        let mut i = idx;
        while i > 0 && !content.is_char_boundary(i) {
            i -= 1;
        }
        i
    }
}

/// Section span: (byte_start, byte_end, heading_level 1..=6, heading_text).
struct SectionSpan {
    start: usize,
    end: usize,
    #[allow(dead_code)] // reserved for heading-level in outline/display
    level: u8,
    heading: String,
}

/// Parse ATX and setext headings, return section spans in document order.
fn find_markdown_sections(content: &str) -> Vec<SectionSpan> {
    let mut sections: Vec<SectionSpan> = Vec::new();
    let mut line_starts = Vec::new();
    let mut pos = 0usize;
    line_starts.push(0);
    for line in content.lines() {
        pos += line.len();
        if pos < content.len() && content.as_bytes()[pos] == b'\n' {
            pos += 1;
        }
        line_starts.push(pos);
    }
    let len = content.len();
    let lines: Vec<&str> = content.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let line_start = line_starts.get(idx).copied().unwrap_or(len);
        let trimmed = line.trim();
        let mut level = 0u8;
        let mut heading_text = String::new();
        if trimmed.starts_with('#') {
            let mut hash_count = 0usize;
            let mut j = 0;
            while j < trimmed.len() && trimmed.as_bytes()[j] == b'#' {
                hash_count += 1;
                j += 1;
            }
            if (1..=6).contains(&hash_count) {
                let rest = trimmed[j..].trim();
                level = hash_count as u8;
                heading_text = rest.to_string();
            }
        } else if !trimmed.is_empty() {
            let next_line = lines.get(idx + 1).map(|l| l.trim());
            if next_line == Some("===") || next_line == Some("---") {
                level = 1;
                heading_text = trimmed.to_string();
            }
        }
        if level > 0 && !heading_text.is_empty() {
            let section_start = line_start;
            let section_end = len;
            if let Some(prev) = sections.last_mut() {
                prev.end = section_start;
            }
            sections.push(SectionSpan {
                start: section_start,
                end: section_end,
                level,
                heading: heading_text,
            });
        }
    }
    if let Some(prev) = sections.last_mut() {
        prev.end = len;
    }
    sections
}

/// Section-based chunking for markdown: one chunk per heading section. type_ = SECTION_CHUNK_TYPE, name = heading text.
/// Used for token-efficient doc retrieval (outline first, then get_section by id).
/// Ingest assigns chunk ids path#0, path#1, ...; get_doc_outline returns (id, name); get_section(section_id) fetches by id.
pub fn chunk_markdown_by_headings(content: &str) -> Vec<Chunk> {
    let sections = find_markdown_sections(content);
    if sections.is_empty() {
        return Vec::new();
    }
    let mut chunks = Vec::new();
    for s in sections {
        let start = snap_to_char_boundary(content, s.start, false);
        let end = snap_to_char_boundary(content, s.end, true);
        if start >= end {
            continue;
        }
        let text = content[start..end].trim();
        if text.is_empty() {
            continue;
        }
        chunks.push(Chunk {
            text: text.to_string(),
            type_: SECTION_CHUNK_TYPE.to_string(),
            name: s.heading.clone(),
            defines: Vec::new(),
            calls: Vec::new(),
        });
    }
    chunks
}

/// Generic recursive split by size and overlap (parity with RecursiveCharacterTextSplitter).
pub fn chunk_generic(content: &str, chunk_size: usize, overlap: usize) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let len = content.chars().count();
    if len == 0 {
        return chunks;
    }
    let char_indices: Vec<usize> = content
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(content.len()))
        .collect();
    while start < char_indices.len() {
        let end = (start + chunk_size).min(char_indices.len());
        let byte_start = char_indices[start];
        let byte_end = if end < char_indices.len() {
            char_indices[end]
        } else {
            content.len()
        };
        let slice = &content[byte_start..byte_end];
        if !slice.trim().is_empty() {
            chunks.push(Chunk {
                text: slice.to_string(),
                type_: "text".to_string(),
                name: "chunk".to_string(),
                defines: Vec::new(),
                calls: Vec::new(),
            });
        }
        if end >= char_indices.len() {
            break;
        }
        start = end.saturating_sub(overlap);
    }
    if chunks.is_empty() {
        chunks.push(Chunk {
            text: content.to_string(),
            type_: "text".to_string(),
            name: "chunk".to_string(),
            defines: Vec::new(),
            calls: Vec::new(),
        });
    }
    chunks
}

/// Dispatch by extension: .py -> chunk_python, .rs -> chunk_rust, .ts/.tsx/.js/.jsx -> chunk_typescript/chunk_javascript,
/// .md/.mdx/.txt -> section-based when has headings (token-efficient docs), else chunk_generic(1024, 128).
pub fn chunk_file(content: &str, file_path: &str) -> Vec<Chunk> {
    let ext = Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if (ext == "py" || ext == "rs" || ext == "ts" || ext == "tsx" || ext == "js" || ext == "jsx")
        && should_bypass_treesitter(content)
    {
        return chunk_generic(content, CHUNK_SIZE, CHUNK_OVERLAP);
    }
    if ext == "py" {
        chunk_python(content, file_path)
    } else if ext == "rs" {
        chunk_rust(content, file_path)
    } else if ext == "ts" || ext == "tsx" {
        chunk_typescript(content, file_path)
    } else if ext == "js" || ext == "jsx" {
        chunk_javascript(content, file_path)
    } else if ext == "md" || ext == "markdown" || ext == "mdx" || ext == "txt" {
        if has_markdown_headings(content) {
            chunk_markdown_by_headings(content)
        } else {
            chunk_generic(content, CHUNK_SIZE, CHUNK_OVERLAP)
        }
    } else {
        chunk_generic(content, CHUNK_SIZE, CHUNK_OVERLAP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// chunk_python_has_defines_and_calls.
    fn chunk_python_has_defines_and_calls() {
        let src = r#"
def handler():
    foo()
    bar.baz()
"#;
        let chunks = chunk_python(src, "test.py");
        assert!(!chunks.is_empty());
        let func = chunks.iter().find(|c| c.name == "handler").unwrap();
        assert_eq!(func.defines, vec!["handler"]);
        assert!(func.calls.contains(&"foo".to_string()));
        assert!(func.calls.contains(&"baz".to_string()));
    }

    #[test]
    /// chunk_file_py_dispatches_to_python.
    fn chunk_file_py_dispatches_to_python() {
        let src = "def x(): pass";
        let chunks = chunk_file(src, "a.py");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].defines, vec!["x"]);
    }

    #[test]
    fn chunk_file_md_empty_content_returns_empty() {
        let chunks = chunk_file("", "readme.md");
        assert!(chunks.is_empty());
    }

    #[test]
    /// chunk_file_non_py_uses_generic.
    fn chunk_file_non_py_uses_generic() {
        let src = "some random text";
        let chunks = chunk_file(src, "readme.md");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "text");
    }

    #[test]
    /// chunk_file_md_with_headings_uses_section_chunks.
    fn chunk_file_md_with_headings_uses_section_chunks() {
        let src = "# Installation\n\nInstall with pip.\n\n## Config\n\nSet FOO=1.";
        let chunks = chunk_file(src, "docs/install.md");
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[0].name, "Installation");
        assert!(chunks[0].text.contains("Install with pip"));
        assert_eq!(chunks[1].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[1].name, "Config");
        assert!(chunks[1].text.contains("Set FOO=1"));
    }

    #[test]
    /// chunk_file_md_setext_heading_produces_section_chunk.
    fn chunk_file_md_setext_heading_produces_section_chunk() {
        let src = "Overview\n===\n\nThis is the overview section.";
        let chunks = chunk_file(src, "doc.md");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[0].name, "Overview");
        assert!(chunks[0].text.contains("overview section"));
    }

    #[test]
    /// chunk_file_md_setext_dash_heading_produces_section_chunk (setext level 2 with ---).
    fn chunk_file_md_setext_dash_heading_produces_section_chunk() {
        let src = "Section Two\n---\n\nBody for section two.";
        let chunks = chunk_file(src, "doc.md");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[0].name, "Section Two");
        assert!(chunks[0].text.contains("Body for section two"));
    }

    #[test]
    /// chunk_markdown_by_headings does not panic when content contains multi-byte UTF-8 (e.g. em dash).
    fn chunk_markdown_by_headings_multibyte_utf8_no_panic() {
        let src = "# Title\n\nBody with em dash — here.\n\n## Next\n\nMore.";
        let chunks = chunk_markdown_by_headings(src);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.text.contains("—")));
    }

    #[test]
    /// chunk_file_mdx_with_headings_uses_section_chunks.
    fn chunk_file_mdx_with_headings_uses_section_chunks() {
        let src = "# Introduction\n\nMDX content here.\n\n## Usage\n\nHow to use.";
        let chunks = chunk_file(src, "docs/intro.mdx");
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[0].name, "Introduction");
        assert_eq!(chunks[1].type_, SECTION_CHUNK_TYPE);
        assert_eq!(chunks[1].name, "Usage");
    }

    #[test]
    /// chunk_file_txt_no_headings_uses_generic.
    fn chunk_file_txt_no_headings_uses_generic() {
        let src = "Plain text file with no markdown headings.";
        let chunks = chunk_file(src, "readme.txt");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "text");
        assert_eq!(chunks[0].name, "chunk");
    }

    #[test]
    /// chunk_file_md_no_headings_uses_generic.
    fn chunk_file_md_no_headings_uses_generic() {
        let src = "Plain paragraph with no headings at all.";
        let chunks = chunk_file(src, "readme.md");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "text");
        assert_eq!(chunks[0].name, "chunk");
    }

    /// Double-chunking guard: chunk_file uses exactly one strategy per file (all section or all generic).
    #[test]
    fn chunk_file_single_strategy_per_file() {
        let with_headings = "# A\n\nBody.\n\n## B\n\nMore.";
        let chunks_section = chunk_file(with_headings, "doc.md");
        assert!(!chunks_section.is_empty());
        for c in &chunks_section {
            assert_eq!(
                c.type_, SECTION_CHUNK_TYPE,
                "all chunks must be section when file has headings"
            );
        }
        let no_headings = "Plain text with no markdown headings.";
        let chunks_generic = chunk_file(no_headings, "readme.md");
        assert!(!chunks_generic.is_empty());
        for c in &chunks_generic {
            assert_eq!(
                c.type_, "text",
                "all chunks must be generic when file has no headings"
            );
        }
    }

    #[test]
    /// chunk_rust_extracts_calls.
    fn chunk_rust_extracts_calls() {
        let src = r#"
/// foo.
fn foo() { bar(); }
/// bar.
fn bar() { baz(); }
"#;
        let chunks = chunk_rust(src, "lib.rs");
        let foo_chunk = chunks.iter().find(|c| c.name == "foo").unwrap();
        assert!(foo_chunk.calls.contains(&"bar".to_string()));
        let bar_chunk = chunks.iter().find(|c| c.name == "bar").unwrap();
        assert!(bar_chunk.calls.contains(&"baz".to_string()));
    }

    #[test]
    /// chunk_generic_empty_input_returns_empty.
    fn chunk_generic_empty_input_returns_empty() {
        let chunks = chunk_generic("", 1024, 128);
        assert!(chunks.is_empty());
    }

    #[test]
    /// chunk_generic_produces_overlapping_chunks_when_large.
    fn chunk_generic_produces_overlapping_chunks_when_large() {
        let content: String = (0..1500).map(|_| 'x').collect();
        let chunks = chunk_generic(&content, 1024, 128);
        assert!(
            chunks.len() >= 2,
            "content > chunk_size should yield multiple chunks"
        );
        assert!(chunks[0].text.len() <= 1024 + 100); // approx
        assert!(!chunks[1].text.is_empty());
    }

    #[test]
    /// chunk_generic_includes_last_character.
    fn chunk_generic_includes_last_character() {
        let content = "abcdef";
        let chunks = chunk_generic(content, 2, 0);
        let concatenated: String = chunks.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            concatenated, content,
            "all characters including last must appear in chunks"
        );
        assert!(chunks.last().unwrap().text.contains('f'));
    }

    // --- Tree-sitter guard (V1.1) ---

    #[test]
    fn guard_normal_python_does_not_bypass() {
        let src = "def x(): pass\nprint(1)\n";
        assert!(!should_bypass_treesitter(src));
        let chunks = chunk_file(src, "a.py");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].type_, "function");
        assert_eq!(chunks[0].defines, vec!["x"]);
    }

    #[test]
    fn guard_normal_rust_does_not_bypass() {
        let src = "fn main() {}\npub fn foo() {}\n";
        assert!(!should_bypass_treesitter(src));
        let chunks = chunk_file(src, "lib.rs");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "function");
    }

    #[test]
    fn guard_empty_does_not_bypass() {
        assert!(!should_bypass_treesitter(""));
    }

    #[test]
    fn guard_single_short_line_does_not_bypass() {
        assert!(!should_bypass_treesitter("def x(): pass"));
    }

    #[test]
    fn guard_avg_line_length_triggers_bypass() {
        // One line of 10_000 chars -> avg 10_000 > 2000
        let long_line: String = (0..10_000).map(|_| 'x').collect();
        assert!(should_bypass_treesitter(&long_line));
    }

    #[test]
    fn guard_many_long_lines_avg_above_threshold() {
        // 5 lines of 2500 chars each -> avg 2500 > 2000
        let line: String = (0..2500).map(|_| 'a').collect();
        let content = format!("{}\n{}\n{}\n{}\n{}", line, line, line, line, line);
        assert!(should_bypass_treesitter(&content));
    }

    #[test]
    fn guard_single_huge_line_over_max_triggers_bypass() {
        let line: String = (0..50_001).map(|_| 'y').collect();
        assert!(should_bypass_treesitter(&line));
    }

    #[test]
    fn guard_single_line_minified_over_10k_triggers_bypass() {
        let minified: String = (0..10_000).map(|_| 'z').collect();
        assert!(should_bypass_treesitter(&minified));
    }

    #[test]
    fn guard_py_bypass_returns_generic_chunks() {
        let long_line: String = (0..10_000).map(|_| 'x').collect();
        let chunks = chunk_file(&long_line, "minified.py");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "text");
        assert_eq!(chunks[0].name, "chunk");
        assert!(chunks
            .iter()
            .all(|c| c.type_ == "text" && c.name == "chunk"));
    }

    #[test]
    fn guard_rs_bypass_returns_generic_chunks() {
        let long_line: String = (0..10_000).map(|_| 'x').collect();
        let chunks = chunk_file(&long_line, "minified.rs");
        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].type_, "text");
        assert_eq!(chunks[0].name, "chunk");
        assert!(chunks
            .iter()
            .all(|c| c.type_ == "text" && c.name == "chunk"));
    }

    // --- TypeScript / JavaScript semantic chunking ---

    #[test]
    fn test_chunk_typescript_basics() {
        let src = r#"
class Foo {
  bar(): void {}
}
function helper(): number {
  return 42;
}
"#;
        let chunks = chunk_typescript(src, "test.ts");
        assert!(!chunks.is_empty(), "should produce at least one chunk");
        let class_chunk = chunks
            .iter()
            .find(|c| c.type_ == "class" && c.name == "Foo");
        assert!(
            class_chunk.is_some(),
            "should have class Foo chunk: {:?}",
            chunks
        );
        assert_eq!(class_chunk.unwrap().defines, vec!["Foo"]);
        let func_chunk = chunks
            .iter()
            .find(|c| c.type_ == "function" && c.name == "helper");
        assert!(
            func_chunk.is_some(),
            "should have function helper chunk: {:?}",
            chunks
        );
        assert_eq!(func_chunk.unwrap().defines, vec!["helper"]);
        let method_chunk = chunks
            .iter()
            .find(|c| c.type_ == "function" && c.name == "bar");
        assert!(
            method_chunk.is_some(),
            "should have method bar chunk: {:?}",
            chunks
        );
    }

    #[test]
    fn test_chunk_javascript_basics() {
        let src = r#"
export function greet() {
  return "hi";
}
async function fetchData() {
  return Promise.resolve(1);
}
"#;
        let chunks = chunk_javascript(src, "test.js");
        assert!(!chunks.is_empty(), "should produce at least one chunk");
        let export_fn = chunks.iter().find(|c| c.name == "greet");
        assert!(
            export_fn.is_some(),
            "should have export function greet: {:?}",
            chunks
        );
        assert_eq!(export_fn.unwrap().defines, vec!["greet"]);
        let async_fn = chunks.iter().find(|c| c.name == "fetchData");
        assert!(
            async_fn.is_some(),
            "should have async function fetchData: {:?}",
            chunks
        );
        assert_eq!(async_fn.unwrap().defines, vec!["fetchData"]);
    }

    #[test]
    fn test_chunk_file_ts_tsx_js_jsx() {
        let src = "function f() { return 1; }\nclass C { m() {} }";
        for path in ["a.ts", "a.tsx", "a.js", "a.jsx"] {
            let chunks = chunk_file(src, path);
            assert!(
                !chunks.is_empty(),
                "chunk_file({}) should yield chunks",
                path
            );
            let has_semantic = chunks
                .iter()
                .any(|c| c.type_ == "class" || c.type_ == "function");
            assert!(
                has_semantic,
                "chunk_file({}) should yield semantic chunks (class/function), got: {:?}",
                path,
                chunks.iter().map(|c| &c.type_).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    /// TS/JS chunks populated with calls from symbols::extract_calls (direct and member).
    fn test_chunk_ts_js_has_calls() {
        let src = r#"
function handler() {
  foo();
  bar();
  obj.method();
}
"#;
        let chunks_ts = chunk_file(src, "test.ts");
        let func_chunk = chunks_ts
            .iter()
            .find(|c| c.name == "handler" && c.type_ == "function")
            .expect("should have handler function chunk");
        assert!(
            !func_chunk.calls.is_empty(),
            "TS handler chunk should have calls: {:?}",
            func_chunk.calls
        );
        assert!(func_chunk.calls.contains(&"foo".to_string()));
        assert!(func_chunk.calls.contains(&"bar".to_string()));
        assert!(func_chunk.calls.contains(&"method".to_string()));

        let chunks_js = chunk_file(src, "test.js");
        let func_chunk_js = chunks_js
            .iter()
            .find(|c| c.name == "handler" && c.type_ == "function")
            .expect("should have handler function chunk");
        assert!(!func_chunk_js.calls.is_empty());
        assert!(func_chunk_js.calls.contains(&"foo".to_string()));
    }
}
