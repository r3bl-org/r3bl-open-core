// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Model Context Protocol (MCP) server managing JSON-RPC over `stdio` and
//! dispatching tool requests.
//!
//! This module coordinates client process lifecycle, tool dispatch, document opening,
//! and graceful termination.
//!
//! See the [Threading Model] documentation for full details on all 3 threads.
//!
//! [Threading Model]: mod@crate#threading-model

use crate::{constants::{debug_flags::DEBUG_MCP_SERVER, lsp_framing, mcp_methods,
                        mcp_protocol, param_names, tool_names},
            error::McpServerError,
            lsp::{RustAnalyzerClient, canonicalize_path},
            mcp::{JsonRpcErrorPayload, McpRequest, McpResponse, ToolResult, get_tools,
                  handle_code_actions, handle_completion, handle_definition,
                  handle_diagnostics, handle_format, handle_hover, handle_references,
                  handle_symbols, handle_workspace_diagnostics},
            value_ext::ValueExt};
use r3bl_tui::ok;
use serde_json::{Value, json};
use std::{io::{BufRead, Write},
          path::PathBuf};

/// Main Model Context Protocol server executing synchronously over `stdin` and `stdout`.
///
/// See [Threading Model] for the complete 3-thread architecture.
///
/// [Threading Model]: mod@crate#threading-model
#[derive(Debug)]
pub struct RustAnalyzerMCPServer {
    /// Active language server client process handle.
    pub client: Option<RustAnalyzerClient>,

    /// Canonical workspace root directory.
    pub workspace_root: PathBuf,
}

impl RustAnalyzerMCPServer {
    /// One-shot helper to create and enter the main server event loop immediately.
    ///
    /// # Errors
    ///
    /// Returns an error if `stdin` reading or `stdout` writing fails.
    pub fn start(workspace_root: PathBuf) -> Result<(), McpServerError> {
        Self::new(workspace_root).enter_main_event_loop()
    }

    /// Creates a new MCP server rooted in the specified workspace directory path.
    #[must_use]
    pub fn new(workspace_root: PathBuf) -> Self {
        let workspace_root = canonicalize_path(&workspace_root);

        Self {
            client: None,
            workspace_root,
        }
    }

    /// Ensures the underlying `rust-analyzer` child process has been started.
    ///
    /// # Errors
    ///
    /// Returns an error if starting the `rust-analyzer` client process fails.
    pub fn ensure_client_started(&mut self) -> Result<(), McpServerError> {
        if self.client.is_none() {
            let mut client = RustAnalyzerClient::new(self.workspace_root.clone());
            client.start()?;
            self.client = Some(client);
        }
        ok!()
    }

    /// Reads a file from disk and synchronizes it with the LSP server via
    /// `didOpen`/`didSave`.
    ///
    /// # Errors
    ///
    /// Returns an error if file reading or LSP communication fails.
    pub fn open_document_if_needed(
        &mut self,
        file_path: &str,
    ) -> Result<String, McpServerError> {
        let absolute_path = self.workspace_root.join(file_path);
        let absolute_path = absolute_path
            .canonicalize()
            .unwrap_or_else(|_| absolute_path.clone());
        let uri = format!(
            "{}{}",
            lsp_framing::FILE_URI_PREFIX,
            absolute_path.display()
        );
        let content = std::fs::read_to_string(&absolute_path)?;

        if let Some(client) = &mut self.client {
            client.open_document(&uri, &content)?;
        }

        Ok(uri)
    }

