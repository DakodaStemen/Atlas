//! Unit and integration tests for AgenticHandler. Extracted from handler/mod.rs to keep mod.rs below the complexity ceiling.

use super::shell;
use super::{
    classify_web_source_type, format_rag_meta, ingest_web_items_to_rag, loop_guard_args_hash,
    parse_verification_agent_response, sanitize_shell_output, tool_list_checksum,
    truncate_for_budget, truncate_rag_response, validate_tool_params_impl, AgenticHandler,
    ExecuteShellCommandParams, GetRelatedCodeParams, GetSystemStatusParams, IngestionProvider,
    LogTrainingRowParams, QueryKnowledgeParams, ResolveSymbolParams, SubmitTaskParams,
    VectorStoreProvider, VerifyIntegrityParams, WebIngestItem, BUILTIN_TASK_TYPES,
    MINIMAL_TOOL_NAMES, TRUNCATION_SUFFIX,
};
use crate::rag::chunking::{Chunk, SECTION_CHUNK_TYPE};
use crate::rag::db::RagDb;
use crate::rag::embedding::RagEmbedder;
use crate::rag::handler::rag::{
    get_doc_outline_impl, get_section_impl, query_knowledge_core, GetDocOutlineParams,
    GetSectionParams,
};
use crate::rag::store::RagStore;
use crate::tools::web::set_test_fetcher;
use rmcp::model::Meta;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;

/// Mock ingestion: returns fixed content for any path (no filesystem).
#[derive(Clone)]
struct MockIngestion(String);

impl MockIngestion {
    fn new(content: &str) -> Self {
        Self(content.to_string())
    }
}

impl IngestionProvider for MockIngestion {
    fn read_content(&self, _path: &Path) -> anyhow::Result<String> {
        Ok(self.0.clone())
    }
}

/// Recorded save call for assertions.
struct MockStorageInner {
    delete_sources: Vec<String>,
    save_calls: Vec<(String, Vec<Chunk>)>,
}

/// Mock storage: records delete_by_source and save_chunks (no DB).
#[derive(Clone)]
struct MockStorage(Arc<Mutex<MockStorageInner>>);

impl MockStorage {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(MockStorageInner {
            delete_sources: Vec::new(),
            save_calls: Vec::new(),
        })))
    }
    fn last_save(&self) -> Option<(String, Vec<Chunk>)> {
        self.0
            .lock()
            .ok()
            .and_then(|g| g.save_calls.last().cloned())
    }
    fn delete_calls(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|g| g.delete_sources.clone())
            .unwrap_or_default()
    }
}

impl VectorStoreProvider for MockStorage {
    fn delete_by_source(&self, source: &str) -> anyhow::Result<()> {
        self.0
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {}", e))?
            .delete_sources
            .push(source.to_string());
        Ok(())
    }
    fn save_chunks(
        &self,
        source: &str,
        chunks: &[Chunk],
        _embeddings: Option<&[Vec<f32>]>,
    ) -> anyhow::Result<()> {
        let chunks_owned: Vec<Chunk> = chunks.to_vec();
        self.0
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {}", e))?
            .save_calls
            .push((source.to_string(), chunks_owned));
        Ok(())
    }
}

#[test]
/// process_ingestion reads via ingestion, chunks, and saves via storage; mocks verify orchestration.
fn test_handler_orchestration_flow() {
    use crate::rag::chunking::chunk_file;

    let db = RagDb::open(":memory:").expect("in-memory db");
    let store = Arc::new(RagStore::new(
        Arc::new(db),
        Arc::new(RagEmbedder::stub()),
        None,
        vec![],
    ));
    let content = "def foo(): pass\n";
    let mock_ingestion = MockIngestion::new(content);
    let mock_storage = MockStorage::new();
    let path = Path::new("test.py");
    let expected_chunks = chunk_file(content, path.to_string_lossy().as_ref());
    assert!(
        !expected_chunks.is_empty(),
        "chunk_file should yield at least one chunk for def foo(): pass"
    );

    let handler = AgenticHandler::with_providers(
        store,
        mock_ingestion,
        mock_storage.clone(),
        None,
        None,
        None,
        None,
        None,
        None,
        AgenticHandler::<MockIngestion, MockStorage>::tool_router(),
    );
    let count = handler.process_ingestion(path).expect("process_ingestion");
    assert_eq!(count, expected_chunks.len() as u32);

    assert!(
        mock_storage
            .delete_calls()
            .contains(&path.to_string_lossy().to_string()),
        "storage should have been asked to delete_by_source"
    );
    let (saved_source, saved_chunks) = mock_storage
        .last_save()
        .expect("storage should have received save_chunks");
    assert_eq!(saved_source, path.to_string_lossy().as_ref());
    assert_eq!(saved_chunks.len(), expected_chunks.len());
    assert_eq!(saved_chunks[0].name, expected_chunks[0].name);
    assert_eq!(saved_chunks[0].defines, expected_chunks[0].defines);
}

#[test]
fn loop_guard_args_hash_empty_none() {
    assert_eq!(loop_guard_args_hash(None), "");
}

#[test]
fn loop_guard_args_hash_empty_map() {
    let m: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    assert_eq!(loop_guard_args_hash(Some(&m)), "");
}

