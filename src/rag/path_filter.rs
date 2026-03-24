//! Shared path filtering: whether a path is under allowed roots or is an allowed web URL.
//! Used by store (retrieval output filtering) and ingest (should_ingest).

use std::path::{Path, PathBuf};

/// True if path string is an http(s) URL. Web-ingested chunks use URL sources.
pub fn is_web_source_path(path: &str) -> bool {
    path.starts_with("http://") || path.starts_with("https://")
}

/// True if path is under an allowed root, or (when `allow_web_sources`) is an http(s) URL.
/// Paths are canonicalized for comparison; URLs are always allowed when `allow_web_sources` is true.
/// Also allows the logical path `research/master_research.md` and the path in env `MASTER_RESEARCH_SOURCE` (when set) for Godly RAG.
pub fn path_under_allowed(
    path: impl AsRef<Path>,
    allowed: &[PathBuf],
    allow_web_sources: bool,
) -> bool {
    let path = path.as_ref();
    if allow_web_sources {
        let s = path.to_string_lossy();
        if s.starts_with("http://") || s.starts_with("https://") {
            return true;
        }
    }
    // Logical master research source (Godly RAG) is not a filesystem path; allow it so query_master_research output is not filtered out.
    let s = path.to_string_lossy();
    if s == "research/master_research.md" {
        return true;
    }
    if let Ok(var) = std::env::var("MASTER_RESEARCH_SOURCE") {
        if !var.is_empty() && s == var {
            return true;
        }
    }
    let abs = path.canonicalize().unwrap_or_else(|e| {
        tracing::debug!(
            "path_under_allowed: canonicalize failed for {:?}: {}",
            path,
            e
        );
        path.to_path_buf()
    });
    allowed.iter().any(|r| abs.starts_with(r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn is_web_source_path_http_https() {
        assert!(is_web_source_path("http://example.com"));
        assert!(is_web_source_path("https://example.com/path"));
        assert!(!is_web_source_path("file:///tmp/x"));
        assert!(!is_web_source_path("/local/path"));
    }

    #[test]
    fn path_under_allowed_web_sources_url() {
        let allowed = vec![PathBuf::from("/allowed")];
        assert!(path_under_allowed("https://example.com", &allowed, true));
        assert!(!path_under_allowed("https://example.com", &allowed, false));
    }

    #[test]
    fn path_under_allowed_canonicalize_failure() {
        let allowed = vec![PathBuf::from("C:\\nonexistent")];
        let bad = Path::new("C:\\nonexistent_xyz_123\\file.txt");
        let r = path_under_allowed(bad, &allowed, false);
        assert!(!r);
    }
}
