//! MCP E2E: spawn rag-mcp serve, send JSON-RPC initialize + tools/call, assert response.
//! Run with: cargo test --test mcp_e2e -- --test-threads=1
//! Requires the binary to be built (cargo build or cargo test builds it first).
//! RMCP stdio transport uses Content-Length header + CRLF + body per message.
//!
//! Each test uses its own temporary DATA_DIR to avoid SQLite lock contention when
//! cargo runs tests in parallel.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Give the server process time to start and reach its read loop (avoids race on Windows/CI).
fn wait_for_server_ready() {
    thread::sleep(Duration::from_millis(1500));
}

/// Send one JSON-RPC message. rmcp uses newline-delimited JSON (one JSON object per line).
fn send_request(stdin: &mut std::process::ChildStdin, body: &str) -> std::io::Result<()> {
    stdin.write_all(body.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()
}

/// A line receiver backed by a dedicated reader thread. Each line from stdout is sent through
/// a channel so the main thread can use `recv_timeout` and never hang indefinitely.
struct LineReceiver {
    rx: mpsc::Receiver<std::io::Result<String>>,
}

impl LineReceiver {
    /// Spawn a background thread that reads lines from `stdout` and sends them on a channel.
    /// The thread exits when stdout is closed (i.e. when the child process is killed).
    fn new(mut stdout: std::process::ChildStdout) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(&mut stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => {
                        // EOF
                        let _ = tx.send(Ok(String::new()));
                        break;
                    }
                    Ok(_) => {
                        if tx.send(Ok(line)).is_err() {
                            break; // receiver dropped
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

    /// Read the next JSON response line, skipping blank/non-JSON lines.
    /// Times out after 10 seconds to prevent CI hangs.
    fn read_response(&self) -> std::io::Result<String> {
        let timeout = Duration::from_secs(10);
        loop {
            match self.rx.recv_timeout(timeout) {
                Ok(Ok(line)) => {
                    if line.is_empty() {
                        return Ok(String::new()); // EOF
                    }
                    let s = line.trim_end_matches("\r\n").trim_end_matches('\n').trim();
                    if s.is_empty() {
                        continue; // blank line, read next
                    }
                    if s.starts_with('{') {
                        return Ok(s.to_string());
                    }
                    // non-JSON line, skip
                }
                Ok(Err(e)) => return Err(e),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "read_response timed out after 10 seconds",
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

#[test]
fn mcp_e2e_initialize_then_tools_call_get_system_status() {
    let bin = env!("CARGO_BIN_EXE_rag-mcp");
    let tmp_dir = tempfile::tempdir().expect("create temp data dir");
    let mut child = Command::new(bin)
        .arg("serve")
        .env("DATA_DIR", tmp_dir.path())
        .env("ALLOWED_ROOTS", tmp_dir.path())
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

    // Initialize (MCP handshake)
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-e2e","version":"0.1.0"}}}"#;
    send_request(&mut stdin, init_req).expect("write initialize");
    let init_resp = receiver.read_response().expect("read initialize response");
    if !init_resp.contains("\"result\"") {
        let mut err_out = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err_out);
        panic!(
            "initialize should return result: {} (stderr: {})",
            init_resp, err_out
        );
    }
    assert!(
        !init_resp.contains("\"error\""),
        "initialize should not error: {}",
        init_resp
    );

    // Notify initialized (required after initialize)
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send_request(&mut stdin, notif).expect("write initialized");

    // tools/call get_system_status (no RAG/DB required)
    let tools_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"get_system_status","arguments":{}}}"#;
    send_request(&mut stdin, tools_req).expect("write tools/call");

    let tools_resp = receiver.read_response().expect("read tools/call response");
    assert!(
        tools_resp.contains("\"result\""),
        "tools/call should return result: {}",
        tools_resp
    );
    assert!(
        !tools_resp.contains("\"error\""),
        "tools/call should not error: {}",
        tools_resp
    );
    assert!(
        tools_resp.contains("CPU") || tools_resp.contains("cpu") || tools_resp.contains("content"),
        "get_system_status response should contain status-like content: {}",
        tools_resp
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// Loop guard: after MCP_LOOP_GUARD_THRESHOLD identical (tool, args) calls, the next returns error and buffer clears; the following call succeeds.
#[test]
fn mcp_e2e_loop_guard_blocks_after_threshold_then_allows() {
    let bin = env!("CARGO_BIN_EXE_rag-mcp");
    let tmp_dir = tempfile::tempdir().expect("create temp data dir");
    let mut child = Command::new(bin)
        .arg("serve")
        .env("DATA_DIR", tmp_dir.path())
        .env("ALLOWED_ROOTS", tmp_dir.path())
        .env("MCP_LOOP_GUARD_THRESHOLD", "3")
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

    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-e2e","version":"0.1.0"}}}"#;
    send_request(&mut stdin, init_req).expect("write initialize");
    let init_resp = receiver.read_response().expect("read initialize response");
    if !init_resp.contains("\"result\"") {
        let mut err_out = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err_out);
        panic!(
            "initialize should return result: {} (stderr: {})",
            init_resp, err_out
        );
    }
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send_request(&mut stdin, notif).expect("write initialized");

    let tools_call = r#"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"get_system_status","arguments":{}}}"#;

    // Call 1 and 2: succeed
    send_request(&mut stdin, tools_call).expect("write call 1");
    let r1 = receiver.read_response().expect("read call 1");
    assert!(r1.contains("\"result\""), "call 1 should succeed: {}", r1);
    send_request(&mut stdin, &tools_call.replace("\"id\":10", "\"id\":11")).expect("write call 2");
    let r2 = receiver.read_response().expect("read call 2");
    assert!(r2.contains("\"result\""), "call 2 should succeed: {}", r2);

    // Call 3: loop guard triggers (3 identical in a row)
    send_request(&mut stdin, &tools_call.replace("\"id\":10", "\"id\":12")).expect("write call 3");
    let r3 = receiver.read_response().expect("read call 3");
    assert!(
        r3.contains("\"error\"") && r3.contains("Loop guard"),
        "call 3 should return loop guard error: {}",
        r3
    );

    // Call 4: buffer was cleared, same tool+args should succeed again
    send_request(&mut stdin, &tools_call.replace("\"id\":10", "\"id\":13")).expect("write call 4");
    let r4 = receiver.read_response().expect("read call 4");
    assert!(
        r4.contains("\"result\""),
        "call 4 after clear should succeed: {}",
        r4
    );

    let _ = child.kill();
    let _ = child.wait();
}

/// Basic coverage: query_knowledge returns a result (not an error).
#[test]
fn mcp_e2e_query_knowledge_returns_result() {
    let bin = env!("CARGO_BIN_EXE_rag-mcp");
    let tmp_dir = tempfile::tempdir().expect("create temp data dir");
    let mut child = Command::new(bin)
        .arg("serve")
        .env("DATA_DIR", tmp_dir.path())
        .env("ALLOWED_ROOTS", tmp_dir.path())
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

    // Initialize
    let init_req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"mcp-e2e","version":"0.1.0"}}}"#;
    send_request(&mut stdin, init_req).expect("write initialize");
    let init_resp = receiver.read_response().expect("read initialize response");
    if !init_resp.contains("\"result\"") {
        let mut err_out = String::new();
        let _ = std::io::Read::read_to_string(&mut stderr, &mut err_out);
        panic!(
            "initialize should return result: {} (stderr: {})",
            init_resp, err_out
        );
    }

    // Notify initialized
    let notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    send_request(&mut stdin, notif).expect("write initialized");

    // tools/call query_knowledge
    let qk_req = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"query_knowledge","arguments":{"query":"test query"}}}"#;
    send_request(&mut stdin, qk_req).expect("write query_knowledge");

    let qk_resp = receiver
        .read_response()
        .expect("read query_knowledge response");
    assert!(
        qk_resp.contains("\"result\""),
        "query_knowledge should return result: {}",
        qk_resp
    );
    assert!(
        !qk_resp.contains("\"error\""),
        "query_knowledge should not error: {}",
        qk_resp
    );

    let _ = child.kill();
    let _ = child.wait();
}
