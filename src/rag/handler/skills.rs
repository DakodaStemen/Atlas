//! Progressive disclosure skills: list_skill_metadata, get_skill_content, get_skill_reference.
//! When RULES_VAULT/GLOBAL_RULES_DIR is set, skills are read from global_rules_dir/02_Skills or skills (repo root).

use super::{
    format_response::apply_response_format, AgenticHandler, IngestionProvider, VectorStoreProvider,
    TRUNCATION_SUFFIX,
};
use rmcp::model::{CallToolResult, Content};
use rmcp::ErrorData as McpError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const SKILLS_SUBDIR: &str = "02_Skills";
const MAX_SKILL_CONTENT_WORDS: usize = 5000;

fn skills_root(global_rules_dir: Option<&std::path::PathBuf>) -> Option<std::path::PathBuf> {
    let dir = global_rules_dir?.join(SKILLS_SUBDIR);
    if dir.exists() {
        Some(dir)
    } else {
        global_rules_dir
            .filter(|d| d.join("skills").exists())
            .map(|d| d.join("skills"))
            .or_else(|| {
                global_rules_dir
                    .filter(|d| d.join("Skills").exists())
                    .map(|d| d.join("Skills"))
            })
    }
}

/// Extracted skill metadata from frontmatter or body.
struct SkillMeta {
    name: String,
    description: String,
    domain: Option<String>,
    category: Option<String>,
    tags: Vec<String>,
}

/// Extract name, description, domain, category, and tags from markdown (frontmatter or first 100 words of body).
fn parse_skill_meta(content: &str) -> SkillMeta {
    let content = content.trim();
    if content.len() < 4 && content.starts_with("---") {
        return SkillMeta {
            name: "Skill".to_string(),
            description: String::new(),
            domain: None,
            category: None,
            tags: vec![],
        };
    }
    let (name, rest, domain, category, tags) = if content.starts_with("---") {
        let end = content[4..].find("\n---").map(|i| i + 4).unwrap_or(0);
        let fm = &content[4..end];
        let name = fm
            .lines()
            .find(|l| l.starts_with("name:") || l.starts_with("title:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_else(|| "Skill".to_string());
        let desc = fm
            .lines()
            .find(|l| l.starts_with("description:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            .unwrap_or_else(|| {
                content[end + 7..]
                    .lines()
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string()
            });
        let domain = fm
            .lines()
            .find(|l| l.starts_with("domain:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            .filter(|s| !s.is_empty());
        let category = fm
            .lines()
            .find(|l| l.starts_with("category:"))
            .map(|l| l.split(':').nth(1).unwrap_or("").trim().to_string())
            .filter(|s| !s.is_empty());
        let tags = fm
            .lines()
            .find(|l| l.starts_with("tags:"))
            .map(|l| {
                let v = l.split(':').nth(1).unwrap_or("").trim();
                if v.starts_with('[') {
                    serde_json::from_str::<Vec<String>>(v).unwrap_or_else(|_| vec![])
                } else {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                }
            })
            .unwrap_or_default();
        (name, desc, domain, category, tags)
    } else {
        let first_line = content.lines().next().unwrap_or("").trim();
        let name = first_line.trim_start_matches('#').trim().to_string();
        let desc = content.lines().nth(1).unwrap_or("").trim().to_string();
        (name, desc, None, None, vec![])
    };
    let desc_short = rest
        .split_whitespace()
        .take(100)
        .collect::<Vec<_>>()
        .join(" ");
    SkillMeta {
        name,
        description: desc_short,
        domain,
        category,
        tags,
    }
}

pub async fn list_skill_metadata_impl<I, S>(
    handler: &AgenticHandler<I, S>,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let root = match skills_root(handler.global_rules_dir.as_ref()) {
        Some(r) => r,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "list_skill_metadata: Set RULES_VAULT or GLOBAL_RULES_DIR and ensure 02_Skills/ or skills/ exists.",
            )]));
        }
    };
    let mut items = Vec::new();
    for entry in walkdir::WalkDir::new(&root)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension() {
                if ext == "md" || ext == "mdc" {
                    let id = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    if let Ok(content) = std::fs::read_to_string(p) {
                        let meta = parse_skill_meta(&content);
                        let mut item = serde_json::json!({
                            "id": id,
                            "name": meta.name,
                            "description": meta.description
                        });
                        if let Some(d) = meta.domain {
                            item["domain"] = serde_json::json!(d);
                        }
                        if let Some(c) = meta.category {
                            item["category"] = serde_json::json!(c);
                        }
                        if !meta.tags.is_empty() {
                            item["tags"] = serde_json::json!(meta.tags);
                        }
                        items.push(item);
                    }
                }
            }
        }
    }
    let json = serde_json::to_string_pretty(&items).unwrap_or_else(|_| "[]".to_string());
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