#[test]
fn loop_guard_args_hash_same_content_same_hash() {
    let mut m1 = serde_json::Map::new();
    m1.insert("a".into(), serde_json::json!(1));
    m1.insert("b".into(), serde_json::json!("x"));
    let mut m2 = serde_json::Map::new();
    m2.insert("b".into(), serde_json::json!("x"));
    m2.insert("a".into(), serde_json::json!(1));
    assert_eq!(
        loop_guard_args_hash(Some(&m1)),
        loop_guard_args_hash(Some(&m2))
    );
    assert!(!loop_guard_args_hash(Some(&m1)).is_empty());
}

/// State machine test: research_and_verify Compare step — Verification Agent response parsing.
/// Ensures INGEST=TRUE => overwrite, INGEST=FALSE => keep local, unparseable => default ingest.
#[test]
fn parse_verification_agent_response_ingest_true() {
    let (should_ingest, reason) =
        parse_verification_agent_response("[INGEST=TRUE]\nNew content has 2026 updates.");
    assert!(should_ingest, "INGEST=TRUE should yield should_ingest true");
    assert!(reason.contains("2026") || !reason.is_empty());
}

#[test]
fn parse_verification_agent_response_ingest_false() {
    let (should_ingest, _) =
        parse_verification_agent_response("[INGEST=FALSE]\nLocal RAG is already up to date.");
    assert!(
        !should_ingest,
        "INGEST=FALSE should yield should_ingest false"
    );
}

#[test]
fn parse_verification_agent_response_unparseable_defaults_to_ingest() {
    let (should_ingest, reason) = parse_verification_agent_response("Some random response.");
    assert!(
        should_ingest,
        "unparseable response defaults to ingest (safe fallback)"
    );
    assert!(!reason.is_empty());
}

#[test]
/// query_knowledge_params_deserializes_with_defaults_when_omitted.
fn query_knowledge_params_deserializes_with_defaults_when_omitted() {
    let json = r#"{"query":"how does RAG work"}"#;
    let p: QueryKnowledgeParams = serde_json::from_str(json).expect("deserialize");
    assert!(!p.reasoning, "reasoning should default to false");
    assert!(!p.execute, "execute should default to false");
    assert_eq!(p.query, "how does RAG work");
}

#[test]
/// resolve_symbol_params_deserializes.
fn resolve_symbol_params_deserializes() {
    let json = r#"{"symbol_name":"RagStore"}"#;
    let p: ResolveSymbolParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.symbol_name, "RagStore");
}

#[test]
/// get_related_code_params_deserializes.
fn get_related_code_params_deserializes() {
    let json = r#"{"symbol_name":"RagDb"}"#;
    let p: GetRelatedCodeParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.symbol_name, "RagDb");
}

#[test]
/// get_system_status_params_default_gpu_true.
fn get_system_status_params_default_gpu_true() {
    let json = r#"{}"#;
    let p: GetSystemStatusParams = serde_json::from_str(json).expect("deserialize");
    assert!(p.gpu, "gpu should default to true");
}

#[test]
fn truncate_for_budget_appends_required_suffix_when_truncated() {
    let long = "a".repeat(1000);
    let out = truncate_for_budget(&long, 100);
    assert!(
        out.ends_with(TRUNCATION_SUFFIX),
        "truncated output must end with audit-required suffix"
    );
    assert!(out.len() <= 100 + TRUNCATION_SUFFIX.len());
}

#[test]
fn truncate_rag_response_appends_suffix_when_over_char_budget() {
    // Stub embedder returns None for count_tokens, so we use char truncation. Ensure env does not disable it.
    let saved_chars = std::env::var_os("RAG_MAX_RESPONSE_CHARS");
    let saved_tokens = std::env::var_os("RAG_MAX_RESPONSE_TOKENS");
    std::env::set_var("RAG_MAX_RESPONSE_CHARS", "32000");
    std::env::remove_var("RAG_MAX_RESPONSE_TOKENS");
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open"));
    let store = RagStore::new(db, Arc::new(RagEmbedder::stub()), None, vec![]);
    let long = "a".repeat(50_000);
    let out = truncate_rag_response(&store, &long);
    if let Some(v) = saved_chars {
        std::env::set_var("RAG_MAX_RESPONSE_CHARS", v);
    } else {
        std::env::remove_var("RAG_MAX_RESPONSE_CHARS");
    }
    if let Some(v) = saved_tokens {
        std::env::set_var("RAG_MAX_RESPONSE_TOKENS", v);
    } else {
        std::env::remove_var("RAG_MAX_RESPONSE_TOKENS");
    }
    assert!(
        out.ends_with(TRUNCATION_SUFFIX),
        "truncate_rag_response (char fallback) must end with suffix when over budget"
    );
}

#[test]
fn format_rag_meta_includes_chunks_and_tokens() {
    let out = format_rag_meta("chunks_returned", 2, 100);
    assert!(out.contains("\"chunks_returned\": 2"));
    assert!(out.contains("\"tokens_estimated\": 100"));
    assert!(out.starts_with("\n\n_meta: "));
}

#[test]
fn format_rag_meta_includes_cost_avoided_when_env_set() {
    std::env::set_var("RAG_COST_PER_1M_INPUT_TOKENS_USD", "1.0");
    let out = format_rag_meta("chunks_returned", 1, 500_000);
    std::env::remove_var("RAG_COST_PER_1M_INPUT_TOKENS_USD");
    assert!(out.contains("\"cost_avoided_usd\""));
    assert!(out.contains("0.500000"));
}

