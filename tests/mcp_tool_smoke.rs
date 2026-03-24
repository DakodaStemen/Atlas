//! Call every tool listed in docs/tools_registry.json via a real stdio MCP session.
//! Run: `cargo test --test mcp_tool_smoke -- --test-threads=1`
//!
//! `route_task` is allowed to return a JSON-RPC error when no Google API key and no Ollama
//! are available; all other tools must return a normal `tools/call` result payload.
//!
//! After each successful call, [`assert_tool_output_quality`] checks a subset of tools for
//! non-empty, shape-like output (wiring + minimal behavioral signal), per the mcp_tool_quality tier.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const READ_TIMEOUT: Duration = Duration::from_secs(300);

fn wait_for_server_ready() {
    // Stdio-based MCP: polling isn't straightforward because the initialize
    // request IS the readiness check and the LineReceiver must be set up first.
    // A fixed delay before sending initialize is the simplest reliable approach.
    // 2500ms accommodates slow CI environments.
    thread::sleep(Duration::from_millis(2500));
}

fn send_request(stdin: &mut std::process::ChildStdin, body: &str) -> std::io::Result<()> {
    stdin.write_all(body.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()
}

struct LineReceiver {
    rx: mpsc::Receiver<std::io::Result<String>>,
}

impl LineReceiver {
    fn new(mut stdout: std::process::ChildStdout) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(&mut stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Ok(String::new()));
                        break;
                    }
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        break;
                    }
                }
            }
        });
        Self { rx }
    }

    fn read_response(&self) -> std::io::Result<String> {
        loop {
            match self.rx.recv_timeout(READ_TIMEOUT) {
                Ok(Ok(line)) => {
                    if line.is_empty() {
                        return Ok(String::new());
                    }
                    let s = line.trim_end_matches("\r\n").trim_end_matches('\n').trim();
                    if s.is_empty() {
                        continue;
                    }
                    if s.starts_with('{') {
                        return Ok(s.to_string());
                    }
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "read_response timed out",
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "stdout reader thread terminated",
                    ));
                }
            }
        }
    }
}

fn registry_tool_names() -> Vec<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/tools_registry.json");
    let raw = std::fs::read_to_string(&path).expect("read tools_registry.json");
    let v: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("parse tools_registry.json");
    v.into_iter()
        .filter_map(|o| o.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect()
}

fn prepare_workspace(root: &Path) -> String {
    std::fs::create_dir_all(root.join("src")).expect("mkdir src");
    std::fs::create_dir_all(root.join("docs")).expect("mkdir docs");
    std::fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "mcp_tool_smoke_ws"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Cargo.toml");
    std::fs::write(root.join("src/lib.rs"), "pub fn smoke_fn() {}\n").expect("lib.rs");
    std::fs::write(
        root.join("README.md"),
        "# Smoke title\n\nIntro\n\n## Section A\n\nBody a.\n",
    )
    .expect("README.md");
    std::fs::write(
        root.join("docs/TOOL_SELECTION_GUIDE.md"),
        "# Guide\n\n## dont\n\nBe careful.\n\n## core_vs_niche\n\nPick tools.\n",
    )
    .expect("TOOL_SELECTION_GUIDE.md");
    let _ = Command::new("git")
        .args(["init"])
        .current_dir(root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.email", "smoke@test.local"])
        .current_dir(root)
        .output();
    let _ = Command::new("git")
        .args(["config", "user.name", "smoke"])
        .current_dir(root)
        .output();
    let _ = Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let _ = Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(root)
        .output();

    root.canonicalize()
        .expect("canonicalize workspace")
        .to_string_lossy()
        .into_owned()
}

fn substitute_placeholders(v: &mut serde_json::Value, ws: &str, pad120: &str) {
    match v {
        serde_json::Value::String(s) => {
            if s == "__WS__" {
                *s = ws.to_string();
            } else if s == "__PAD120__" {
                *s = pad120.to_string();
            }
        }
        serde_json::Value::Array(a) => {
            for x in a {
                substitute_placeholders(x, ws, pad120);
            }
        }
        serde_json::Value::Object(m) => {
            for x in m.values_mut() {
                substitute_placeholders(x, ws, pad120);
            }
        }
        _ => {}
    }
}

fn tool_arguments(ws: &str) -> serde_json::Value {
    let pad120 = "x".repeat(120);
    let mut v: serde_json::Value = serde_json::from_str(include_str!("tool_smoke_args.json"))
        .expect("parse tool_smoke_args.json");
    substitute_placeholders(&mut v, ws, &pad120);
    v
}

fn tools_call_line(id: u64, name: &str, args: &serde_json::Value) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": { "name": name, "arguments": args }
    })
    .to_string()
}

