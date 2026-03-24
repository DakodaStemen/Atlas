use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Complexity cap constants (see COGNITIVE_CEILING.md). Files over FILE_LINE_CAP require a Breakdown Plan before refactor.
pub(crate) const COMPLEXITY_FILE_LINE_CAP: usize = 500;
/// Function line cap (50); reserved for future function-level reporting in complexity CLI (COGNITIVE_CEILING.md).
#[allow(dead_code)]
pub(crate) const COMPLEXITY_FN_LINE_CAP: usize = 50;

/// Run complexity report: scan .rs files under path, count lines, flag files over COMPLEXITY_FILE_LINE_CAP.
pub(crate) fn run_complexity(
    path: Option<&Path>,
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = path
        .map(PathBuf::from)
        .or_else(|| config.allowed_roots.first().cloned())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let root = root.canonicalize().unwrap_or(root);
    let mut entries: Vec<(PathBuf, usize)> = Vec::new();
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let n = e.file_name().to_string_lossy();
            !(n == "target" || n == ".git" || n == "node_modules")
        })
        .filter_map(|e| e.ok())
    {
        let p = entry.path();
        if p.extension().is_some_and(|e| e == "rs") && p.is_file() {
            let lines = std::fs::read_to_string(p)
                .map(|s| s.lines().count())
                .unwrap_or(0);
            entries.push((p.to_path_buf(), lines));
        }
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    let over_cap: Vec<_> = entries
        .iter()
        .filter(|(_, lines)| *lines > COMPLEXITY_FILE_LINE_CAP)
        .collect();
    println!(
        "complexity: scanned {} .rs files under {} (cap {} lines)",
        entries.len(),
        root.display(),
        COMPLEXITY_FILE_LINE_CAP
    );
    for (path, lines) in &entries {
        let flag = if *lines > COMPLEXITY_FILE_LINE_CAP {
            " [OVER CAP]"
        } else {
            ""
        };
        println!("  {}: {}{}", path.display(), lines, flag);
    }
    if !over_cap.is_empty() {
        eprintln!(
            "complexity: {} file(s) over cap (Breakdown Plan required before refactor; see COGNITIVE_CEILING.md).",
            over_cap.len()
        );
    }
    Ok(())
}

/// Audit env: ORT, Nomic, Reranker paths.
pub(crate) fn run_audit(config: &rag_mcp::config::Config) {
    // ORT
    match std::env::var("ORT_DYLIB_PATH") {
        Ok(path) => {
            let p = PathBuf::from(&path);
            if p.exists() {
                eprintln!("ORT: OK ({})", p.display());
            } else {
                eprintln!("ORT: path set but file not found ({})", p.display());
            }
        }
        Err(_) => eprintln!("ORT: not set (set ORT_DYLIB_PATH for semantic search)"),
    }
    // Nomic
    let nomic_tokenizer = config.nomic_path.with_file_name("tokenizer.json");
    if config.nomic_path.exists() && nomic_tokenizer.exists() {
        eprintln!("Nomic: OK");
    } else if !config.nomic_path.exists() {
        eprintln!("Nomic: missing ({})", config.nomic_path.display());
    } else {
        eprintln!(
            "Nomic: model OK, tokenizer missing ({})",
            nomic_tokenizer.display()
        );
    }
    // Reranker
    let reranker_tokenizer = config
        .reranker_path
        .with_file_name("reranker-tokenizer.json");
    if config.reranker_path.exists() && reranker_tokenizer.exists() {
        eprintln!("Reranker: OK");
    } else if !config.reranker_path.exists() {
        eprintln!("Reranker: missing ({})", config.reranker_path.display());
    } else {
        eprintln!(
            "Reranker: model OK, tokenizer missing ({})",
            reranker_tokenizer.display()
        );
    }
}

/// Parse lessons_learned.md into sections (## YYYY-MM-DD: Title + body).
pub(crate) fn parse_lessons_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let blocks: Vec<&str> = content.split("\n## ").collect();
    for (i, block) in blocks.iter().enumerate() {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        let first_line = block.lines().next().unwrap_or("");
        // Skip: (first block and title-only header line) OR empty first line.
        let skip_leading_title =
            i == 0 && first_line.starts_with('#') && !first_line.contains(": ");
        if skip_leading_title || first_line.is_empty() {
            continue;
        }
        let title = first_line.trim_start_matches("## ").trim().to_string();
        let body = block.lines().skip(1).collect::<Vec<_>>().join("\n");
        if !title.is_empty() {
            sections.push((title, body));
        }
    }
    sections
}