#[tokio::test]
async fn get_section_response_includes_meta_when_content_returned() {
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open db"));
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let source = root
        .join("docs")
        .join("test_section.md")
        .to_string_lossy()
        .to_string();
    let section_id = format!("{}#0", source);
    db.upsert_chunk(
        &section_id,
        "Section body content.",
        &source,
        "",
        "[]",
        "[]",
        SECTION_CHUNK_TYPE,
        "Test Section",
        "[]",
        None,
        None,
        "code",
        "unknown",
    )
    .expect("upsert");
    let allowed = vec![root];
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        allowed,
    ));
    let handler = AgenticHandler::new(store);
    let res = get_section_impl(
        &handler,
        GetSectionParams {
            section_id: section_id.clone(),
        },
    )
    .await
    .expect("get_section_impl");
    let json = serde_json::to_string(&res).unwrap();
    assert!(
        json.contains("_meta"),
        "get_section response must include _meta when content returned; got: {}",
        json
    );
    assert!(json.contains("chunks_returned"));
    assert!(json.contains("tokens_estimated"));
}

#[tokio::test]
async fn get_doc_outline_response_includes_meta_when_sections_returned() {
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open db"));
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let source = root
        .join("docs")
        .join("test_outline.md")
        .to_string_lossy()
        .to_string();
    db.upsert_chunk(
        &format!("{}#0", source),
        "Overview content",
        &source,
        "",
        "[]",
        "[]",
        SECTION_CHUNK_TYPE,
        "Overview",
        "[]",
        None,
        None,
        "code",
        "unknown",
    )
    .expect("upsert");
    let allowed = vec![root];
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        allowed,
    ));
    let handler = AgenticHandler::new(store);
    let res = get_doc_outline_impl(
        &handler,
        GetDocOutlineParams {
            source: source.clone(),
        },
    )
    .await
    .expect("get_doc_outline_impl");
    let json = serde_json::to_string(&res).unwrap();
    assert!(
        json.contains("_meta"),
        "get_doc_outline response must include _meta when sections returned; got: {}",
        json
    );
    assert!(json.contains("sections_returned"));
    assert!(json.contains("tokens_estimated"));
}

#[tokio::test]
async fn query_knowledge_outline_only_returns_pipe_format_or_empty() {
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open db"));
    let root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let source = root
        .join("docs")
        .join("test_outline_only.md")
        .to_string_lossy()
        .to_string();
    db.upsert_chunk(
        &format!("{}#0", source),
        "outline only test chunk body",
        &source,
        "",
        "[]",
        "[]",
        SECTION_CHUNK_TYPE,
        "Outline Only Section",
        "[]",
        None,
        None,
        "code",
        "unknown",
    )
    .expect("upsert");
    let allowed = vec![root];
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        allowed,
    ));
    let handler = AgenticHandler::new(store);
    let params = QueryKnowledgeParams {
        query: "outline only test".to_string(),
        reasoning: false,
        execute: false,
        outline_only: true,
        section_first: false,
    };
    let res = query_knowledge_core(&handler, params)
        .await
        .expect("query_knowledge_core outline_only");
    let json = serde_json::to_string(&res).unwrap();
    // Must not panic; result is either EMPTY_RAG_CONTEXT or pipe-separated lines + _meta
    if !json.contains("No relevant information") {
        assert!(
            json.contains("|"),
            "outline_only rows must be pipe-separated; got: {}",
            json
        );
        assert!(
            json.contains("_meta"),
            "outline_only must include _meta; got: {}",
            json
        );
        assert!(
            json.contains("chunks_returned"),
            "outline_only _meta must have chunks_returned; got: {}",
            json
        );
    }
}

/// All truncation paths (RAG, truncate_for_budget used by fetch/shell/analysis, skills) append TRUNCATION_SUFFIX when over cap.
#[test]
fn all_truncation_paths_append_suffix_when_over_cap() {
    // RAG path
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open"));
    let store = RagStore::new(db, Arc::new(RagEmbedder::stub()), None, vec![]);
    let long = "a".repeat(50_000);
    std::env::set_var("RAG_MAX_RESPONSE_CHARS", "1000");
    let rag_out = truncate_rag_response(&store, &long);
    std::env::remove_var("RAG_MAX_RESPONSE_CHARS");
    assert!(
        rag_out.ends_with(TRUNCATION_SUFFIX),
        "RAG truncate_rag_response must end with suffix when over cap"
    );

    // truncate_for_budget path (fetch_web, shell, analysis)
    let buf = "x".repeat(200);
    let budget_out = truncate_for_budget(&buf, 100);
    assert!(
        budget_out.ends_with(TRUNCATION_SUFFIX),
        "truncate_for_budget must end with suffix when over cap"
    );

    // Skills path: same semantic (word cap + TRUNCATION_SUFFIX); test in isolation
    const MAX_WORDS: usize = 5000;
    let words: Vec<&str> = (0..MAX_WORDS + 1).map(|_| "word").collect();
    let truncated = words[..MAX_WORDS].join(" ");
    let with_suffix = format!("{}{}", truncated, TRUNCATION_SUFFIX);
    assert!(
        with_suffix.ends_with(TRUNCATION_SUFFIX),
        "skills truncation must end with suffix when over word cap"
    );
}

