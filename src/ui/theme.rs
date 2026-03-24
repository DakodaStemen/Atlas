//! Design token parser: Colors.csv and Typography.csv from docs/design/data.
//! Exposes parsed tokens and ANSI color helpers for CLI output.
//!
//! **Base path resolution:** `load_theme(base)` takes an optional base path (e.g. repo root).
//! When `base` is `None` or the path does not exist, returns a default empty `Theme` (no disk I/O).
//! Otherwise resolves `base/docs/design/data/Colors.csv` and `Typography.csv`; missing or
//! unparseable files yield empty maps/vecs inside the theme; the function never panics.

use std::collections::HashMap;
use std::path::Path;

/// One color token from Colors.csv (token, value hex, usage).
#[derive(Clone, Debug, Default)]
pub struct ColorToken {
    pub token: String,
    pub value: String,
    pub usage: String,
}

/// One typography row from Typography.csv.
#[derive(Clone, Debug, Default)]
pub struct TypographyToken {
    pub role: String,
    pub font_stack: String,
    pub scale_rem: String,
    pub weight: String,
    pub usage: String,
}

/// Parsed theme: colors and typography. Empty if CSVs missing or unparseable.
#[derive(Clone, Debug, Default)]
pub struct Theme {
    pub colors: HashMap<String, ColorToken>,
    pub typography: Vec<TypographyToken>,
}

/// ANSI escape for reset.
pub const ANSI_RESET: &str = "\x1b[0m";
/// ANSI bright red (error).
pub const ANSI_RED: &str = "\x1b[31m";
/// ANSI green (success/primary).
pub const ANSI_GREEN: &str = "\x1b[32m";
/// ANSI yellow (warning).
pub const ANSI_YELLOW: &str = "\x1b[33m";
/// ANSI blue (info/links).
pub const ANSI_BLUE: &str = "\x1b[34m";

/// Map design token name to ANSI code (for CLI). Uses hardcoded fallbacks so CLI works without CSVs.
fn ansi_for_token_name(token: &str) -> Option<&'static str> {
    match token {
        "emerald-500" | "emerald-600" => Some(ANSI_GREEN),
        "red-500" => Some(ANSI_RED),
        "amber-500" => Some(ANSI_YELLOW),
        "blue-500" => Some(ANSI_BLUE),
        _ => None,
    }
}

/// Load theme from a base path (e.g. repo root). Resolves docs/design/data/Colors.csv and Typography.csv.
/// Returns default empty theme on missing files or parse errors; never panics.
pub fn load_theme(base: Option<&Path>) -> Theme {
    let base = match base {
        Some(b) if b.exists() => b,
        _ => return Theme::default(),
    };
    let data_dir = base.join("docs").join("design").join("data");
    let colors_path = data_dir.join("Colors.csv");
    let typography_path = data_dir.join("Typography.csv");

    let colors = parse_colors_csv(&colors_path);
    let typography = parse_typography_csv(&typography_path);

    Theme { colors, typography }
}

/// Parse Colors.csv: token,value,usage. Skips header; skips malformed lines.
fn parse_colors_csv(path: &Path) -> HashMap<String, ColorToken> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let mut out = HashMap::new();
    for (i, line) in content.lines().enumerate() {
        if i == 0 && line.trim().eq_ignore_ascii_case("token,value,usage") {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, ',').map(|s| s.trim()).collect();
        if parts.len() >= 2 {
            let token = parts[0].to_string();
            let value = parts[1].to_string();
            let usage = parts.get(2).map(|s| s.to_string()).unwrap_or_default();
            out.insert(
                token.clone(),
                ColorToken {
                    token,
                    value,
                    usage,
                },
            );
        }
    }
    out
}

/// Parse Typography.csv: role,font_stack,scale_rem,weight,usage.
fn parse_typography_csv(path: &Path) -> Vec<TypographyToken> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        if i == 0 && line.trim().to_lowercase().starts_with("role,") {
            continue;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(5, ',').map(|s| s.trim()).collect();
        if parts.len() >= 4 {
            out.push(TypographyToken {
                role: parts[0].to_string(),
                font_stack: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                scale_rem: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                weight: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                usage: parts.get(4).map(|s| s.to_string()).unwrap_or_default(),
            });
        }
    }
    out
}

/// Return ANSI code for a semantic role: success (green), error (red), warning (yellow). Maps design tokens to ANSI.
pub fn ansi_for_role(_theme: &Theme, role: &str) -> &'static str {
    match role {
        "success" | "primary" => ansi_for_token_name("emerald-500").unwrap_or(ANSI_GREEN),
        "error" => ansi_for_token_name("red-500").unwrap_or(ANSI_RED),
        "warning" => ansi_for_token_name("amber-500").unwrap_or(ANSI_YELLOW),
        "info" => ansi_for_token_name("blue-500").unwrap_or(ANSI_BLUE),
        _ => ANSI_RESET,
    }
}

