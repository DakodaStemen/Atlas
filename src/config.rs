//! Config: paths (data_dir, models_dir, db, nomic, reranker), env (DATA_DIR, ALLOWED_ROOTS),
//! web sources allowlist (web_sources_path, load_web_sources), and Config struct for RAG.
//! Default resolution: data_dir = DATA_DIR or ./data; models_dir = data_dir/models; allowed_roots = ALLOWED_ROOTS or (cwd + Windows Desktop/Work).
//!
//! Brain vs Vault: When HOLLOW_VAULT or VAULT_DIR is set, the vault directory holds the persistent RAG DB and training data (brain = workspace from ALLOWED_ROOTS/PRISM_ROOT; vault = shared knowledge). See env vars below.
//!
//! **Config::new() resolution order:** (1) vault_dir from VAULT_DIR/HOLLOW_VAULT; (2) data_dir from vault or DATA_DIR/PRISM_ROOT/./data; (3) models_dir = data_dir/models, then nomic/reranker via resolve_model_path (models_dir then data_dir then legacy); (4) db_path = data_dir/rag.db; (5) allowed_roots from ALLOWED_ROOTS or PRISM_ROOT or (cwd + Windows Desktop/Work); (6) optional GOLDEN_SET_DIR, TOOL_SELECTION_GUIDE_PATH, DESIGN_TOKENS_DIR, RULES_VAULT/GLOBAL_RULES_DIR.

use std::env;
use std::path::{Path, PathBuf};

/// Path to web sources allowlist: data_dir/web_sources.json (JSON array of { url, domain? }).
pub fn web_sources_path(data_dir: &Path) -> PathBuf {
    data_dir.join("web_sources.json")
}

/// One entry in web_sources.json: url (required), domain (optional for classify_source).
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
/// One web source entry: url (required), optional domain for classification.
pub struct WebSourceEntry {
    pub url: String,
    #[serde(default)]
    pub domain: Option<String>,
}

/// Load web sources allowlist from path. Returns empty vec if file missing or invalid.
/// Filter: only entries with url starting with https:// (after trim) are kept; others are dropped.
pub fn load_web_sources(path: &Path) -> Vec<WebSourceEntry> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    match serde_json::from_str::<Vec<WebSourceEntry>>(&content) {
        Ok(entries) => entries
            .into_iter()
            .filter(|e| e.url.trim().starts_with("https://"))
            .collect(),
        Err(_) => vec![],
    }
}

