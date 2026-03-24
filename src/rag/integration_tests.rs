//! Integration tests: ingest → DB → retrieval / symbols / doc outline.
//! Run with `cargo test --lib integration_`.

#[cfg(test)]
mod tests {
    use crate::rag::chunking::SECTION_CHUNK_TYPE;
    use crate::rag::db::RagDb;
    use crate::rag::embedding::RagEmbedder;
    use crate::rag::ingest::ingest_directory;
    use crate::rag::store::RagStore;
    use std::path::PathBuf;
    use std::sync::Arc;

    /// Ingest small tree (one .rs + one .md with headings) then hybrid_search / hierarchical_search;
    /// assert non-empty and section chunks present for .md.
    #[test]
    fn integration_ingest_then_hybrid_search_returns_section_chunks_for_md() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let root = temp_dir.path();
        let db_path = root.join("rag.db");
        let manifest_path = root.join("rag_manifest.json");

        std::fs::write(
            root.join("lib.rs"),
            "pub fn main() { println!(\"hello\"); }",
        )
        .expect("write rs");
        std::fs::create_dir_all(root.join("docs")).expect("create dir");
        std::fs::write(
            root.join("docs").join("guide.md"),
            "# Installation\n\nInstall here.\n\n## Config\n\nSet FOO=1.",
        )
        .expect("write md");

        let allowed: Vec<PathBuf> =
            vec![root.canonicalize().unwrap_or_else(|_| root.to_path_buf())];
        let db = Arc::new(RagDb::open(&db_path).expect("open db"));
        let embedder = Arc::new(RagEmbedder::stub());

        let count = ingest_directory(
            root,
            Arc::clone(&db),
            Arc::clone(&embedder),
            allowed.clone(),
            manifest_path,
        )
        .expect("ingest");
        assert!(count >= 1, "expected at least one file ingested");