pub async fn get_skill_content_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    skill_id: String,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let root = match skills_root(handler.global_rules_dir.as_ref()) {
        Some(r) => r,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "get_skill_content: Set RULES_VAULT or GLOBAL_RULES_DIR and ensure 02_Skills/ or skills/ exists.",
            )]));
        }
    };
    let skill_id = skill_id.trim();
    if skill_id.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_skill_content: skill_id is required.",
        )]));
    }
    let md_path = root.join(format!("{}.md", skill_id));
    let mdc_path = root.join(format!("{}.mdc", skill_id));
    let path = if md_path.exists() {
        md_path.canonicalize().unwrap_or(md_path)
    } else if mdc_path.exists() {
        mdc_path.canonicalize().unwrap_or(mdc_path)
    } else {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "get_skill_content: skill not found: {}",
            skill_id
        ))]));
    };
    if !path.starts_with(&root) {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_skill_content: path escape not allowed.",
        )]));
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "get_skill_content: read error: {}",
                e
            ))]));
        }
    };
    let words: Vec<&str> = content.split_whitespace().collect();
    let out = if words.len() > MAX_SKILL_CONTENT_WORDS {
        let truncated = words[..MAX_SKILL_CONTENT_WORDS].join(" ");
        format!("{}{}", truncated, TRUNCATION_SUFFIX)
    } else {
        content
    };
    Ok(CallToolResult::success(vec![Content::text(
        apply_response_format(out),
    )]))
}

pub async fn get_skill_reference_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    skill_id: String,
    path: String,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let root = match skills_root(handler.global_rules_dir.as_ref()) {
        Some(r) => r,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                "get_skill_reference: Set RULES_VAULT or GLOBAL_RULES_DIR and ensure 02_Skills/ or skills/ exists.",
            )]));
        }
    };
    let skill_id = skill_id.trim();
    let path = path.trim().replace('\\', "/");
    if skill_id.is_empty() || path.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_skill_reference: skill_id and path are required.",
        )]));
    }
    let skill_dir = root.join(skill_id);
    let ref_path = skill_dir.join("references").join(&path);
    let ref_path = ref_path.canonicalize().unwrap_or(ref_path);
    if !ref_path.exists() || !ref_path.is_file() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "get_skill_reference: file not found: {}",
            path
        ))]));
    }
    if !ref_path.starts_with(&skill_dir) {
        return Ok(CallToolResult::success(vec![Content::text(
            "get_skill_reference: path escape not allowed.",
        )]));
    }
    let content = match std::fs::read_to_string(&ref_path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "get_skill_reference: read error: {}",
                e
            ))]));
        }
    };
    let words: Vec<&str> = content.split_whitespace().collect();
    let out = if words.len() > MAX_SKILL_CONTENT_WORDS {
        let truncated = words[..MAX_SKILL_CONTENT_WORDS].join(" ");
        format!("{}{}", truncated, TRUNCATION_SUFFIX)
    } else {
        content
    };
    Ok(CallToolResult::success(vec![Content::text(
        apply_response_format(out),
    )]))
}

