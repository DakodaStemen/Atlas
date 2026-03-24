//! Slack Incoming Webhook: best-effort POST of short messages. Used after commit_to_memory and
//! log_training_row when env SLACK_WEBHOOK_URL_* is set. Only https://hooks.slack.com/ URLs allowed.

use anyhow::anyhow;
use std::sync::OnceLock;

const SLACK_WEBHOOK_PREFIX: &str = "https://hooks.slack.com/";
const SLACK_TEXT_MAX_LEN: usize = 4000;
const SLACK_TIMEOUT_SECS: u64 = 5;

static HTTP_CLIENT: OnceLock<reqwest::blocking::Client> = OnceLock::new();

fn blocking_client() -> reqwest::blocking::Client {
    HTTP_CLIENT
        .get_or_init(|| {
            reqwest::blocking::Client::builder()
                .use_rustls_tls()
                .user_agent("AgenticMonolith/1.0 (slack)")
                .timeout(std::time::Duration::from_secs(SLACK_TIMEOUT_SECS))
                .build()
                .expect("slack reqwest client")
        })
        .clone()
}

/// Returns true only for Slack Incoming Webhook URLs. Rejects any other host.
fn is_slack_webhook_url(url: &str) -> bool {
    let url = url.trim();
    url.starts_with(SLACK_WEBHOOK_PREFIX) && url.len() > SLACK_WEBHOOK_PREFIX.len()
}

/// POST a text message to a Slack Incoming Webhook. URL must be https://hooks.slack.com/...
/// Text is truncated to 4000 chars. Best-effort; call from spawn_blocking so MCP response is not blocked.
pub fn notify_slack(webhook_url: &str, text: &str) -> Result<(), anyhow::Error> {
    let url = webhook_url.trim();
    if !is_slack_webhook_url(url) {
        return Err(anyhow!(
            "Slack webhook URL must start with {} and not be empty",
            SLACK_WEBHOOK_PREFIX
        ));
    }
    if !url.starts_with("https://") {
        return Err(anyhow!("Slack webhook URL must use https://"));
    }
    let text = if text.len() > SLACK_TEXT_MAX_LEN {
        let cut = SLACK_TEXT_MAX_LEN.saturating_sub(3);
        let safe_cut = text
            .char_indices()
            .take_while(|(i, _)| *i < cut)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        format!("{}...", &text[..safe_cut])
    } else {
        text.to_string()
    };
    let body = serde_json::json!({ "text": text });
    let client = blocking_client();
    let res = client
        .post(url)
        .json(&body)
        .send()
        .map_err(|e| anyhow!("Slack webhook request failed: {}", e))?;
    if !res.status().is_success() {
        let status = res.status();
        let body_s = res.text().unwrap_or_else(|_| String::new());
        return Err(anyhow!(
            "Slack webhook returned {}: {}",
            status,
            body_s.trim()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{is_slack_webhook_url, notify_slack, SLACK_WEBHOOK_PREFIX};

    #[test]
    fn accepts_valid_slack_webhook_url() {
        assert!(is_slack_webhook_url(
            "https://hooks.slack.com/services/T00/B00/xxx"
        ));
        assert!(is_slack_webhook_url(
            "  https://hooks.slack.com/services/ABC/123/secret  "
        ));
    }

    #[test]
    fn rejects_non_slack_urls() {
        assert!(!is_slack_webhook_url("https://evil.com/webhook"));
        assert!(!is_slack_webhook_url(
            "http://hooks.slack.com/services/T/B/x"
        ));
        assert!(!is_slack_webhook_url("https://hooks.slack.com"));
        assert!(!is_slack_webhook_url(""));
    }

    #[test]
    fn notify_slack_rejects_invalid_url() {
        let err = notify_slack("https://example.com/foo", "hello").unwrap_err();
        assert!(err.to_string().contains(SLACK_WEBHOOK_PREFIX));
    }
}