        let store = RagStore::new(db, embedder, None, allowed);
        let rows = store
            .hybrid_search("Installation", 10, None)
            .expect("hybrid_search");
        assert!(!rows.is_empty(), "hybrid_search should return chunks (FTS)");
        let section_rows: Vec<_> = rows
            .iter()
            .filter(|r| r.type_ == SECTION_CHUNK_TYPE)
            .collect();
        assert!(
            !section_rows.is_empty(),
            "expected at least one section chunk from .md, got types: {:?}",
            rows.iter().map(|r| &r.type_).collect::<Vec<_>>()
        );
    }

    /// Ingest file with symbol; get_related_code(symbol) returns definition and references.
    #[test]
    fn integration_ingest_then_get_related_code_returns_definition_and_refs() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let root = temp_dir.path();
        let db_path = root.join("rag.db");
        let manifest_path = root.join("rag_manifest.json");

        std::fs::write(root.join("def.py"), "def helper(): return 42").expect("write def");
        std::fs::write(
            root.join("caller.py"),
            "from def import helper\ndef main(): helper()",
        )
        .expect("write caller");

        let allowed: Vec<PathBuf> =
            vec![root.canonicalize().unwrap_or_else(|_| root.to_path_buf())];
        let db = Arc::new(RagDb::open(&db_path).expect("open db"));
        let embedder = Arc::new(RagEmbedder::stub());

        ingest_directory(
            root,
            Arc::clone(&db),
            Arc::clone(&embedder),
            allowed.clone(),
            manifest_path,
        )
        .expect("ingest");

        let store = RagStore::new(db, embedder, None, allowed);
        let rows = store
            .get_related_code("helper", None)
            .expect("get_related_code");
        assert!(!rows.is_empty());
        let has_def = rows.iter().any(|r| r.defines.contains("helper"));
        assert!(has_def, "expected defining chunk for helper");
    }

    /// Ingest one .md with headings; get_chunks_by_source + filter section then get_chunks_by_ids;
    /// assert outline and section text (mirrors get_doc_outline / get_section).
    #[test]
    fn integration_ingest_md_then_doc_outline_and_section_content() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let root = temp_dir.path();
        let db_path = root.join("rag.db");
        let manifest_path = root.join("rag_manifest.json");

        std::fs::create_dir_all(root.join("docs")).expect("create dir");
        let md_path = root.join("docs").join("readme.md");
        std::fs::write(
            &md_path,
            "# Overview\n\nThis is the overview.\n\n## Details\n\nMore here.",
        )
        .expect("write md");

        let allowed: Vec<PathBuf> =
            vec![root.canonicalize().unwrap_or_else(|_| root.to_path_buf())];
        let db = Arc::new(RagDb::open(&db_path).expect("open db"));
        let embedder = Arc::new(RagEmbedder::stub());

        ingest_directory(
            root,
            Arc::clone(&db),
            Arc::clone(&embedder),
            allowed,
            manifest_path,
        )
        .expect("ingest");

        let source = md_path.to_string_lossy().to_string();
        let rows = db
            .get_chunks_by_source(&source)
            .expect("get_chunks_by_source");
        let sections: Vec<_> = rows
            .iter()
            .filter(|r| r.type_ == SECTION_CHUNK_TYPE)
            .map(|r| (r.id.clone(), r.name.clone()))
            .collect();
        assert!(!sections.is_empty(), "expected section chunks");
        assert!(sections.iter().any(|(_, name)| name == "Overview"));
        let first_id = sections[0].0.clone();
        let by_id = db
            .get_chunks_by_ids(&[first_id])
            .expect("get_chunks_by_ids");
        assert_eq!(by_id.len(), 1);
        assert!(by_id[0].text.contains("overview"));
    }

    /// Full-flow research_and_verify path: ingest workspace files with known content, run Lookup (hierarchical_search),
    /// assert non-empty and that workspace chunks have last_updated set (time-based Validate uses this).
    #[test]
    fn integration_ingest_then_hierarchical_search_returns_chunks_with_last_updated() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let root = temp_dir.path();
        let db_path = root.join("rag.db");
        let manifest_path = root.join("rag_manifest.json");

        let topic = "Rust ownership and borrowing";
        std::fs::write(
            root.join("guide.md"),
            "# Rust ownership and borrowing\n\nThis document explains ownership.",
        )
        .expect("write md");

        let allowed: Vec<PathBuf> =
            vec![root.canonicalize().unwrap_or_else(|_| root.to_path_buf())];
        let db = Arc::new(RagDb::open(&db_path).expect("open db"));
        let embedder = Arc::new(RagEmbedder::stub());

        ingest_directory(
            root,
            Arc::clone(&db),
            Arc::clone(&embedder),
            allowed.clone(),
            manifest_path,
        )
        .expect("ingest");

        let store = RagStore::new(db.clone(), embedder, None, allowed);
        let rows = store
            .hierarchical_search(topic, store.rerank_candidates, 20)
            .expect("hierarchical_search");
        assert!(
            !rows.is_empty(),
            "hierarchical_search should return chunks for topic '{}' (FTS/graph_walk fallback)",
            topic
        );
        let with_date = rows.iter().filter(|r| r.last_updated.is_some()).count();
        assert!(
            with_date > 0,
            "workspace ingest sets last_updated from file mtime; expected at least one chunk with last_updated, got {}",
            with_date
        );
    }

    /// Web-like chunk (source = URL) then hybrid_search retrieves it (mirrors ingest_web_context then query_knowledge).
    #[test]
    fn integration_ingest_web_chunk_then_hybrid_search_retrieves_it() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let db_path = temp_dir.path().join("rag.db");
        let db = Arc::new(RagDb::open(&db_path).expect("open db"));
        let embedder = Arc::new(RagEmbedder::stub());
        let web_source = "https://example.com/page";
        let distinctive_text = "web chunk unique token alpha beta";
        let emb = vec![0.0f32; crate::rag::db::RAG_EMBED_DIM];
        db.upsert_chunk(
            &format!("{}#0", web_source),
            distinctive_text,
            web_source,
            "",
            "[]",
            "[]",
            "text",
            "section",
            "[]",
            Some(&emb),
            None,
            "web",
            "official",
        )
        .expect("upsert");
        let allowed: Vec<PathBuf> = vec![temp_dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| temp_dir.path().to_path_buf())];
        let store = RagStore::new(db, embedder, None, allowed);
        let rows = store
            .hybrid_search("web chunk unique token", 10, None)
            .expect("hybrid_search");
        assert!(
            !rows.is_empty(),
            "hybrid_search should retrieve web chunk (URL source allowed in path_under_allowed)"
        );
        assert!(
            rows.iter()
                .any(|r| r.source == web_source && r.text.contains("unique token")),
            "expected chunk from {} with matching text",
            web_source
        );
    }
}