/// Wrap text with ANSI color for success (green), then reset.
pub fn wrap_success(s: &str) -> String {
    format!("{}{}{}", ANSI_GREEN, s, ANSI_RESET)
}

/// Wrap text with ANSI color for error (red), then reset.
pub fn wrap_error(s: &str) -> String {
    format!("{}{}{}", ANSI_RED, s, ANSI_RESET)
}

/// Wrap text with ANSI color for warning (yellow), then reset.
pub fn wrap_warning(s: &str) -> String {
    format!("{}{}{}", ANSI_YELLOW, s, ANSI_RESET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_for_role_unknown_returns_default() {
        let theme = Theme::default();
        let r = ansi_for_role(&theme, "unknown_role_xyz");
        assert_eq!(r, ANSI_RESET);
    }

    #[test]
    fn parse_colors_csv_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("Colors.csv");
        let content = r#"token,value,usage
emerald-500,#10b981,primary accent success
red-500,#ef4444,error destructive
"#;
        std::fs::write(&path, content).unwrap();
        let colors = parse_colors_csv(&path);
        assert_eq!(colors.len(), 2);
        assert_eq!(
            colors.get("emerald-500").map(|c| c.value.as_str()),
            Some("#10b981")
        );
        assert_eq!(
            colors.get("emerald-500").map(|c| c.usage.as_str()),
            Some("primary accent success")
        );
        assert_eq!(
            colors.get("red-500").map(|c| c.value.as_str()),
            Some("#ef4444")
        );
    }

    #[test]
    fn parse_typography_csv_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("Typography.csv");
        let content = r#"role,font_stack,scale_rem,weight,usage
heading,Outfit Inter system-ui,1.5-2,600-700,page titles
body,Inter system-ui sans-serif,1,400,body text
"#;
        std::fs::write(&path, content).unwrap();
        let typo = parse_typography_csv(&path);
        assert_eq!(typo.len(), 2);
        assert_eq!(typo[0].role, "heading");
        assert_eq!(typo[0].font_stack, "Outfit Inter system-ui");
        assert_eq!(typo[1].role, "body");
        assert_eq!(typo[1].weight, "400");
    }

    #[test]
    fn load_theme_none_returns_default() {
        let theme = load_theme(None);
        assert!(theme.colors.is_empty());
        assert!(theme.typography.is_empty());
    }

    #[test]
    fn load_theme_missing_dir_returns_default() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("nonexistent");
        let theme = load_theme(Some(base.as_path()));
        assert!(theme.colors.is_empty());
        assert!(theme.typography.is_empty());
    }

    #[test]
    fn load_theme_missing_files_returns_empty_theme() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("docs").join("design").join("data")).unwrap();
        let theme = load_theme(Some(tmp.path()));
        assert!(theme.colors.is_empty());
        assert!(theme.typography.is_empty());
    }

    #[test]
    fn load_theme_parses_both_csvs() {
        let tmp = tempfile::tempdir().unwrap();
        let data_dir = tmp.path().join("docs").join("design").join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(
            data_dir.join("Colors.csv"),
            "token,value,usage\nslate-950,#0f172a,backgrounds dark\n",
        )
        .unwrap();
        std::fs::write(
            data_dir.join("Typography.csv"),
            "role,font_stack,scale_rem,weight,usage\nmono,ui-monospace,0.875,400,code\n",
        )
        .unwrap();
        let theme = load_theme(Some(tmp.path()));
        assert_eq!(theme.colors.len(), 1);
        assert_eq!(
            theme.colors.get("slate-950").map(|c| c.value.as_str()),
            Some("#0f172a")
        );
        assert_eq!(theme.typography.len(), 1);
        assert_eq!(theme.typography[0].role, "mono");
    }

    #[test]
    fn malformed_csv_handled_gracefully() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("Colors.csv");
        std::fs::write(&path, "not,a,valid,header\n").unwrap();
        let colors = parse_colors_csv(&path);
        assert!(colors.is_empty() || colors.len() == 1);
    }

    #[test]
    fn wrap_success_error_warning() {
        assert!(wrap_success("ok").contains("ok") && wrap_success("ok").contains(ANSI_RESET));
        assert!(wrap_error("err").contains("err") && wrap_error("err").contains(ANSI_RESET));
        assert!(wrap_warning("warn").contains("warn") && wrap_warning("warn").contains(ANSI_RESET));
    }
}
