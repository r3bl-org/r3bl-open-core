// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core client structure and lifecycle state machine for `rust-analyzer`.

use crate::{constants::lsp_methods,
            error::McpServerError,
            lsp::{protocol::{primitive_types::{JsonRpcRequestId, LspWriter},
                             readiness_types::ServerReadiness,
                             table_types::{DocumentUri, SafeDiagnosticsTable,
                                           SafePendingRequestsTable}},
                  readiness_monitor::ServerReadinessMonitor}};
use r3bl_tui::ok;
use serde_json::json;
use std::{fmt::{self, Debug},
          path::{Path, PathBuf},
          process::Child,
          sync::Arc};

/// Synchronous JSON-RPC translation client that communicates with a background
/// `rust-analyzer` child process.
#[derive(Default)]
pub struct RustAnalyzerClient {
    /// Canonical workspace root directory.
    pub workspace_root: PathBuf,

    /// Handle to the spawned `rust-analyzer` child process.
    pub rust_analyzer_child_proc: Option<Child>,

    /// Thread-safe registry mapping in-flight JSON-RPC request IDs to single-use reply
    /// channels.
    pub pending_requests: SafePendingRequestsTable,

    /// Monotonically increasing request ID counter.
    pub request_id: JsonRpcRequestId,

    /// Thread-safe writer handle to `rust-analyzer` `stdin`.
    pub lsp_writer: Option<LspWriter>,

    /// Current handshake state with the language server.
    pub handshake_status: HandshakeStatus,

    /// Set of document URIs currently opened via `textDocument/didOpen`.
    pub open_documents: Vec<DocumentUri>,

    /// Cache of compiler diagnostics received via `textDocument/publishDiagnostics`
    /// notifications, keyed by document URI.
    pub diagnostics: SafeDiagnosticsTable,

    /// Live server readiness and indexing status monitor.
    pub readiness_monitor: Arc<ServerReadinessMonitor>,
}

/// Lifecycle state of the Language Server Protocol (LSP) handshake with `rust-analyzer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HandshakeStatus {
    /// Handshake has not yet started or has completed shutdown.
    #[default]
    Uninitialized,
    /// Handshake has completed successfully and the client is ready for LSP queries.
    Initialized,
}

impl Debug for RustAnalyzerClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RustAnalyzerClient")
            .field("workspace_root", &self.workspace_root)
            .field("handshake_status", &self.handshake_status)
            .finish_non_exhaustive()
    }
}

/// Canonicalizes a workspace path with a fallback to resolving relative to the current
/// working directory. On Windows, verbatim UNC prefixes (`\\?\`) are stripped for
/// compatibility with tools and language servers.
#[must_use]
pub fn canonicalize_path(path: &Path) -> PathBuf {
    let canonical = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        }
    });

    #[cfg(windows)]
    {
        let s = canonical.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }

    canonical
}

/// Converts a path to a valid RFC 3986 `file://` URI string.
/// On Windows, backslashes are converted to forward slashes and prefixed with `file:///`.
#[must_use]
pub fn path_to_file_uri(path: &Path) -> String {
    let path_buf = canonicalize_path(path);
    let s = path_buf.to_string_lossy();
    #[cfg(windows)]
    {
        let normalized = s.replace('\\', "/");
        if normalized.starts_with('/') {
            format!("file://{normalized}")
        } else {
            format!("file:///{normalized}")
        }
    }
    #[cfg(not(windows))]
    {
        if s.starts_with('/') {
            format!("file://{s}")
        } else {
            format!("file:///{s}")
        }
    }
}

impl RustAnalyzerClient {
    /// Creates a new `RustAnalyzerClient` instance targeting the specified workspace
    /// root.
    #[must_use]
    pub fn new(workspace_root: PathBuf) -> Self {
        let mut client = Self::default();
        client.workspace_root = canonicalize_path(&workspace_root);
        client
    }