const VALIDATE_SKILL_MAX_CHARS: usize = 50_000;
const VALIDATE_SKILL_REQUIRED_HEADINGS: &[&str] = &["## Purpose", "## When to use"];

/// Validate a skill file: frontmatter (name/title, description), optional required headings, max size.
pub async fn validate_skill_impl<I, S>(
    handler: &AgenticHandler<I, S>,
    skill_id: String,
) -> Result<CallToolResult, McpError>
where
    I: IngestionProvider + Send + Sync,
    S: VectorStoreProvider + Send + Sync,
{
    let root = match skills_root(handler.global_rules_dir.as_ref()) {
        Some(r) => r,
        None => {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "valid": false,
                    "errors": ["Set RULES_VAULT or GLOBAL_RULES_DIR and ensure 02_Skills/ or skills/ exists."],
                    "warnings": []
                }).to_string()
            )]));
        }
    };
    let skill_id = skill_id.trim();
    if skill_id.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "valid": false,
                "errors": ["skill_id is required."],
                "warnings": []
            })
            .to_string(),
        )]));
    }
    let md_path = root.join(format!("{}.md", skill_id));
    let mdc_path = root.join(format!("{}.mdc", skill_id));
    let path = if md_path.exists() {
        md_path.canonicalize().unwrap_or(md_path)
    } else if mdc_path.exists() {
        mdc_path.canonicalize().unwrap_or(mdc_path)
    } else {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "valid": false,
                "errors": [format!("Skill not found: {}", skill_id)],
                "warnings": []
            })
            .to_string(),
        )]));
    };
    if !path.starts_with(&root) {
        return Ok(CallToolResult::success(vec![Content::text(
            serde_json::json!({
                "valid": false,
                "errors": ["Path escape not allowed."],
                "warnings": []
            })
            .to_string(),
        )]));
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            return Ok(CallToolResult::success(vec![Content::text(
                serde_json::json!({
                    "valid": false,
                    "errors": [format!("Read error: {}", e)],
                    "warnings": []
                })
                .to_string(),
            )]));
        }
    };
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    if content.len() > VALIDATE_SKILL_MAX_CHARS {
        warnings.push(format!(
            "Skill exceeds {} chars ({}); consider splitting.",
            VALIDATE_SKILL_MAX_CHARS,
            content.len()
        ));
    }
    let has_frontmatter = content.trim().starts_with("---");
    if has_frontmatter {
        let end = content[4..].find("\n---").map(|i| i + 4).unwrap_or(0);
        let fm = &content[4..end];
        let has_name = fm
            .lines()
            .any(|l| l.starts_with("name:") || l.starts_with("title:"));
        let has_desc = fm.lines().any(|l| l.starts_with("description:"));
        if !has_name {
            errors.push("Frontmatter must contain 'name:' or 'title:'.".to_string());
        }
        if !has_desc {
            errors.push("Frontmatter must contain 'description:'.".to_string());
        }
    } else {
        warnings.push("No YAML frontmatter; consider adding name, description.".to_string());
    }
    for heading in VALIDATE_SKILL_REQUIRED_HEADINGS {
        if !content.contains(heading) {
            warnings.push(format!("Consider adding section '{}'.", heading));
        }
    }
    let valid = errors.is_empty();
    let result = serde_json::json!({
        "valid": valid,
        "errors": errors,
        "warnings": warnings
    });
    Ok(CallToolResult::success(vec![Content::text(
        result.to_string(),
    )]))
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListSkillMetadataParams {}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateSkillParams {
    #[serde(default)]
    pub skill_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSkillContentParams {
    #[serde(default)]
    pub skill_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct GetSkillReferenceParams {
    #[serde(default)]
    pub skill_id: String,
    #[serde(default)]
    pub path: String,
}