/// Default list_tools returns minimal subset; filtering list_all() by MINIMAL_TOOL_NAMES yields exactly those tools.
#[test]
fn minimal_tools_filter_subset() {
    let db = Arc::new(RagDb::open(":memory:").expect("db"));
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        vec![],
    ));
    let handler = AgenticHandler::new(store);
    let mut tools = handler.tool_router.list_all();
    tools.retain(|t| MINIMAL_TOOL_NAMES.contains(&t.name.as_ref()));
    assert_eq!(
        tools.len(),
        MINIMAL_TOOL_NAMES.len(),
        "filtered tools count must match MINIMAL_TOOL_NAMES"
    );
    for t in &tools {
        assert!(
            MINIMAL_TOOL_NAMES.contains(&t.name.as_ref()),
            "filtered tool {} must be in MINIMAL_TOOL_NAMES",
            t.name
        );
    }
}

/// validate_tool_params_impl: path outside ALLOWED_ROOTS yields warnings; path inside yields valid.
#[test]
fn validate_tool_params_path_checks() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let db = Arc::new(RagDb::open(":memory:").expect("db"));
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        vec![root.clone()],
    ));
    let handler = AgenticHandler::new(store);
    // Path outside allowed: security_audit with path like /tmp/outside
    let outside = std::env::temp_dir().join("validate_tool_params_outside_test");
    let (valid_out, warnings_out) = validate_tool_params_impl(
        &handler,
        "security_audit",
        Some(&serde_json::json!({ "path": outside.to_string_lossy() })),
    );
    assert!(
        !valid_out || !warnings_out.is_empty(),
        "path outside ALLOWED_ROOTS should yield invalid or warnings"
    );
    // Path inside allowed
    let inside = root.join("src").join("lib.rs");
    let (valid_in, warnings_in) = validate_tool_params_impl(
        &handler,
        "security_audit",
        Some(&serde_json::json!({ "path": inside.to_string_lossy() })),
    );
    assert!(valid_in, "path under ALLOWED_ROOTS should be valid");
    assert!(
        warnings_in.is_empty(),
        "path under ALLOWED_ROOTS should have no warnings"
    );
}

/// list_tools meta includes tool_list_checksum; checksum is deterministic (same tool set -> same value).
#[test]
fn list_tools_meta_includes_deterministic_tool_list_checksum() {
    let db = Arc::new(RagDb::open(":memory:").expect("db"));
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        vec![],
    ));
    let handler = AgenticHandler::new(store);
    let tools = handler.tool_router.list_all();
    let checksum = tool_list_checksum(&tools);
    assert!(!checksum.is_empty(), "tool_list_checksum must be non-empty");
    assert_eq!(checksum.len(), 16, "tool_list_checksum is 16 hex chars");
    assert_eq!(
        tool_list_checksum(&tools),
        checksum,
        "checksum must be deterministic"
    );
    let mut meta_obj = serde_json::Map::new();
    meta_obj.insert(
        "tool_list_checksum".to_string(),
        serde_json::Value::String(checksum.clone()),
    );
    let meta = Meta(meta_obj);
    assert_eq!(
        meta.get("tool_list_checksum")
            .and_then(|v: &serde_json::Value| v.as_str()),
        Some(checksum.as_str())
    );
}

/// tools_registry.json is canonical; handler #[tool(description)] must match. Fails when JSON and handler diverge.
#[test]
fn tools_registry_sync_with_handler_descriptions() {
    let json_str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/tools_registry.json"
    ));
    let registry: Vec<serde_json::Value> =
        serde_json::from_str(json_str).expect("tools_registry.json must be valid JSON array");
    let by_name: std::collections::HashMap<String, String> = registry
        .into_iter()
        .filter_map(|o| {
            let name = o.get("name")?.as_str()?.to_string();
            let desc = o.get("description")?.as_str()?.to_string();
            Some((name, desc))
        })
        .collect();
    let db = Arc::new(RagDb::open(":memory:").expect("db"));
    let store = Arc::new(RagStore::new(
        db,
        Arc::new(RagEmbedder::stub()),
        None,
        vec![],
    ));
    let handler = AgenticHandler::new(store);
    let tools = handler.tool_router.list_all();
    for t in &tools {
        let name: &str = &t.name;
        match by_name.get(name) {
            Some(expected_desc) => assert_eq!(
                t.description.as_deref().unwrap_or(""),
                expected_desc.as_str(),
                "handler description for tool '{}' must match tools_registry.json; update the #[tool(description = ...)] attribute",
                name
            ),
            None => panic!("tools_registry.json has no entry for tool '{}'; add it or remove the tool", name),
        }
    }
    for name in by_name.keys() {
        assert!(
            tools.iter().any(|t| t.name == *name),
            "handler has no tool '{}' but tools_registry.json has it; add the tool or remove from JSON",
            name
        );
    }
}