    /// Dispatches an MCP tool invocation to the appropriate handler.
    ///
    /// # Errors
    ///
    /// Returns an error if the tool name is unknown or if executing the tool handler
    /// fails.
    #[allow(clippy::too_many_lines)]
    pub fn handle_tool_call(
        &mut self,
        tool_name: &str,
        args: Value,
    ) -> Result<ToolResult, McpServerError> {
        self.ensure_client_started()?;

        match tool_name {
            tool_names::HOVER => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_hover(client, &uri, &args)
            }
            tool_names::DEFINITION => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_definition(client, &uri, &args)
            }
            tool_names::REFERENCES => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_references(client, &uri, &args)
            }
            tool_names::COMPLETION => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_completion(client, &uri, &args)
            }
            tool_names::SYMBOLS => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_symbols(client, &uri, file_path)
            }
            tool_names::FORMAT => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_format(client, &uri)
            }
            tool_names::CODE_ACTIONS => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_code_actions(client, &uri, &args)
            }
            tool_names::SET_WORKSPACE => {
                let workspace_path = args.get_str_param(param_names::WORKSPACE_PATH)?;

                if let Some(client) = &mut self.client {
                    client.shutdown()?;
                }
                self.client = None;

                self.workspace_root =
                    canonicalize_path(std::path::Path::new(workspace_path));
                self.ensure_client_started()?;

                Ok(ToolResult::text(format!(
                    "Workspace set to: {}",
                    self.workspace_root.display()
                )))
            }
            tool_names::DIAGNOSTICS => {
                let file_path = args.get_str_param(param_names::FILE_PATH)?;
                let uri = self.open_document_if_needed(file_path)?;
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_diagnostics(client, &uri, file_path)
            }
            tool_names::WORKSPACE_DIAGNOSTICS => {
                let workspace_root = self.workspace_root.clone();
                let client = self
                    .client
                    .as_mut()
                    .ok_or(McpServerError::ClientNotAvailable)?;
                handle_workspace_diagnostics(client, workspace_root.as_path())
            }
            _ => Err(McpServerError::UnknownTool(tool_name.to_string())),
        }
    }

    /// Handles a single incoming [`McpRequest`] and produces a JSON-RPC response, or
    /// `None` for notifications.
    #[must_use]
    pub fn handle_request(&mut self, request: McpRequest) -> Option<McpResponse> {
        // Handle MCP notifications (no id or notification method).
        if request.id.is_none()
            || request.method.starts_with("notifications/")
            || request.method == mcp_methods::NOTIFICATIONS_INITIALIZED
        {
            DEBUG_MCP_SERVER.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "RustAnalyzerMCPServer::silently_handled_notification",
                    method = %request.method,
                };
            });
            return None;
        }

        let response = match request.method.as_str() {
            mcp_methods::INITIALIZE => McpResponse::success(
                request.id,
                json!({
                    "protocolVersion": mcp_protocol::VERSION,
                    "serverInfo": {
                        "name": mcp_protocol::SERVER_NAME,
                        "version": mcp_protocol::SERVER_VERSION
                    },
                    "capabilities": {
                        "tools": {}
                    }
                }),
            ),
            mcp_methods::TOOLS_LIST => McpResponse::success(
                request.id,
                json!({
                    "tools": get_tools()
                }),
            ),
            mcp_methods::TOOLS_CALL => {
                if let Some(params) = request.params {
                    let tool_name = params
                        .get(param_names::NAME)
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let args = params
                        .get(param_names::ARGUMENTS)
                        .cloned()
                        .unwrap_or_else(|| json!({}));

                    match self.handle_tool_call(tool_name, args) {
                        Ok(result) => match serde_json::to_value(result) {
                            Ok(serialized_result) => {
                                McpResponse::success(request.id, serialized_result)
                            }
                            Err(e) => {
                                let error_payload =
                                    JsonRpcErrorPayload::custom(e.to_string());
                                McpResponse::error(request.id, error_payload)
                            }
                        },
                        Err(e) => {
                            DEBUG_MCP_SERVER.then(|| {
                                // % is Display, ? is Debug.
                                tracing::error! {
                                    message = "RustAnalyzerMCPServer::tool_call_error",
                                    error = %e,
                                };
                            });
                            let error_payload = JsonRpcErrorPayload::from(&e);
                            McpResponse::error(request.id, error_payload)
                        }
                    }
                } else {
                    McpResponse::error(
                        request.id,
                        JsonRpcErrorPayload::invalid_params("Missing params object"),
                    )
                }
            }
            _ => McpResponse::error(
                request.id,
                JsonRpcErrorPayload::method_not_found(&request.method),
            ),
        };

        Some(response)
    }

    /// Runs the main server event loop reading newline-delimited JSON-RPC from `stdin`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading `stdin` or writing `stdout` fails.
    pub fn enter_main_event_loop(&mut self) -> Result<(), McpServerError> {
        DEBUG_MCP_SERVER.then(|| {
            // % is Display, ? is Debug.
            tracing::info! {
                message = "RustAnalyzerMCPServer::enter_main_event_loop",
                status = "starting",
            };
        });

        // Eagerly start the rust-analyzer subprocess in the background so indexing begins
        // immediately while the MCP host negotiates the initial handshake.
        let _ignored_eager_start = self.ensure_client_started();

        let stdin = std::io::stdin();
        let mut stdout = std::io::stdout().lock();

        // Read newline-delimited JSON-RPC requests from stdin and dispatch them to the
        // appropriate tool handler.
        for line in stdin.lock().lines() {
            let line = line?;
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                continue;
            }

            let request_payload = serde_json::from_str::<McpRequest>(trimmed_line);
            let Ok(request) = request_payload else {
                // The request is malformed JSON. Log the parse error and continue to the
                // next line.
                DEBUG_MCP_SERVER.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug! {
                        message = "RustAnalyzerMCPServer::failed_to_parse_request",
                        raw_line = %trimmed_line,
                    };
                });
                continue;
            };

            DEBUG_MCP_SERVER.then(|| {
                // % is Display, ? is Debug.
                tracing::debug! {
                    message = "RustAnalyzerMCPServer::received_request",
                    method = %request.method,
                };
            });

            // Handle the request and produce a response (if not a notification).
            let response_payload = self.handle_request(request);
            if let Some(response) = response_payload {
                // Write the response to stdout as a single line of JSON.
                serde_json::to_writer(&mut stdout, &response)?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
            }
        }

        DEBUG_MCP_SERVER.then(|| {
            // % is Display, ? is Debug.
            tracing::info! {
                message = "RustAnalyzerMCPServer::shutdown",
            };
        });

        // Gracefully terminate the child process to prevent orphan/zombie processes.
        if let Some(client) = &mut self.client {
            let _ignored_shutdown = client.shutdown();
        }

        ok!()
    }
}