/// Doc manifest format: { "domain_name": [ "url1", "url2", ... ], ... }.
/// Flatten to Vec<WebSourceEntry> and write to output_path (e.g. data_dir/web_sources.json).
pub fn build_web_sources_from_doc_manifest(
    doc_manifest_path: &Path,
    output_path: &Path,
) -> Result<Vec<WebSourceEntry>, Box<dyn std::error::Error + Send + Sync>> {
    let content = std::fs::read_to_string(doc_manifest_path)?;
    let manifest: std::collections::HashMap<String, serde_json::Value> =
        serde_json::from_str(&content)?;
    let mut entries = Vec::new();
    for (domain, urls_value) in manifest {
        let urls: Vec<String> = match urls_value {
            serde_json::Value::Array(arr) => arr
                .into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            serde_json::Value::String(s) => vec![s],
            _ => continue,
        };
        for url in urls {
            let url = url.trim().to_string();
            if url.starts_with("https://") {
                entries.push(WebSourceEntry {
                    url,
                    domain: Some(domain.clone()),
                });
            }
        }
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(&entries)?;
    std::fs::write(output_path, json)?;
    eprintln!(
        "Wrote {} entries to {}",
        entries.len(),
        output_path.display()
    );
    Ok(entries)
}

/// Environment variables for paths:
/// - DATA_DIR: local data directory (default ./data when no vault).
/// - VAULT_DIR or HOLLOW_VAULT: when set, RAG DB and training data live here (vault); data_dir equals this. Brain/workspace = ALLOWED_ROOTS or PRISM_ROOT.
/// - PRISM_ROOT: when vault is set or for compatibility, can be used as single workspace root if ALLOWED_ROOTS is unset.
/// - ALLOWED_ROOTS: comma-separated workspace roots for ingest and file tools (default: cwd + Windows Desktop/Work, or PRISM_ROOT if set and ALLOWED_ROOTS unset).
/// - GOLDEN_SET_DIR: when set, approve_pattern writes golden_set.md and training.jsonl here (e.g. .prism) for Prism compatibility.
/// - TOOL_SELECTION_GUIDE_PATH: when set, get_tool_selection_guide reads from this path; otherwise first allowed root/docs/TOOL_SELECTION_GUIDE.md.
/// - DESIGN_TOKENS_DIR: when set, get_design_tokens reads from this dir; otherwise first allowed root/docs/design/data.
/// - RULES_VAULT or GLOBAL_RULES_DIR: when set, compile_rules reads global rules from this dir (e.g. Standards, Rules, Workflows).
#[derive(Clone, Debug)]
///   RAG paths (data_dir, models, db, nomic, reranker), allowed_roots, optional tool guide and design tokens paths.
pub struct Config {
    /// When set, RAG DB and training live here (vault). Otherwise data_dir is from DATA_DIR/PRISM_ROOT/./data.
    pub vault_dir: Option<PathBuf>,
    /// When set, Golden Vibe writes to this dir (golden_set.md, training.jsonl) for Prism-compatible paths.
    pub golden_set_dir: Option<PathBuf>,
    pub data_dir: PathBuf,
    pub models_dir: PathBuf,
    pub nomic_path: PathBuf,
    pub reranker_path: PathBuf,
    pub db_path: PathBuf,
    /// Workspace roots for ingest and file tools (brain). From ALLOWED_ROOTS or PRISM_ROOT.
    pub allowed_roots: Vec<PathBuf>,
    /// When set, get_tool_selection_guide reads from this path. From TOOL_SELECTION_GUIDE_PATH.
    pub tool_selection_guide_path: Option<PathBuf>,
    /// When set, get_design_tokens reads from this directory. From DESIGN_TOKENS_DIR.
    pub design_tokens_dir: Option<PathBuf>,
    /// When set, compile_rules reads global rules from this directory. From RULES_VAULT or GLOBAL_RULES_DIR.
    pub global_rules_dir: Option<PathBuf>,
}

/// Default delegates to Config::new() (env-based resolution).
impl Default for Config {
    /// Delegates to Config::new() (env-based).
    fn default() -> Self {
        Self::new()
    }
}

/// Read optional env var (None when unset or invalid). Used to avoid repeating env::var(key).ok().
fn opt_env(key: &str) -> Option<String> {
    env::var(key).ok()
}

/// Vault directory when HOLLOW_VAULT or VAULT_DIR is set (persistent RAG DB + training). None otherwise.
fn read_vault_dir() -> Option<PathBuf> {
    opt_env("VAULT_DIR")
        .or_else(|| opt_env("HOLLOW_VAULT"))
        .map(PathBuf::from)
}

/// Data directory when not using vault: DATA_DIR, or PRISM_ROOT/.prism, or ./data.
fn read_data_dir_fallback() -> PathBuf {
    opt_env("DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| opt_env("PRISM_ROOT").map(|s| PathBuf::from(s).join(".prism")))
        .unwrap_or_else(|| PathBuf::from("./data"))
}

/// PRISM_ROOT as optional path (for allowed_roots when ALLOWED_ROOTS is unset).
fn read_prism_root() -> Option<PathBuf> {
    opt_env("PRISM_ROOT").map(PathBuf::from)
}

/// Read env-derived config: vault_dir (optional), data_dir. Returns (vault_dir, data_dir).
fn read_config_env() -> (Option<PathBuf>, PathBuf) {
    let vault_dir = read_vault_dir();
    let data_dir = vault_dir.clone().unwrap_or_else(read_data_dir_fallback);
    (vault_dir, data_dir)
}

impl Config {
    /// data_dir from vault (VAULT_DIR/HOLLOW_VAULT) or DATA_DIR/PRISM_ROOT/./data; db_path = data_dir/rag.db; allowed_roots from ALLOWED_ROOTS or PRISM_ROOT or (cwd + Desktop/Work).
    pub fn new() -> Self {
        let (vault_dir, data_dir) = read_config_env();
        let models_dir = data_dir.join("models");

        // Model path resolution: prefer models_dir, then data_dir, then default to models_dir.
        let nomic_path = Self::resolve_model_path(
            &models_dir,
            &data_dir,
            "nomic-embed.onnx",
            Some("model.onnx"),
        );
        let reranker_path = Self::resolve_model_path(&models_dir, &data_dir, "reranker.onnx", None);

        let db_path = data_dir.join("rag.db");
        let allowed_roots = Self::resolve_allowed_roots(read_prism_root());
        let golden_set_dir = opt_env("GOLDEN_SET_DIR").map(PathBuf::from);
        let tool_selection_guide_path = opt_env("TOOL_SELECTION_GUIDE_PATH").map(PathBuf::from);
        let design_tokens_dir = opt_env("DESIGN_TOKENS_DIR").map(PathBuf::from);
        let global_rules_dir = opt_env("RULES_VAULT")
            .or_else(|| opt_env("GLOBAL_RULES_DIR"))
            .map(PathBuf::from);

        Self {
            vault_dir,
            golden_set_dir,
            data_dir,
            models_dir,
            nomic_path,
            reranker_path,
            db_path,
            allowed_roots,
            tool_selection_guide_path,
            design_tokens_dir,
            global_rules_dir,
        }
    }

    /// Resolve a model file path. Return order: models_dir/primary if exists, else data_dir/primary, else data_dir/legacy (if given), else models_dir/primary as default.
    /// Resolves ONNX model path: first models_dir/primary, then data_dir/primary, then data_dir/legacy_in_data if provided, else models_dir/primary (default).
    pub(crate) fn resolve_model_path(
        models_dir: &Path,
        data_dir: &Path,
        primary: &str,
        legacy_in_data: Option<&str>,
    ) -> PathBuf {
        let in_models = models_dir.join(primary);
        if in_models.exists() {
            return in_models;
        }
        let in_data = data_dir.join(primary);
        if in_data.exists() {
            return in_data;
        }
        if let Some(legacy) = legacy_in_data {
            let legacy_path = data_dir.join(legacy);
            if legacy_path.exists() {
                return legacy_path;
            }
        }
        in_models
    }

    /// Resolve ALLOWED_ROOTS (comma-separated), or PRISM_ROOT as single workspace root, or default (cwd + Windows Desktop/Work). Canonicalized.
    /// ZIO bounty core path is appended when present so RAG retrieval can return ingested ZScheduler/Executor chunks.
    fn resolve_allowed_roots(prism_root: Option<PathBuf>) -> Vec<PathBuf> {
        let var = opt_env("ALLOWED_ROOTS").unwrap_or_default();
        let mut raw: Vec<PathBuf> = if !var.is_empty() {
            var.split(',')
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        } else if let Some(prism) = prism_root {
            vec![prism]
        } else {
            let base = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let mut roots = vec![base];
            if let Ok(home) = env::var("USERPROFILE") {
                roots.push(PathBuf::from(home).join("Desktop").join("Work"));
            }
            roots
        };
        // ZIO bounty core: allow RAG retrieval for ingested ZScheduler.scala / Executor.scala.
        if let Ok(zio_path) = env::var("ZIO_BOUNTY_CORE_PATH") {
            let zio_bounty_core = PathBuf::from(zio_path);
            if zio_bounty_core.exists() && !raw.iter().any(|r| r == &zio_bounty_core) {
                raw.push(zio_bounty_core);
            }
        }
        raw.into_iter()
            .map(|p| p.canonicalize().unwrap_or(p))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    /// Prefer models_dir when primary exists there.
    fn resolve_model_path_prefers_models_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let models_dir = tmp.path().join("models");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&models_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        let primary = "model.onnx";
        fs::write(models_dir.join(primary), b"").unwrap();
        fs::write(data_dir.join(primary), b"").unwrap();
        let out = Config::resolve_model_path(&models_dir, &data_dir, primary, None);
        assert_eq!(out, models_dir.join(primary));
    }

    #[test]
    /// Fall back to data_dir when primary not in models_dir.
    fn resolve_model_path_falls_back_to_data_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let models_dir = tmp.path().join("models");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&models_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        let primary = "model.onnx";
        fs::write(data_dir.join(primary), b"").unwrap();
        let out = Config::resolve_model_path(&models_dir, &data_dir, primary, None);
        assert_eq!(out, data_dir.join(primary));
    }

    #[test]
    /// Use legacy path in data_dir when primary missing and legacy given.
    fn resolve_model_path_uses_legacy_in_data_when_no_primary() {
        let tmp = tempfile::tempdir().unwrap();
        let models_dir = tmp.path().join("models");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&models_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        fs::write(data_dir.join("legacy.onnx"), b"").unwrap();
        let out = Config::resolve_model_path(
            &models_dir,
            &data_dir,
            "nomic-embed.onnx",
            Some("legacy.onnx"),
        );
        assert_eq!(out, data_dir.join("legacy.onnx"));
    }

    #[test]
    /// Default path is models_dir when file missing (no fallback).
    fn resolve_model_path_defaults_to_models_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let models_dir = tmp.path().join("models");
        let data_dir = tmp.path().join("data");
        fs::create_dir_all(&models_dir).unwrap();
        fs::create_dir_all(&data_dir).unwrap();
        let out = Config::resolve_model_path(&models_dir, &data_dir, "missing.onnx", None);
        assert_eq!(out, models_dir.join("missing.onnx"));
    }

    #[test]
    /// Config::new() has at least one allowed root (e.g. cwd).
    fn config_default_allowed_roots_non_empty() {
        let config = Config::new();
        assert!(
            !config.allowed_roots.is_empty(),
            "default allowed_roots should include at least cwd"
        );
    }

    #[test]
    /// When ALLOWED_ROOTS and PRISM_ROOT are unset, first allowed root is current dir.
    fn config_default_allowed_roots_first_is_cwd_when_unset() {
        if std::env::var_os("ALLOWED_ROOTS").is_some() || std::env::var_os("PRISM_ROOT").is_some() {
            return; // skip when env overrides default
        }
        let config = Config::new();
        let cwd = std::env::current_dir().unwrap().canonicalize().unwrap();
        let first = config.allowed_roots[0].canonicalize().unwrap();
        assert_eq!(
            first, cwd,
            "when ALLOWED_ROOTS/PRISM_ROOT unset, first root should be cwd"
        );
    }

    #[test]
    /// web_sources_path joins data_dir with web_sources.json.
    fn web_sources_path_joins_data_dir_with_filename() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path();
        let path = web_sources_path(data_dir);
        assert_eq!(path, data_dir.join("web_sources.json"));
    }

    #[test]
    /// Missing file returns empty vec.
    fn load_web_sources_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nonexistent.json");
        assert!(load_web_sources(&path).is_empty());
    }

    #[test]
    /// Invalid JSON returns empty vec.
    fn load_web_sources_invalid_json_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.json");
        std::fs::write(&path, "not valid json").unwrap();
        assert!(load_web_sources(&path).is_empty());
    }

    #[test]
    /// Only https:// URLs are kept; http dropped.
    fn load_web_sources_filters_non_https_urls() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sources.json");
        let json = r#"[{"url":"https://example.com/a"},{"url":"http://insecure.com/b"}]"#;
        std::fs::write(&path, json).unwrap();
        let entries = load_web_sources(&path);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].url, "https://example.com/a");
    }

    /// A3 Brain vs Vault: when VAULT_DIR is set, vault_dir is Some and db_path is under it.
    #[test]
    /// When VAULT_DIR set, vault_dir is Some and db_path under it.
    fn config_vault_dir_and_db_path_when_vault_dir_set() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = tmp
            .path()
            .join("vault")
            .canonicalize()
            .unwrap_or_else(|_| tmp.path().join("vault"));
        std::fs::create_dir_all(&vault).ok();
        let vault_str = vault.to_string_lossy().to_string();
        std::env::set_var("VAULT_DIR", &vault_str);
        let config = Config::new();
        std::env::remove_var("VAULT_DIR");
        assert!(
            config.vault_dir.is_some(),
            "vault_dir must be set when VAULT_DIR is set"
        );
        let v = config.vault_dir.as_ref().unwrap();
        assert!(
            config.db_path.starts_with(v),
            "db_path must be under vault_dir"
        );
        assert_eq!(
            config.data_dir, *v,
            "data_dir must equal vault_dir when vault is set"
        );
    }
}
