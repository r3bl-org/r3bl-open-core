// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! JSON-RPC message framing, synchronous request-response routing, and notification
//! dispatch.

use crate::{constants::{debug_flags::DEBUG_LSP_CLIENT, json_rpc, lsp_framing, timing},
            error::McpServerError,
            lsp::{client::RustAnalyzerClient,
                  protocol::{envelope_types::IncomingLspMessage,
                             primitive_types::JsonRpcRequestId,
                             table_types::{SafeDiagnosticsTable,
                                           SafePendingRequestsTable}},
                  readiness_monitor::ServerReadinessMonitor}};
use r3bl_tui::ok;
use serde_json::{Value, json};
use std::{io::Write,
          sync::{Arc, mpsc::sync_channel},
          time::Duration};

impl RustAnalyzerClient {
    /// Dispatches an incoming LSP JSON string to waiting request channels or updates
    /// diagnostic caches and server readiness.
    pub fn process_incoming_json(
        json_str: &str,
        pending_requests: &SafePendingRequestsTable,
        diagnostics: &SafeDiagnosticsTable,
        readiness_monitor: &Arc<ServerReadinessMonitor>,
    ) {
        let Ok(parsed_val) = serde_json::from_str::<Value>(json_str) else {
            DEBUG_LSP_CLIENT.then(|| {
                tracing::warn! {
                    message = "Failed to parse incoming LSP string as JSON-RPC",
                    raw = %json_str,
                };
            });
            return;
        };

        let Some(parsed_msg) = IncomingLspMessage::parse(&parsed_val) else {
            DEBUG_LSP_CLIENT.then(|| {
                tracing::warn! {
                    message = "IncomingLspMessage::parse returned None (invalid structure)",
                    raw = ?parsed_val,
                };
            });
            return;
        };

        match parsed_msg {
            IncomingLspMessage::ResponseSuccess { id, result } => {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::debug! {
                        message = "IncomingLspMessage::ResponseSuccess",
                        id = id.0,
                    };
                });
                Self::deliver_response(pending_requests, id, result);
            }
            IncomingLspMessage::ResponseError { id, error } => {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::warn! {
                        message = "IncomingLspMessage::ResponseError",
                        id = id.0,
                        error = ?error,
                    };
                });
                Self::deliver_response(pending_requests, id, json!(null));
            }
            IncomingLspMessage::PublishDiagnostics {
                uri,
                diagnostics: diags,
            } => {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::info! {
                        message = "IncomingLspMessage::PublishDiagnostics",
                        uri = %uri,
                        diagnostics_count = diags.len(),
                    };
                });
                if let Ok(mut diag_guard) = diagnostics.lock() {
                    diag_guard.insert(uri, diags);
                }
            }
            IncomingLspMessage::ServerStatus(status) => {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::info! {
                        message = "IncomingLspMessage::ServerStatus",
                        indexing_status = ?status.status,
                        health = %status.health,
                        status_message = ?status.message,
                    };
                });
                readiness_monitor.update(status);
            }
            IncomingLspMessage::OtherNotification { method } => {
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::trace! {
                        message = "Unhandled incoming LSP notification",
                        method = %method,
                    };
                });
            }
        }
    }

    /// Delivers an LSP response payload to the single-use channel registered for the
    /// specified request ID.
    pub fn deliver_response(
        pending_requests: &SafePendingRequestsTable,
        id: JsonRpcRequestId,
        payload: Value,
    ) {
        let sender_opt = {
            let mut guard = match pending_requests.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            guard.remove(&id)
        };

        if let Some(sender) = sender_opt {
            let _ignored = sender.send(payload);
        } else {
            DEBUG_LSP_CLIENT.then(|| {
                tracing::warn! {
                    message = "Received response for unknown or timed-out request ID",
                    id = id.0,
                };
            });
        }
    }

    /// Sends a one-way JSON-RPC notification to `rust-analyzer`'s `stdin`.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::ClientNotAvailable`] if the client is not running, or
    /// [`McpServerError::Io`] / [`McpServerError::Json`] if serialization or writing
    /// fails.
    pub fn send_notification(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<(), McpServerError> {
        let stdin_writer = self
            .lsp_writer
            .as_mut()
            .ok_or(McpServerError::ClientNotAvailable)?;

        let notification_json = if let Some(params_val) = params {
            json!({
                "jsonrpc": json_rpc::VERSION,
                "method": method,
                "params": params_val
            })
        } else {
            json!({
                "jsonrpc": json_rpc::VERSION,
                "method": method,
            })
        };

        let body_bytes = serde_json::to_vec(&notification_json)?;

        let header = format!(
            "{}{}\r\n\r\n",
            lsp_framing::CONTENT_LENGTH_HEADER_PREFIX,
            body_bytes.len()
        );

        DEBUG_LSP_CLIENT.then(|| {
            tracing::trace! {
                message = "Sending LSP notification",
                method = %method,
            };
        });

        stdin_writer.write_all(header.as_bytes())?;
        stdin_writer.write_all(&body_bytes)?;
        stdin_writer.flush()?;

        ok!()
    }

    /// Sends a synchronous JSON-RPC request to `rust-analyzer` and blocks until the
    /// response is received or the default timeout expires.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError::ClientNotAvailable`] if not connected,
    /// [`McpServerError::RequestCancelled`] on timeout, or
    /// [`McpServerError::Io`] / [`McpServerError::Json`] on failure.
    pub fn send_request(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, McpServerError> {
        let request_id = self.request_id.next();
        let (tx, rx) = sync_channel(1);

        {
            let mut guard = self.pending_requests.lock().map_err(|_| {
                McpServerError::Custom("Pending requests lock poisoned".to_string())
            })?;
            guard.insert(request_id, tx);
        }

        let request_json = if let Some(params_val) = params {
            json!({
                "jsonrpc": json_rpc::VERSION,
                "id": request_id,
                "method": method,
                "params": params_val
            })
        } else {
            json!({
                "jsonrpc": json_rpc::VERSION,
                "id": request_id,
                "method": method,
            })
        };

        let body_bytes = serde_json::to_vec(&request_json)?;

        let header = format!(
            "{}{}\r\n\r\n",
            lsp_framing::CONTENT_LENGTH_HEADER_PREFIX,
            body_bytes.len()
        );

        DEBUG_LSP_CLIENT.then(|| {
            tracing::trace! {
                message = "Sending LSP request",
                id = request_id.0,
                method = %method,
            };
        });

        let stdin_writer = self
            .lsp_writer
            .as_mut()
            .ok_or(McpServerError::ClientNotAvailable)?;

        stdin_writer.write_all(header.as_bytes())?;
        stdin_writer.write_all(&body_bytes)?;
        stdin_writer.flush()?;

        // Wait for response payload on single-use receiver with timeout.
        let timeout = Duration::from_secs(timing::LSP_REQUEST_TIMEOUT_SECS);
        match rx.recv_timeout(timeout) {
            Ok(response_value) => Ok(response_value),
            Err(_) => {
                // Remove timed-out channel from pending requests.
                if let Ok(mut guard) = self.pending_requests.lock() {
                    guard.remove(&request_id);
                }
                DEBUG_LSP_CLIENT.then(|| {
                    tracing::warn! {
                        message = "LSP request timed out",
                        id = request_id.0,
                        method = %method,
                        timeout_secs = timing::LSP_REQUEST_TIMEOUT_SECS,
                    };
                });
                Err(McpServerError::RequestCancelled)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp::readiness_types::IndexingStatus;
    use std::{collections::HashMap, sync::Mutex};

    #[test]
    fn test_process_incoming_json_routes_response_success() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diags = Arc::new(Mutex::new(HashMap::new()));
        let readiness_monitor = Arc::new(ServerReadinessMonitor::default());
        let (tx, rx) = sync_channel(1);
        pending.lock().unwrap().insert(JsonRpcRequestId(42), tx);

        let json_payload =
            r#"{"jsonrpc":"2.0","id":42,"result":{"contents":"hover info"}}"#;
        RustAnalyzerClient::process_incoming_json(
            json_payload,
            &pending,
            &diags,
            &readiness_monitor,
        );

        assert!(pending.lock().unwrap().is_empty());
        let received = rx.recv().unwrap();
        assert_eq!(received, json!({"contents":"hover info"}));
    }

    #[test]
    fn test_process_incoming_json_routes_response_error() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diags = Arc::new(Mutex::new(HashMap::new()));
        let readiness_monitor = Arc::new(ServerReadinessMonitor::default());
        let (tx, rx) = sync_channel(1);
        pending.lock().unwrap().insert(JsonRpcRequestId(99), tx);

        let json_payload = r#"{"jsonrpc":"2.0","id":99,"error":{"code":-32601,"message":"Method not found"}}"#;
        RustAnalyzerClient::process_incoming_json(
            json_payload,
            &pending,
            &diags,
            &readiness_monitor,
        );

        assert!(pending.lock().unwrap().is_empty());
        let received = rx.recv().unwrap();
        assert_eq!(received, Value::Null);
    }

    #[test]
    fn test_process_incoming_json_caches_publish_diagnostics() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diags = Arc::new(Mutex::new(HashMap::new()));
        let readiness_monitor = Arc::new(ServerReadinessMonitor::default());

        let json_payload = r#"{
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///src/lib.rs",
                "diagnostics": [
                    { "message": "type error", "severity": 1 }
                ]
            }
        }"#;
        RustAnalyzerClient::process_incoming_json(
            json_payload,
            &pending,
            &diags,
            &readiness_monitor,
        );

        let guard = diags.lock().unwrap();
        assert_eq!(guard.len(), 1);
        let lib_diags = guard.get("file:///src/lib.rs").unwrap();
        assert_eq!(lib_diags.len(), 1);
        assert_eq!(lib_diags[0]["message"], "type error");
    }

    #[test]
    fn test_process_incoming_json_updates_server_status_and_signals_condvar() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diags = Arc::new(Mutex::new(HashMap::new()));
        let readiness_monitor = Arc::new(ServerReadinessMonitor::default());

        let json_payload = r#"{
            "jsonrpc": "2.0",
            "method": "experimental/serverStatus",
            "params": {
                "quiescent": true,
                "health": "ok",
                "message": null
            }
        }"#;
        RustAnalyzerClient::process_incoming_json(
            json_payload,
            &pending,
            &diags,
            &readiness_monitor,
        );

        let snapshot = readiness_monitor.get_snapshot();
        assert_eq!(snapshot.status, IndexingStatus::Complete);
        assert_eq!(snapshot.health, "ok");
        assert_eq!(snapshot.message, None);
    }

    #[test]
    fn test_process_incoming_json_handles_invalid_json() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let diags = Arc::new(Mutex::new(HashMap::new()));
        let readiness_monitor = Arc::new(ServerReadinessMonitor::default());

        // Should not panic on malformed JSON
        RustAnalyzerClient::process_incoming_json(
            "not valid json",
            &pending,
            &diags,
            &readiness_monitor,
        );
        RustAnalyzerClient::process_incoming_json(
            r#"{"unexpected": "format"}"#,
            &pending,
            &diags,
            &readiness_monitor,
        );
        assert!(pending.lock().unwrap().is_empty());
        assert!(diags.lock().unwrap().is_empty());
    }

    #[test]
    fn test_deliver_response_unknown_id_does_not_panic() {
        let pending = Arc::new(Mutex::new(HashMap::new()));
        RustAnalyzerClient::deliver_response(
            &pending,
            JsonRpcRequestId(9999),
            json!({"test": 1}),
        );
        assert!(pending.lock().unwrap().is_empty());
    }
}
