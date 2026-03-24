//! Helpers for synchronous and async process execution with a given working directory.
//! A default 60-second timeout is applied to both sync and async variants.

use anyhow::Result;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

/// Default timeout for spawned processes (60 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Poll interval for the sync timeout loop.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Runs a process synchronously in `cwd` with a 60-second timeout.
/// Returns an error if the program cannot be executed or if the timeout is exceeded.
pub fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<Output> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}': {}", program, e))?;

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = child.stdout.take().map_or_else(Vec::new, |mut s| {
                    let mut buf = Vec::new();
                    let _ = std::io::Read::read_to_end(&mut s, &mut buf);
                    buf
                });
                let stderr = child.stderr.take().map_or_else(Vec::new, |mut s| {
                    let mut buf = Vec::new();
                    let _ = std::io::Read::read_to_end(&mut s, &mut buf);
                    buf
                });
                return Ok(Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= DEFAULT_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(anyhow::anyhow!(
                        "Process '{}' timed out after {} seconds",
                        program,
                        DEFAULT_TIMEOUT.as_secs()
                    ));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to wait on '{}': {}",
                    program,
                    e
                ));
            }
        }
    }
}

/// Runs a process asynchronously in `cwd` via Tokio with a 60-second timeout.
/// Returns an error if the program cannot be executed or if the timeout is exceeded.
/// On timeout the child process is killed before returning.
pub async fn run_command_async(program: &str, args: &[String], cwd: &Path) -> Result<Output> {
    let mut child = tokio::process::Command::new(program)
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to execute '{}' (async): {}", program, e))?;

    let timeout_result = tokio::time::timeout(DEFAULT_TIMEOUT, async {
        let status = child.wait().await?;
        let stdout = match child.stdout.take() {
            Some(mut s) => {
                let mut buf = Vec::new();
                tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await?;
                buf
            }
            None => Vec::new(),
        };
        let stderr = match child.stderr.take() {
            Some(mut s) => {
                let mut buf = Vec::new();
                tokio::io::AsyncReadExt::read_to_end(&mut s, &mut buf).await?;
                buf
            }
            None => Vec::new(),
        };
        Ok::<Output, std::io::Error>(Output { status, stdout, stderr })
    })
    .await;

    match timeout_result {
        Ok(result) => result
            .map_err(|e| anyhow::anyhow!("Failed to wait on '{}' (async): {}", program, e)),
        Err(_elapsed) => {
            let _ = child.kill().await;
            Err(anyhow::anyhow!(
                "Process '{}' timed out after {} seconds (async)",
                program,
                DEFAULT_TIMEOUT.as_secs()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn run_command_missing_program_returns_err() {
        let cwd = Path::new(".");
        let r = run_command("this_program_does_not_exist_xyz", &[], cwd);
        assert!(r.is_err());
    }

    #[test]
    fn run_command_invalid_cwd_returns_err() {
        let cwd = Path::new("/nonexistent_cwd_xyz_123");
        let r = run_command("echo", &["hi"], cwd);
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn run_command_async_missing_program_returns_err() {
        let cwd = Path::new(".");
        let args: Vec<String> = vec![];
        let r = run_command_async("nonexistent_program_xyz_async", &args, cwd).await;
        assert!(r.is_err());
    }
}
