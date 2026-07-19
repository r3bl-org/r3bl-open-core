// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Subprocess management and background I/O reader thread orchestration for
//! `rust-analyzer`.

use crate::{constants::{debug_flags::DEBUG_LSP_CLIENT, lsp_framing},
            error::McpServerError,
            lsp::{client::RustAnalyzerClient,
                  protocol::table_types::{SafeDiagnosticsTable,
                                          SafePendingRequestsTable},
                  readiness_monitor::ServerReadinessMonitor}};
use std::{io::{BufRead, BufReader, Read},
          path::{Path, PathBuf},
          process::{Child, Command, Stdio},
          sync::Arc,
          thread::JoinHandle};

impl RustAnalyzerClient {
    /// Locates the `rust-analyzer` executable in the user's `PATH`.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::ProcessSpawn`] if `rust-analyzer` is not found.
    pub fn locate_rust_analyzer_binary() -> Result<PathBuf, McpServerError> {
        let which_output = Command::new("which")
            .arg(lsp_framing::RUST_ANALYZER_BINARY)
            .output();

        match which_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let trimmed = stdout.trim();
                if trimmed.is_empty() {
                    Err(McpServerError::ProcessSpawn(
                        "rust-analyzer binary not found in PATH".to_string(),
                    ))
                } else {
                    Ok(PathBuf::from(trimmed))
                }
            }
            _ => Err(McpServerError::ProcessSpawn(
                "rust-analyzer binary not found in PATH".to_string(),
            )),
        }
    }

    /// Spawns the `rust-analyzer` child process with piped `stdio` streams in the
    /// workspace directory.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::ProcessSpawn`] if spawning the child process fails.
    pub fn spawn_rust_analyzer_child_process(
        &self,
        ra_path: &Path,
    ) -> Result<Child, McpServerError> {
        let child = Command::new(ra_path)
            .current_dir(&self.workspace_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::error! {
                        message = "Failed to spawn rust-analyzer",
                        error = %e,
                        path = %ra_path.display(),
                        workspace = %self.workspace_root.display(),
                    };
                });
                McpServerError::ProcessSpawn(format!(
                    "Failed to spawn rust-analyzer at {}: {}",
                    ra_path.display(),
                    e
                ))
            })?;

        DEBUG_LSP_CLIENT.then(|| {
            tracing::info! {
                message = "Spawned rust-analyzer child process",
                pid = child.id(),
                workspace = %self.workspace_root.display(),
            };
        });

        Ok(child)
    }

    /// Spawns a background thread that drains `stderr` from `rust-analyzer`.
    ///
    /// Draining stderr continuously is critical to prevent operating system pipe buffer
    /// deadlocks (typically 64 KB limit on Linux).
    ///
    /// # Panics
    ///
    /// Panics if the OS fails to spawn the thread.
    #[must_use]
    pub fn spawn_stderr_reader_thread(
        stderr: impl Read + Send + 'static,
    ) -> JoinHandle<()> {
        std::thread::Builder::new()
            .name("lsp-stderr-reader".to_string())
            .spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(l) => {
                            DEBUG_LSP_CLIENT.then(|| {
                                tracing::debug! {
                                    message = "rust-analyzer stderr",
                                    line = %l,
                                };
                            });
                        }
                        Err(e) => {
                            DEBUG_LSP_CLIENT.then(|| {
                                tracing::error! {
                                    message = "Error reading rust-analyzer stderr",
                                    error = %e,
                                };
                            });
                            break;
                        }
                    }
                }
            })
            .expect("Failed to spawn lsp-stderr-reader thread")
    }

    /// Spawns a background thread that continuously reads and frames `Content-Length`
    /// JSON-RPC messages from `rust-analyzer`'s `stdout`.
    ///
    /// Dispatches response payloads to waiting request channels and caches published
    /// compiler diagnostics.
    ///
    /// # Panics
    ///
    /// Panics if the OS fails to spawn the thread.
    #[must_use]
    pub fn spawn_stdout_reader_thread(
        stdout: impl Read + Send + 'static,
        pending_requests: SafePendingRequestsTable,
        diagnostics: SafeDiagnosticsTable,
        readiness_monitor: Arc<ServerReadinessMonitor>,
    ) -> JoinHandle<()> {
        std::thread::Builder::new()
            .name("lsp-stdout-reader".to_string())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                let mut header_buf = String::new();

                loop {
                    header_buf.clear();

                    // Step 1: Read HTTP/LSP headers until the empty separator line.
                    let mut content_length: Option<usize> = None;
                    loop {
                        header_buf.clear();
                        match reader.read_line(&mut header_buf) {
                            Ok(0) => {
                                // EOF encountered on LSP stdout.
                                DEBUG_LSP_CLIENT.then(|| {
                                    tracing::info! {
                                        message = "EOF on rust-analyzer stdout; reader thread exiting",
                                    };
                                });
                                return;
                            }
                            Ok(_) => {
                                let trimmed = header_buf.trim();
                                if trimmed.is_empty() {
                                    // Empty line signifies end of headers.
                                    break;
                                }
                                if let Some(stripped) = trimmed.strip_prefix(lsp_framing::CONTENT_LENGTH_HEADER_PREFIX)
                                    && let Ok(len) = stripped.trim().parse::<usize>()
                                {
                                    content_length = Some(len);
                                }
                            }
                            Err(e) => {
                                DEBUG_LSP_CLIENT.then(|| {
                                    tracing::error! {
                                        message = "Error reading LSP header",
                                        error = %e,
                                    };
                                });
                                return;
                            }
                        }
                    }

                    // Step 2: Read exact payload byte count if Content-Length was provided.
                    let Some(length) = content_length else {
                        DEBUG_LSP_CLIENT.then(|| {
                            tracing::warn! {
                                message = "Missing Content-Length in LSP message headers; skipping",
                            };
                        });
                        continue;
                    };

                    let mut body_buf = vec![0u8; length];
                    if let Err(e) = reader.read_exact(&mut body_buf) {
                        DEBUG_LSP_CLIENT.then(|| {
                            tracing::error! {
                                message = "Error reading LSP message body bytes",
                                expected_bytes = length,
                                error = %e,
                            };
                        });
                        return;
                    }

                    // Step 3: Parse JSON-RPC payload and route to waiting channels or caches.
                    let body_str = String::from_utf8_lossy(&body_buf);
                    Self::process_incoming_json(
                        &body_str,
                        &pending_requests,
                        &diagnostics,
                        &readiness_monitor,
                    );
                }
            })
            .expect("Failed to spawn lsp-stdout-reader thread")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locate_rust_analyzer_binary_finds_binary() {
        let binary_path = RustAnalyzerClient::locate_rust_analyzer_binary();
        assert!(
            binary_path.is_ok(),
            "rust-analyzer should be found in PATH in developer environment"
        );
        let path = binary_path.unwrap();
        assert!(path.is_absolute());
    }
}