fn assert_acceptable(name: &str, resp: &str) {
    if resp.contains("\"result\"") {
        return;
    }
    if name == "route_task" && resp.contains("\"error\"") {
        return;
    }
    panic!(
        "tool '{}' expected JSON-RPC result (or route_task error without LLM): {}",
        name, resp
    );
}

/// Text payload from a successful `tools/call` response (MCP `content[].text`).
fn extract_tool_text(resp: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(resp).ok()?;
    let result = v.get("result")?;
    let content = result.get("content")?.as_array()?;
    let mut parts = Vec::new();
    for block in content {
        if let Some(t) = block.get("text").and_then(|x| x.as_str()) {
            parts.push(t.to_string());
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn assert_tool_output_quality(name: &str, resp: &str) {
    if !resp.contains("\"result\"") {
        return;
    }
    if name == "route_task" {
        return;
    }
    let haystack = extract_tool_text(resp).unwrap_or_else(|| resp.to_string());
    if haystack.trim().is_empty() {
        panic!("tool '{}' returned empty text content: {}", name, resp);
    }
    match name {
        "get_system_status" => assert!(
            haystack.contains("CPU")
                || haystack.contains("cpu")
                || haystack.contains("Memory")
                || haystack.contains("memory"),
            "get_system_status: expected host/status-like text, got: {}",
            haystack
        ),
        "validate_tool_params" => assert!(
            haystack.contains("\"valid\"") && haystack.contains("\"warnings\""),
            "validate_tool_params: expected JSON with valid and warnings, got: {}",
            haystack
        ),
        "query_knowledge" => assert!(
            haystack.contains("No relevant information found")
                || haystack.contains('|')
                || haystack.contains("_meta")
                || haystack.contains("chunk"),
            "query_knowledge: expected empty-index message, outline hits, or meta; got: {}",
            haystack
        ),
        "execute_shell_command" => assert!(
            haystack.contains("Cargo.toml")
                || haystack.contains("README")
                || haystack.contains("lib.rs")
                || haystack.contains("src"),
            "execute_shell_command: expected `ls` listing workspace files, got: {}",
            haystack
        ),
        "scan_secrets" => assert!(
            haystack.contains("\"findings\"") && haystack.contains("\"ok\""),
            "scan_secrets: expected JSON with findings and ok, got: {}",
            haystack
        ),
        "get_metrics" => assert!(
            haystack.contains("Tool Calls") || haystack.contains("MCP Server Metrics"),
            "get_metrics: expected metrics report header, got: {}",
            haystack
        ),
        "get_loop_state" => assert!(
            haystack.contains("current_iteration") && haystack.contains("max_iterations"),
            "get_loop_state: expected loop state JSON keys, got: {}",
            haystack
        ),
        _ => {}
    }
}

#[test]
fn mcp_all_registry_tools_respond() {
    let bin = env!("CARGO_BIN_EXE_rag-mcp");
    let tmp_data = tempfile::tempdir().expect("data dir");
    let tmp_ws = tempfile::tempdir().expect("workspace");
    let ws = prepare_workspace(tmp_ws.path());
    let args_table = tool_arguments(&ws);

    let names = registry_tool_names();
    assert_eq!(
        names.len(),
        52,
        "tools_registry.json tool count changed; update this assertion and tool_smoke_args.json"
    );
    for n in &names {
        assert!(
            args_table.get(n).is_some(),
            "mcp_tool_smoke.rs tool_arguments() missing entry for registry tool '{}'",
            n
        );
    }

    let mut child = Command::new(bin)
        .arg("serve")
        .env("DATA_DIR", tmp_data.path())
        .env("ALLOWED_ROOTS", &ws)
        .env("MCP_FULL_TOOLS", "1")
        .env("MCP_LOOP_GUARD_DISABLED", "1")
        .env("MCP_MAX_LOOP_ITERATIONS", "100000")
        .env("MCP_TOOL_TIMEOUT_SECS", "300")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rag-mcp serve");

    wait_for_server_ready();

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let receiver = LineReceiver::new(stdout);

    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-tool-smoke","version":"0.1.0"}}}"#;
    send_request(&mut stdin, init_req).expect("write initialize");
    let init_resp = receiver.read_response().expect("read initialize");
    if !init_resp.contains("\"result\"") {
        let mut err_out = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err_out);
        panic!("initialize failed: {} stderr: {}", init_resp, err_out);
    }

    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send_request(&mut stdin, notif).expect("write initialized");

    let mut id: u64 = 2;
    for name in &names {
        let args = args_table
            .get(name)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let line = tools_call_line(id, name, &args);
        id += 1;
        send_request(&mut stdin, &line).expect("tools/call write");
        let resp = receiver.read_response().unwrap_or_else(|e| {
            let mut err = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut err);
            panic!("read {}: {} stderr: {}", name, e, err);
        });
        assert_acceptable(name, &resp);
        assert_tool_output_quality(name, &resp);
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn mcp_shell_rejects_disallowed_command() {
    let bin = env!("CARGO_BIN_EXE_rag-mcp");
    let tmp_data = tempfile::tempdir().expect("data dir");
    let tmp_ws = tempfile::tempdir().expect("workspace");
    let ws = prepare_workspace(tmp_ws.path());

    let mut child = Command::new(bin)
        .arg("serve")
        .env("DATA_DIR", tmp_data.path())
        .env("ALLOWED_ROOTS", &ws)
        .env("MCP_FULL_TOOLS", "1")
        .env("MCP_LOOP_GUARD_DISABLED", "1")
        .env("MCP_MAX_LOOP_ITERATIONS", "100000")
        .env("MCP_TOOL_TIMEOUT_SECS", "60")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rag-mcp serve");

    wait_for_server_ready();

    let mut stdin = child.stdin.take().expect("stdin");
    let stdout = child.stdout.take().expect("stdout");
    let mut stderr = child.stderr.take().expect("stderr");
    let receiver = LineReceiver::new(stdout);

    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-tool-smoke","version":"0.1.0"}}}"#;
    send_request(&mut stdin, init_req).expect("write initialize");
    let init_resp = receiver.read_response().expect("read initialize");
    if !init_resp.contains("\"result\"") {
        let mut err_out = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err_out);
        panic!("initialize failed: {} stderr: {}", init_resp, err_out);
    }
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send_request(&mut stdin, notif).expect("write initialized");

    let line = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"execute_shell_command","arguments":{"command":"python3 -c \"print(1)\""}}}"#;
    send_request(&mut stdin, line).expect("tools/call write");
    let resp = receiver.read_response().expect("read shell reject");
    assert!(
        resp.contains("\"result\""),
        "expected result for rejected shell: {}",
        resp
    );
    let text = extract_tool_text(&resp).expect("extract text");
    assert!(
        text.contains("allowlist") || text.contains("not on"),
        "expected allowlist rejection message, got: {}",
        text
    );

    let _ = child.kill();
    let _ = child.wait();
}