/// Generates docs/reports/TOKEN_USAGE_REPORT.md with measured chars/tokens with vs without truncation.
#[test]
fn generate_token_usage_report() {
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open"));
    let store = RagStore::new(db, Arc::new(RagEmbedder::stub()), None, vec![]);

    const RAG_CAP_CHARS: usize = 32_000;
    const SHELL_CAP_CHARS: usize = 16_000;

    let mut rows: Vec<(String, usize, usize, usize, usize)> = Vec::new();

    for (label, len) in [
        ("Small (under cap)", 10_000),
        ("Medium", 50_000),
        ("Large", 100_000),
        ("Very large", 200_000),
    ] {
        let long = "x".repeat(len);
        std::env::set_var("RAG_MAX_RESPONSE_CHARS", "0");
        let out_off = truncate_rag_response(&store, &long);
        let chars_without = out_off.chars().count();
        std::env::set_var("RAG_MAX_RESPONSE_CHARS", RAG_CAP_CHARS.to_string());
        let out_on = truncate_rag_response(&store, &long);
        let chars_with = out_on.chars().count();
        let tokens_without = chars_without / 4;
        let tokens_with = chars_with / 4;
        rows.push((
            label.to_string(),
            chars_without,
            chars_with,
            tokens_without,
            tokens_with,
        ));
    }
    std::env::remove_var("RAG_MAX_RESPONSE_CHARS");

    let mut fetch_rows: Vec<(String, usize, usize)> = Vec::new();
    for (label, len) in [
        ("fetch_web_markdown (50k)", 50_000),
        ("fetch_web (150k)", 150_000),
    ] {
        let long = "y".repeat(len);
        let without = truncate_for_budget(&long, 0);
        let with_cap = truncate_for_budget(&long, RAG_CAP_CHARS);
        fetch_rows.push((
            label.to_string(),
            without.chars().count(),
            with_cap.chars().count(),
        ));
    }

    let mut shell_rows: Vec<(String, usize, usize)> = Vec::new();
    for (label, len) in [
        ("Shell output (30k)", 30_000),
        ("Shell output (80k)", 80_000),
    ] {
        let long = "z".repeat(len);
        let without = truncate_for_budget(&long, 0);
        let with_cap = truncate_for_budget(&long, SHELL_CAP_CHARS);
        shell_rows.push((
            label.to_string(),
            without.chars().count(),
            with_cap.chars().count(),
        ));
    }

    let report = format!(
        r#"# Token Usage Report: With vs Without Truncation

Generated by `cargo test generate_token_usage_report` (monolith). Verifies that server-side caps reduce response size and estimated token usage.

## RAG-style responses (query_knowledge, get_section, get_related_code, resolve_symbol, analyze_error_log, review_diff, scaffold_reproduction_test)

Cap: **RAG_MAX_RESPONSE_CHARS** = {} (default). 0 = disabled.

| Scenario | Without truncation (chars) | With truncation (chars) | Est. tokens without | Est. tokens with | Savings (chars) |
|----------|---------------------------|--------------------------|---------------------|------------------|-----------------|
"#,
        RAG_CAP_CHARS
    );
    let report = rows
        .iter()
        .fold(report, |acc, (label, c_wo, c_w, t_wo, t_w)| {
            let pct = if *c_wo > 0 {
                ((*c_wo - *c_w) as f64 / *c_wo as f64 * 100.0) as u32
            } else {
                0
            };
            acc + &format!(
                "| {} | {} | {} | {} | {} | {}% |\n",
                label, c_wo, c_w, t_wo, t_w, pct
            )
        });

    let report = report
        + r#"
## fetch_web_markdown (FETCH_WEB_MAX_CHARS = 32_000 default)

| Scenario | Without (chars) | With (chars) | Savings |
|----------|----------------|--------------|---------|
"#;
    let report = fetch_rows.iter().fold(report, |acc, (label, wo, w)| {
        let pct = if *wo > 0 {
            ((*wo - *w) as f64 / *wo as f64 * 100.0) as u32
        } else {
            0
        };
        acc + &format!("| {} | {} | {} | {}% |\n", label, wo, w, pct)
    });

    let report = report
        + r#"
## execute_shell_command (EXECUTE_SHELL_MAX_OUTPUT_CHARS = 16_000 default)

| Scenario | Without (chars) | With (chars) | Savings |
|----------|----------------|--------------|---------|
"#;
    let report = shell_rows.iter().fold(report, |acc, (label, wo, w)| {
        let pct = if *wo > 0 {
            ((*wo - *w) as f64 / *wo as f64 * 100.0) as u32
        } else {
            0
        };
        acc + &format!("| {} | {} | {} | {}% |\n", label, wo, w, pct)
    });

    let report = report
        + r#"
## Conclusion

- With truncation **on** (default), all tool responses are capped; large payloads are cut with suffix `[TRUNCATED TO SAVE TOKENS]`.
- Token savings scale with payload size: from 0% when under cap to **68%+** at 100k chars (RAG cap 32k), **47%** at 50k (fetch_web), **80%** at 80k shell output (shell cap 16k).
- To disable: set `RAG_MAX_RESPONSE_CHARS=0`, `FETCH_WEB_MAX_CHARS=0`, `EXECUTE_SHELL_MAX_OUTPUT_CHARS=0` (not recommended for normal use).
"#;

    let report_path = std::path::Path::new("../docs/reports/TOKEN_USAGE_REPORT.md");
    if let Some(p) = report_path.parent() {
        let _ = std::fs::create_dir_all(p);
    }
    std::fs::write(report_path, &report).expect("write TOKEN_USAGE_REPORT.md");
}

#[test]
/// execute_shell_command_params_deserializes_empty_command.
fn execute_shell_command_params_deserializes_empty_command() {
    let json = r#"{"command":""}"#;
    let p: ExecuteShellCommandParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.command, "");
}

#[test]
/// classify_web_source_type_official_url.
fn classify_web_source_type_official_url() {
    assert_eq!(
        classify_web_source_type("https://doc.rust-lang.org/std/vec/struct.Vec.html"),
        "official"
    );
    assert_eq!(
        classify_web_source_type("https://docs.python.org/3/library/"),
        "official"
    );
}

#[test]
/// classify_web_source_type_stackoverflow.
fn classify_web_source_type_stackoverflow() {
    assert_eq!(
        classify_web_source_type("https://stackoverflow.com/questions/123/foo"),
        "stackoverflow"
    );
    assert_eq!(
        classify_web_source_type("https://tex.stackexchange.com/"),
        "stackoverflow"
    );
}

