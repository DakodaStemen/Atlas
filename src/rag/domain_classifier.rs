//! Domain classification: classify_source (path → tag).

/// Simple path-based domain classification for RAG metrics.
/// Maps source file paths to high-level domains for training.jsonl tagging and analytics.
///
/// **Domain tags returned** (for consumers of `training.jsonl` and RAG metrics):
/// - **rust_std**, **react_docs**, **external_docs** — web sources (URL or `web/<domain>/` prefix)
/// - **fintech** — paths containing "payment", "billing"
/// - **infrastructure** — "db", "store", "repository"
/// - **routing** — "handler", "route", "controller"
/// - **auth** — "auth", "session"
/// - **api** — "api", "client"
/// - **testing** — "test", "spec"
/// - **general** — fallback for all other paths
///
/// Matching order: web/ prefix (then URL), then https:// URL, then path substrings (fintech → infrastructure → routing → auth → api → testing), then general.
pub fn classify_source(source: &str) -> &'static str {
    // web/<domain>/<url> prefix: use the domain segment for fine-tuning weight.
    if let Some(rest) = source.strip_prefix("web/") {
        if let Some(segment) = rest.split('/').next() {
            let d = segment.to_lowercase();
            return match d.as_str() {
                "rust_std" => "rust_std",
                "react_docs" => "react_docs",
                _ => "external_docs",
            };
        }
    }

    // URL-based: derive domain from host/path for training row tagging.
    if source.starts_with("https://") {
        let s = source.to_lowercase();
        if s.contains("doc.rust-lang.org") || s.contains("rust-lang.org") {
            return "rust_std";
        }
        if s.contains("react.dev") || s.contains("reactjs.org") {
            return "react_docs";
        }
        return "external_docs";
    }

    // Path substrings (checked in order): fintech → infrastructure → routing → auth → api → testing.
    let s = source.to_lowercase();
    if s.contains("payment") || s.contains("billing") {
        return "fintech";
    }
    if s.contains("db") || s.contains("store") || s.contains("repository") {
        return "infrastructure";
    }
    if s.contains("handler") || s.contains("route") || s.contains("controller") {
        return "routing";
    }
    if s.contains("auth") || s.contains("session") {
        return "auth";
    }
    if s.contains("api") || s.contains("client") {
        return "api";
    }
    if s.contains("test") || s.contains("spec") {
        return "testing";
    }
    "general"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// test_classify_source.
    fn test_classify_source() {
        assert_eq!(classify_source("src/billing/payment.rs"), "fintech");
        assert_eq!(classify_source("src/billing/invoice.ts"), "fintech");
        assert_eq!(classify_source("src/rag/db.rs"), "infrastructure");
        assert_eq!(classify_source("src/rag/store.rs"), "infrastructure");
        assert_eq!(classify_source("src/rag/repository.ts"), "infrastructure");
        assert_eq!(classify_source("src/api/handler.rs"), "routing");
        assert_eq!(classify_source("src/routes/users.ts"), "routing");
        assert_eq!(classify_source("src/auth/session.rs"), "auth");
        assert_eq!(classify_source("src/services/auth_service.py"), "auth");
        assert_eq!(classify_source("src/api/client.rs"), "api");
        assert_eq!(classify_source("src/external/api.ts"), "api");
        assert_eq!(classify_source("tests/integration_test.rs"), "testing");
        assert_eq!(classify_source("src/utils/helper.spec.ts"), "testing");
        assert_eq!(classify_source("src/main.rs"), "general");
        assert_eq!(classify_source("README.md"), "general");
        // Web URLs and web/ prefix
        assert_eq!(
            classify_source("https://doc.rust-lang.org/std/"),
            "rust_std"
        );
        assert_eq!(classify_source("https://react.dev/"), "react_docs");
        assert_eq!(classify_source("https://example.com/doc"), "external_docs");
        assert_eq!(
            classify_source("web/rust_std/https://doc.rust-lang.org/"),
            "rust_std"
        );
        assert_eq!(
            classify_source("web/react_docs/https://react.dev/"),
            "react_docs"
        );
        assert_eq!(
            classify_source("web/other/https://example.com/"),
            "external_docs"
        );
    }

    #[test]
    /// test_classify_source_edge_cases.
    fn test_classify_source_edge_cases() {
        assert_eq!(classify_source(""), "general");
        assert_eq!(classify_source("https://example.com"), "external_docs");
    }
}