    /// Returns `true` if the background `rust-analyzer` process is currently running and
    /// initialized.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.handshake_status == HandshakeStatus::Initialized
            && self.rust_analyzer_child_proc.is_some()
    }

    /// Spawns the `rust-analyzer` subprocess, starts the background I/O reader threads,
    /// and performs the initial LSP initialization handshake.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError`] if the binary cannot be found, process spawning fails,
    /// or the initialization handshake times out.
    pub fn start(&mut self) -> Result<(), McpServerError> {
        if self.is_running() {
            return ok!();
        }

        let ra_path = Self::locate_rust_analyzer_binary()?;
        let mut child = self.spawn_rust_analyzer_child_process(&ra_path)?;

        let lsp_stdin = child.stdin.take().ok_or_else(|| {
            McpServerError::ProcessPipe("Failed to capture stdin".to_string())
        })?;
        let lsp_stdout = child.stdout.take().ok_or_else(|| {
            McpServerError::ProcessPipe("Failed to capture stdout".to_string())
        })?;
        let lsp_stderr = child.stderr.take().ok_or_else(|| {
            McpServerError::ProcessPipe("Failed to capture stderr".to_string())
        })?;

        self.lsp_writer = Some(Box::new(lsp_stdin));
        self.rust_analyzer_child_proc = Some(child);

        // Spawn background reader threads.
        let _stderr_handle = Self::spawn_stderr_reader_thread(lsp_stderr);
        let _stdout_handle = Self::spawn_stdout_reader_thread(
            lsp_stdout,
            Arc::clone(&self.pending_requests),
            Arc::clone(&self.diagnostics),
            Arc::clone(&self.readiness_monitor),
        );

        // Perform LSP handshake.
        self.initialize()?;

        ok!()
    }

    /// Sends the LSP `initialize` request and subsequent `initialized` notification to
    /// complete the protocol startup handshake.
    fn initialize(&mut self) -> Result<(), McpServerError> {
        let workspace_uri = path_to_file_uri(&self.workspace_root);
        let current_pid = std::process::id();

        let init_params = json!({
            "processId": current_pid,
            "rootUri": workspace_uri,
            "workspaceFolders": [
                {
                    "uri": workspace_uri,
                    "name": self.workspace_root.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                }
            ],
            "initializationOptions": {
                "checkOnSave": false,
                "diagnostics": {
                    "enable": true
                },
                "cargo": {
                    "loadOutDirsFromCheck": true
                },
                "procMacro": {
                    "enable": true
                }
            },
            "capabilities": {
                "experimental": {
                    "serverStatusNotification": true
                },
                "textDocument": {
                    "hover": {
                        "contentFormat": ["markdown", "plaintext"]
                    },
                    "completion": {
                        "completionItem": {
                            "snippetSupport": true
                        }
                    },
                    "definition": {
                        "linkSupport": true
                    },
                    "references": {},
                    "documentSymbol": {},
                    "codeAction": {
                        "codeActionLiteralSupport": {
                            "codeActionKind": {
                                "valueSet": [
                                    "quickfix",
                                    "refactor",
                                    "refactor.extract",
                                    "refactor.inline",
                                    "refactor.rewrite",
                                    "source",
                                    "source.organizeImports"
                                ]
                            }
                        },
                        "resolveSupport": {
                            "properties": ["edit"]
                        }
                    },
                    "publishDiagnostics": {
                        "relatedInformation": true,
                        "tagSupport": {
                            "valueSet": [1, 2]
                        }
                    },
                    "formatting": {},
                    "diagnostic": {
                        "dynamicRegistration": false,
                        "relatedDocumentSupport": false
                    }
                },
                "workspace": {
                    "didChangeConfiguration": {
                        "dynamicRegistration": false
                    },
                    "diagnostics": {
                        "refreshSupport": false
                    }
                }
            }
        });

        self.send_request(lsp_methods::INITIALIZE, Some(init_params))?;
        self.send_notification(lsp_methods::INITIALIZED, Some(json!({})))?;

        // Request workspace reload to trigger initial check.
        let _ignored = self.send_request("rust-analyzer/reloadWorkspace", None);

        self.handshake_status = HandshakeStatus::Initialized;
        ok!()
    }

    /// Waits until `rust-analyzer` finishes indexing (`is_indexed == true`) or until the
    /// specified timeout expires.
    ///
    /// Returns the latest [`ServerReadiness`] snapshot.
    #[must_use]
    pub fn wait_until_indexed(&self, timeout: std::time::Duration) -> ServerReadiness {
        self.readiness_monitor.wait_until_indexed(timeout)
    }

    /// Sets the server readiness state directly for unit testing.
    #[cfg(test)]
    pub fn set_readiness_for_test(&self, readiness: ServerReadiness) {
        self.readiness_monitor.update(readiness);
    }

    /// Shuts down the LSP session cleanly and terminates the background child process.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError`] if sending the shutdown request fails.
    pub fn shutdown(&mut self) -> Result<(), McpServerError> {
        if self.handshake_status == HandshakeStatus::Initialized {
            let _ignored_shutdown = self.send_request(lsp_methods::SHUTDOWN, None);
            let _ignored_exit = self.send_notification(lsp_methods::EXIT, None);
        }

        self.lsp_writer = None;

        if let Some(mut child_proc) = self.rust_analyzer_child_proc.take() {
            let _ignored_kill = child_proc.kill();
            let _ignored_wait = child_proc.wait();
        }

        self.open_documents.clear();
        if let Ok(mut diag_guard) = self.diagnostics.lock() {
            diag_guard.clear();
        }
        self.readiness_monitor.reset();
        self.handshake_status = HandshakeStatus::Uninitialized;
        ok!()
    }
}