/// System prompt for review-lessons: classify each ## section as Valid or Anti-Pattern.
pub(crate) const LESSONS_REVIEW_PROMPT: &str = "You are a senior engineer reviewing a project's lessons_learned.md (memory for an autonomous coding agent). \
For each lesson section (each ## heading), classify it as either Valid (good practice, keep) or Anti-Pattern (bad practice, hack, or misleading—should be pruned or rewritten). \
Reply with exactly one line per section in this format: SECTION_TITLE -> Valid  or  SECTION_TITLE -> Anti-Pattern: one-line reason. \
Use the exact section title from the document (the text after ##).";

/// Call OpenAI API to classify lessons as Valid or Anti-Pattern.
pub(crate) fn call_openai_lessons_review(
    content: &str,
    api_key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let prompt = format!(
        "{}\n\n<lessons_learned>\n{}\n</lessons_learned>",
        LESSONS_REVIEW_PROMPT, content
    );
    let body = serde_json::json!({
        "model": "gpt-4",
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 2048
    });
    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().unwrap_or_default();
        return Err(format!("OpenAI API {}: {}", status, text).into());
    }
    let json: serde_json::Value = res.json()?;
    let text = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .unwrap_or("(no content)")
        .to_string();
    Ok(format!("## API classification\n\n```\n{}\n```\n\n", text))
}

/// Call Anthropic API to classify lessons as Valid or Anti-Pattern.
pub(crate) fn call_anthropic_lessons_review(
    content: &str,
    api_key: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let prompt = format!(
        "{}\n\n<lessons_learned>\n{}\n</lessons_learned>",
        LESSONS_REVIEW_PROMPT, content
    );
    let body = serde_json::json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 2048,
        "messages": [{"role": "user", "content": prompt}]
    });
    let res = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&body)
        .send()?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().unwrap_or_default();
        return Err(format!("Anthropic API {}: {}", status, text).into());
    }
    let json: serde_json::Value = res.json()?;
    let text = json
        .get("content")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .and_then(|b| b.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("(no content)")
        .to_string();
    Ok(format!("## API classification\n\n```\n{}\n```\n\n", text))
}

/// Memory Review (Janitor): produce lessons_audit_YYYY-MM-DD.md. Optional API call if OPENAI_API_KEY or ANTHROPIC_API_KEY set.
pub(crate) fn run_review_lessons(
    config: &rag_mcp::config::Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let root = config
        .allowed_roots
        .first()
        .cloned()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let docs_dir = root.join("docs");
    let lessons_path = docs_dir.join("lessons_learned.md");
    if !lessons_path.exists() {
        eprintln!(
            "review-lessons: {} not found. Create docs/lessons_learned.md first.",
            lessons_path.display()
        );
        std::process::exit(1);
    }
    let content = std::fs::read_to_string(&lessons_path)?;
    let sections = parse_lessons_sections(&content);
    let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
    let audit_path = docs_dir.join(format!("lessons_audit_{}.md", date_str));

    let mut report = format!(
        "# Memory Review (Janitor) — {}\n\nGenerated by `rag-mcp review-lessons`. Use this to prune or correct anti-patterns in docs/lessons_learned.md.\n\n",
        date_str
    );

    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        if !api_key.is_empty() {
            match call_openai_lessons_review(&content, &api_key) {
                Ok(api_report) => {
                    report.push_str(&api_report);
                    std::fs::create_dir_all(&docs_dir)?;
                    std::fs::write(&audit_path, &report)?;
                    eprintln!(
                        "review-lessons: wrote API audit to {}",
                        audit_path.display()
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!(
                        "review-lessons: OpenAI API failed ({}), writing manual template.",
                        e
                    );
                }
            }
        }
    }
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        if !api_key.is_empty() {
            match call_anthropic_lessons_review(&content, &api_key) {
                Ok(api_report) => {
                    report.push_str(&api_report);
                    std::fs::create_dir_all(&docs_dir)?;
                    std::fs::write(&audit_path, &report)?;
                    eprintln!(
                        "review-lessons: wrote API audit to {}",
                        audit_path.display()
                    );
                    return Ok(());
                }
                Err(e) => {
                    eprintln!(
                        "review-lessons: Anthropic API failed ({}), writing manual template.",
                        e
                    );
                }
            }
        }
    }

    for (title, body) in &sections {
        let body_preview = body.lines().take(5).collect::<Vec<_>>().join("\n");
        report.push_str(&format!(
            "## {}\n\n```\n{}\n```\n\nStatus: [ ] Valid  [ ] Anti-Pattern\n\n---\n\n",
            title,
            if body_preview.len() > 400 {
                format!("{}...", body_preview.chars().take(400).collect::<String>())
            } else {
                body_preview
            }
        ));
    }
    std::fs::create_dir_all(&docs_dir)?;
    std::fs::write(&audit_path, &report)?;
    eprintln!(
        "review-lessons: wrote manual audit template to {}. Set OPENAI_API_KEY or ANTHROPIC_API_KEY for automated classification.",
        audit_path.display()
    );
    Ok(())
}