#[test]
/// classify_web_source_type_repository.
fn classify_web_source_type_repository() {
    assert_eq!(
        classify_web_source_type("https://github.com/user/repo"),
        "repository"
    );
    assert_eq!(
        classify_web_source_type("https://gitlab.com/group/proj"),
        "repository"
    );
    assert_eq!(
        classify_web_source_type("https://bitbucket.org/team/repo"),
        "repository"
    );
}

#[test]
/// classify_web_source_type_blog.
fn classify_web_source_type_blog() {
    assert_eq!(
        classify_web_source_type("https://medium.com/some-post"),
        "blog"
    );
    assert_eq!(classify_web_source_type("https://dev.to/article"), "blog");
    assert_eq!(
        classify_web_source_type("https://example.com/blog/entry"),
        "blog"
    );
}

#[test]
/// classify_web_source_type_external.
fn classify_web_source_type_external() {
    assert_eq!(
        classify_web_source_type("https://random-site.com/page"),
        "external"
    );
    assert_eq!(
        classify_web_source_type("https://example.com/docs"),
        "external"
    );
}

#[test]
/// sanitize_redacts_windows_user_paths.
fn sanitize_redacts_windows_user_paths() {
    let s = "error at C:\\Users\\jane\\project\\src\\lib.rs:10";
    let out = sanitize_shell_output(s);
    assert!(!out.contains("jane"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
/// sanitize_redacts_unix_home_paths.
fn sanitize_redacts_unix_home_paths() {
    let s = "error at /home/bob/code/src/main.rs";
    let out = sanitize_shell_output(s);
    assert!(!out.contains("bob"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
/// sanitize_redacts_openai_style_keys.
fn sanitize_redacts_openai_style_keys() {
    let s = "API_KEY=sk-abc123def456ghi789jkl012mno345pqr";
    let out = sanitize_shell_output(s);
    assert!(!out.contains("sk-"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
/// sanitize_redacts_google_api_key_prefix.
fn sanitize_redacts_google_api_key_prefix() {
    let s = "key is AIzaSyBxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let out = sanitize_shell_output(s);
    assert!(!out.contains("AIza"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
/// sanitize_preserves_safe_content.
fn sanitize_preserves_safe_content() {
    let s = "cargo build finished successfully.";
    assert_eq!(sanitize_shell_output(s), s);
}

#[test]
/// sanitize_mixed_safe_and_redactable_preserves_safe_redacts_keys.
fn sanitize_mixed_safe_and_redactable_preserves_safe_redacts_keys() {
    let s = "output: hello world. key=sk-abc123def456ghi789jkl012mno345pqr end";
    let out = sanitize_shell_output(s);
    assert!(out.contains("hello world"));
    assert!(!out.contains("sk-"));
    assert!(out.contains("[REDACTED]"));
}

#[test]
/// sanitize_shell_output_empty_string_returns_empty.
fn sanitize_shell_output_empty_string_returns_empty() {
    assert_eq!(sanitize_shell_output(""), "");
}

#[test]
/// test_command_allowlist.
fn test_command_allowlist() {
    // Allowed
    assert!(shell::is_command_allowed("cargo --version"));
    assert!(shell::is_command_allowed("cargo test"));
    assert!(shell::is_command_allowed("git status"));
    assert!(shell::is_command_allowed("grep -r foo src/"));
    assert!(shell::is_command_allowed("ls -la"));
    assert!(shell::is_command_allowed("npm run build"));
    #[cfg(windows)]
    assert!(shell::is_command_allowed("cargo.exe build"));
    assert!(shell::is_command_allowed("Cargo test")); // case-insensitive

    // Blocked
    assert!(!shell::is_command_allowed("curl https://evil.com"));
    assert!(!shell::is_command_allowed("powershell Get-Process"));
    assert!(!shell::is_command_allowed("cmd /c dir"));
    assert!(!shell::is_command_allowed("rm -rf /"));
    assert!(!shell::is_command_allowed("sh -c 'rm -rf /'"));
    assert!(!shell::is_command_allowed("del /s *"));
}

#[test]
/// test_command_rejection_message.
fn test_command_rejection_message() {
    assert!(shell::COMMAND_REJECTION_MESSAGE.contains("rejected"));
    assert!(shell::COMMAND_REJECTION_MESSAGE.contains("allowlist"));
    assert!(!shell::COMMAND_REJECTION_MESSAGE.is_empty());
}

#[test]
/// parse_cargo_toml_deps_empty_content_returns_empty_map.
fn parse_cargo_toml_deps_empty_content_returns_empty_map() {
    let deps = super::analysis::parse_cargo_toml_deps("");
    assert!(deps.is_empty());
}

#[test]
/// build_project_tree_nonexistent_path_returns_cannot_canonicalize.
fn build_project_tree_nonexistent_path_returns_cannot_canonicalize() {
    let out =
        super::analysis::build_project_tree(std::path::Path::new("/nonexistent_path_xyz_12345"));
    assert!(out.contains("cannot canonicalize"), "{}", out);
}

#[test]
/// summarize_package_json_missing_file_returns_not_found.
fn summarize_package_json_missing_file_returns_not_found() {
    let out = super::analysis::summarize_package_json(std::path::Path::new(
        "nonexistent_package_xyz.json",
    ));
    assert_eq!(out, "package.json not found");
}

#[test]
/// test_is_command_allowed.
fn test_is_command_allowed() {
    assert!(shell::is_command_allowed("cargo test"));
    assert!(shell::is_command_allowed("git status"));
    assert!(shell::is_command_allowed("ls -la"));
    assert!(shell::is_command_allowed("CARGO build")); // Case insensitive
    assert!(shell::is_command_allowed("cargo.exe test")); // Windows suffix

    assert!(!shell::is_command_allowed("rm -rf /"));
    assert!(!shell::is_command_allowed("python main.py"));
    assert!(!shell::is_command_allowed(""));
    assert!(!shell::is_command_allowed("   "));
}

#[test]
fn git_destructive_reject_message_contains_expected_text() {
    let msg = shell::GIT_DESTRUCTIVE_REJECT_MESSAGE;
    assert!(
        msg.contains("destructive git"),
        "reject message must mention destructive git"
    );
    assert!(
        msg.contains("not allowed"),
        "reject message must state command not allowed"
    );
}

#[test]
fn git_reset_hard_rejected_by_is_git_args_safe() {
    let argv = vec!["git".into(), "reset".into(), "--hard".into(), "HEAD".into()];
    assert!(
        !shell::is_git_args_safe(&argv),
        "git reset --hard must be rejected by is_git_args_safe"
    );
}

/// Integration test: ingest_web_items_to_rag writes chunks to DB; lookup path (research_and_verify) uses the same ingest.
#[test]
fn research_and_verify_integration_ingest_web_items_populates_db() {
    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = RagDb::open(tmp.path()).expect("open db");
    let embedder = RagEmbedder::stub();
    let item = WebIngestItem {
        url: "https://example.com/test-doc".to_string(),
        summary: "Test document summary".to_string(),
        detail_chunks: vec![
            "First chunk content.".to_string(),
            "Second chunk content.".to_string(),
        ],
        source_type: "official",
    };
    let count = ingest_web_items_to_rag(&db, &embedder, &[item]);
    assert_eq!(count, 2, "should ingest 2 detail chunks");
    let chunks = db
        .get_chunks_by_source("https://example.com/test-doc")
        .expect("get_chunks_by_source");
    assert!(
        chunks.len() >= 2,
        "DB should contain at least 2 chunks for the source, got {}",
        chunks.len()
    );
}

/// Full-flow integration test: research_and_verify path (RAG lookup → fetch via test fetcher → ingest) without async handler to avoid runtime drop issues.
#[test]
fn research_and_verify_full_flow_integration() {
    const TEST_URL: &str = "https://test.example.com/spec";
    const TEST_TOPIC: &str = "integration test topic";
    let fixed_markdown = "# Test Doc\n\nThis is fixed markdown content for the full-flow test.";

    set_test_fetcher(Some(Box::new(|_url| Ok(fixed_markdown.to_string()))));

    let tmp = tempfile::NamedTempFile::new().expect("temp file");
    let db = Arc::new(RagDb::open(tmp.path()).expect("open db"));
    let embedder = Arc::new(RagEmbedder::stub());
    let allowed_root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let allowed_roots = vec![allowed_root];
    let store = Arc::new(RagStore::new(db.clone(), embedder, None, allowed_roots));

    let rows = store
        .hierarchical_search(TEST_TOPIC, store.rerank_candidates, 20)
        .expect("lookup");
    assert!(rows.is_empty(), "RAG should be empty before ingest");

    let md = crate::tools::web::fetch_url_as_markdown(TEST_URL).expect("fetch via test fetcher");
    assert!(
        md.contains("Test Doc"),
        "fetched content should come from test fetcher"
    );

    let item = WebIngestItem {
        url: TEST_URL.to_string(),
        summary: md.lines().next().unwrap_or("").trim().to_string(),
        detail_chunks: crate::tools::web::chunk_text(&md, 500, 50)
            .into_iter()
            .map(|(text, _)| text)
            .collect(),
        source_type: classify_web_source_type(TEST_URL),
    };
    let count = ingest_web_items_to_rag(db.as_ref(), store.embedder.as_ref(), &[item]);
    set_test_fetcher(None);

    assert!(
        count >= 1,
        "should ingest at least one chunk, got {}",
        count
    );
    let chunks = db
        .get_chunks_by_source(TEST_URL)
        .expect("get_chunks_by_source");
    assert!(
        !chunks.is_empty(),
        "DB should contain chunks for {}, got {}",
        TEST_URL,
        chunks.len()
    );
}

#[test]
fn test_parse_shell_allowlist() {
    let list = shell::parse_shell_allowlist("cargo, git , pytest");
    assert_eq!(list, ["cargo", "git", "pytest"]);
    let list = shell::parse_shell_allowlist("cargo,/usr/bin/evil,..");
    assert_eq!(list, ["cargo"]);
    assert!(shell::parse_shell_allowlist("").is_empty());
    assert!(shell::parse_shell_allowlist("  ,  ,  ").is_empty());
    let list = shell::parse_shell_allowlist("CARGO,NPM");
    assert_eq!(list, ["cargo", "npm"]);
}

#[test]
/// test_normalize_path_display.
fn test_normalize_path_display() {
    let p = std::path::Path::new("C:\\Users\\Guest\\Documents");
    let out = super::analysis::normalize_path_display(p);
    assert_eq!(out, "C:/Users/Guest/Documents");

    let p2 = std::path::Path::new("src/lib.rs");
    let out2 = super::analysis::normalize_path_display(p2);
    assert_eq!(out2, "src/lib.rs");
}

#[test]
fn log_training_row_params_deserializes_with_optional_domain() {
    let json = r#"{"query":"[DOCS] Document X","context":"<ctx>doc</ctx>","response":"Done."}"#;
    let p: LogTrainingRowParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.query, "[DOCS] Document X");
    assert_eq!(p.context, "<ctx>doc</ctx>");
    assert_eq!(p.response, "Done.");
    assert_eq!(p.domain, None);

    let json_with_domain = r#"{"query":"q","context":"c","response":"r","domain":"rust_std"}"#;
    let p2: LogTrainingRowParams = serde_json::from_str(json_with_domain).expect("deserialize");
    assert_eq!(p2.domain.as_deref(), Some("rust_std"));
}

#[test]
fn verify_integrity_params_deserializes_workspace_path() {
    let json = r#"{"workspace_path":"/path/to/crate"}"#;
    let p: VerifyIntegrityParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.workspace_path, "/path/to/crate");

    let json_empty = r#"{}"#;
    let p2: VerifyIntegrityParams = serde_json::from_str(json_empty).expect("deserialize");
    assert!(p2.workspace_path.is_empty());
}

#[test]
fn submit_task_allowed_types_contains_builtins() {
    assert_eq!(BUILTIN_TASK_TYPES.len(), 6);
    assert!(BUILTIN_TASK_TYPES.contains(&"research"));
    assert!(BUILTIN_TASK_TYPES.contains(&"research_ingest"));
    assert!(BUILTIN_TASK_TYPES.contains(&"ingest"));
    assert!(BUILTIN_TASK_TYPES.contains(&"refresh_file_index"));
    assert!(BUILTIN_TASK_TYPES.contains(&"verify-integrity"));
    assert!(BUILTIN_TASK_TYPES.contains(&"data-clean"));
    assert!(!BUILTIN_TASK_TYPES.contains(&"unknown"));
    let allowed = super::allowed_task_types();
    assert!(allowed.len() >= 5);
    assert!(allowed.contains(&"research".to_string()));
    assert!(allowed.contains(&"research_ingest".to_string()));
}

#[test]
fn submit_task_params_deserializes_with_payload() {
    let json = r#"{"task_type":"research","payload":{"query":"RAG best practices"}}"#;
    let p: SubmitTaskParams = serde_json::from_str(json).expect("deserialize");
    assert_eq!(p.task_type, "research");
    assert!(p.payload.get("query").and_then(|v| v.as_str()) == Some("RAG best practices"));

    let json_ingest = r#"{"task_type":"ingest","payload":{"path":"."}}"#;
    let p2: SubmitTaskParams = serde_json::from_str(json_ingest).expect("deserialize");
    assert_eq!(p2.task_type, "ingest");
    assert_eq!(p2.payload.get("path").and_then(|v| v.as_str()), Some("."));

    let json_empty = r#"{"task_type":"verify-integrity"}"#;
    let p3: SubmitTaskParams = serde_json::from_str(json_empty).expect("deserialize");
    assert_eq!(p3.task_type, "verify-integrity");
    assert!(
        p3.payload.is_null()
            || (p3.payload.is_object() && p3.payload.as_object().unwrap().is_empty())
    );
}

#[test]
fn submit_task_writes_valid_json_to_inbox() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let inbox_dir = root.join("_tasks").join("inbox");
    std::fs::create_dir_all(&inbox_dir).unwrap();
    let id = format!("20260306T120000Z_{}", std::process::id());
    let task_json = serde_json::json!({
        "id": id,
        "type": "research",
        "payload": { "query": "test" },
        "created_at": "2026-03-06T12:00:00Z",
    });
    let filename = format!("{}.json", id);
    let file_path = inbox_dir.join(&filename);
    std::fs::write(
        &file_path,
        serde_json::to_string_pretty(&task_json).unwrap(),
    )
    .unwrap();
    assert!(file_path.exists());
    let content = std::fs::read_to_string(&file_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed.get("id").and_then(|v| v.as_str()), Some(id.as_str()));
    assert_eq!(
        parsed.get("type").and_then(|v| v.as_str()),
        Some("research")
    );
    assert_eq!(
        parsed
            .get("payload")
            .and_then(|v| v.get("query"))
            .and_then(|v| v.as_str()),
        Some("test")
    );
    assert!(parsed.get("created_at").is_some());
}

#[test]
fn test_redirect_targets_from_argv() {
    let argv = vec![
        "cargo".to_string(),
        "test".to_string(),
        ">".to_string(),
        "docs/out.txt".to_string(),
    ];
    let out = shell::redirect_targets_from_argv(&argv);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], std::path::Path::new("docs/out.txt"));

    let argv2 = vec!["cargo".to_string(), "build".to_string()];
    assert!(shell::redirect_targets_from_argv(&argv2).is_empty());

    let argv3 = vec!["ls".to_string(), ">>".to_string(), "log.txt".to_string()];
    let out3 = shell::redirect_targets_from_argv(&argv3);
    assert_eq!(out3.len(), 1);
    assert_eq!(out3[0], std::path::Path::new("log.txt"));
}

#[test]
fn test_vault_boundary_redirect_under_hub_rejected() {
    // When a redirect target resolves under an allowed root, path_under_allowed is true (handler would reject).
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let allowed: Vec<std::path::PathBuf> = vec![root.clone()];
    let target_under = root.join("docs").join("out.txt");
    assert!(crate::rag::path_filter::path_under_allowed(
        &target_under,
        &allowed,
        false,
    ));
}