impl Drop for RustAnalyzerClient {
    fn drop(&mut self) { let _ignored_shutdown = self.shutdown(); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::readiness_types::IndexingStatus;

    #[test]
    fn test_canonicalize_path_relative_and_absolute() {
        let current_dir = std::env::current_dir().unwrap();
        let canonical_current = canonicalize_path(Path::new("."));
        assert_eq!(canonical_current, canonicalize_path(&current_dir));

        let non_existent = std::env::temp_dir().join("non_existent_folder_abc123");
        let fallback = canonicalize_path(&non_existent);
        assert_eq!(fallback, non_existent);
    }

    #[test]
    fn test_client_new_initial_state() {
        let client = RustAnalyzerClient::new(PathBuf::from("."));
        assert_eq!(client.handshake_status, HandshakeStatus::Uninitialized);
        assert!(!client.is_running());
        assert!(client.open_documents.is_empty());
    }

    #[test]
    fn test_handshake_status_default() {
        assert_eq!(HandshakeStatus::default(), HandshakeStatus::Uninitialized);
    }

    #[test]
    fn test_wait_until_indexed_already_indexed() {
        let client = RustAnalyzerClient::new(PathBuf::from("."));
        client.set_readiness_for_test(ServerReadiness {
            status: IndexingStatus::Complete,
            health: "ok".to_string(),
            message: None,
        });

        let status = client.wait_until_indexed(std::time::Duration::from_millis(50));
        assert_eq!(status.status, IndexingStatus::Complete);
        assert_eq!(status.health, "ok");
    }

    #[test]
    fn test_wait_until_indexed_timeout_when_not_indexed() {
        let client = RustAnalyzerClient::new(PathBuf::from("."));
        // Remains status == IndexingStatus::InProgress
        let status = client.wait_until_indexed(std::time::Duration::from_millis(20));
        assert_eq!(status.status, IndexingStatus::InProgress);
        assert_eq!(status.health, "ok");
    }

    #[test]
    fn test_wait_until_indexed_signaled_by_background_thread() {
        let client = RustAnalyzerClient::new(PathBuf::from("."));
        let monitor = Arc::clone(&client.readiness_monitor);

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(15));
            monitor.update(ServerReadiness {
                status: IndexingStatus::Complete,
                health: "ok".to_string(),
                message: None,
            });
        });

        let status = client.wait_until_indexed(std::time::Duration::from_millis(500));
        assert_eq!(status.status, IndexingStatus::Complete);
    }
}
