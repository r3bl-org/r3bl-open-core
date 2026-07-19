// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//cspell:words uinteger

//! Language Server Protocol (LSP) message types, JSON-RPC wire envelopes, and server
//! status.

use crate::constants::{json_rpc, lsp_methods};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap,
          io::Write,
          sync::{Arc, Mutex, mpsc::SyncSender}};

// LSP Specification Primitives & Request ID
pub mod primitive_types {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Zero-based line index in a text document, mandated by the [LSP Position
    /// Specification] as a 32-bit unsigned integer (`uinteger`).
    ///
    /// [LSP Position Specification]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
    pub type LspLineNumber = u32;

    /// Zero-based character offset within a line in a text document, mandated by the
    /// [LSP Position Specification] as a 32-bit unsigned integer (`uinteger`).
    ///
    /// [LSP Position Specification]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
    pub type LspCharPosition = u32;

    /// Payload byte length parsed from the header part defined by the [LSP Base Protocol
    /// Specification].
    ///
    /// [LSP Base Protocol Specification]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#headerPart
    pub type LspPayloadByteLength = usize;

    /// Thread-safe `stdin` writer handle to the `rust-analyzer` child process.
    pub type LspWriter = Box<dyn Write + Send>;

    /// Numeric request identifier for request/response correlation, defined by the
    /// [JSON-RPC 2.0 Request Specification].
    ///
    /// [JSON-RPC 2.0 Request Specification]: https://www.jsonrpc.org/specification#request_object
    #[derive(
        Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
    )]
    #[serde(transparent)]
    pub struct JsonRpcRequestId(pub u64);

    mod impl_json_rpc_request_id {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl Default for JsonRpcRequestId {
            /// Initial request ID for monotonically increasing JSON-RPC requests (`1`).
            fn default() -> Self { Self(json_rpc::INITIAL_REQUEST_ID) }
        }

        impl JsonRpcRequestId {
            /// Post-increments the request ID, returning the current value and advancing
            /// internal state.
            #[allow(clippy::should_implement_trait)]
            #[must_use]
            pub fn next(&mut self) -> Self {
                let current = *self;
                self.0 += 1;
                current
            }
        }
    }
}

// Thread-Safe Registry Tables & State
pub mod table_types {
    use super::primitive_types::JsonRpcRequestId;
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Owned canonical document URI string (e.g. `"file:///path/to/main.rs"`).
    pub type DocumentUri = String;

    /// Borrowed canonical document URI string slice.
    pub type DocumentUriRef<'a> = &'a str;

    /// List of compiler diagnostic JSON objects for a single document.
    pub type DiagnosticList = Vec<Value>;

    /// In-memory table caching compiler diagnostics keyed by document URI.
    pub type DiagnosticsTable = HashMap<DocumentUri, DiagnosticList>;

    /// Thread-safe reference-counted [`DiagnosticsTable`].
    pub type SafeDiagnosticsTable = Arc<Mutex<DiagnosticsTable>>;

    /// Single-use transmitter channel that delivers an LSP response to a waiting caller.
    pub type ResponseSender = SyncSender<Value>;

    /// Registry table of in-flight JSON-RPC requests awaiting replies from rust-analyzer.
    pub type PendingRequestsTable = HashMap<JsonRpcRequestId, ResponseSender>;

    /// Thread-safe reference-counted [`PendingRequestsTable`].
    pub type SafePendingRequestsTable = Arc<Mutex<PendingRequestsTable>>;
}

// Server Readiness & Indexing Status
pub mod readiness_types {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Status indicating whether rust-analyzer has completed workspace indexing or is
    /// actively scanning / loading crates.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
    #[serde(from = "bool", into = "bool")]
    pub enum IndexingStatus {
        /// Background crate indexing, cargo metadata fetching, or proc-macro expansion is
        /// actively running.
        #[default]
        InProgress,
        /// Workspace indexing is complete and the AST database is ready for queries.
        Complete,
    }

    mod impl_indexing_status {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl From<bool> for IndexingStatus {
            fn from(quiescent: bool) -> Self {
                if quiescent {
                    Self::Complete
                } else {
                    Self::InProgress
                }
            }
        }

        impl From<IndexingStatus> for bool {
            fn from(status: IndexingStatus) -> Self {
                match status {
                    IndexingStatus::Complete => true,
                    IndexingStatus::InProgress => false,
                }
            }
        }
    }

    /// Live readiness and indexing status from rust-analyzer `experimental/serverStatus`.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ServerReadiness {
        /// Background workspace indexing status.
        #[serde(rename = "quiescent", alias = "status")]
        pub status: IndexingStatus,

        /// Overall health status reported by rust-analyzer (e.g. `"ok"`, `"warning"`,
        /// `"error"`).
        pub health: String,

        /// Human-readable background progress description (e.g. `"Fetching crates..."`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub message: Option<String>,
    }

    mod impl_server_readiness {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl Default for ServerReadiness {
            fn default() -> Self {
                Self {
                    status: IndexingStatus::InProgress,
                    health: "ok".to_string(),
                    message: None,
                }
            }
        }
    }
}

// JSON-RPC Protocol Envelopes & Dispatching
pub mod envelope_types {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use super::{primitive_types::JsonRpcRequestId, readiness_types::ServerReadiness};

    /// Outgoing request to the rust-analyzer Language Server.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LspRequest {
        /// JSON-RPC version string (always `"2.0"`).
        pub jsonrpc: String,
        /// Numeric request identifier for response correlation.
        pub id: JsonRpcRequestId,
        /// LSP method name (e.g. `"textDocument/hover"`).
        pub method: String,
        /// Parameters payload.
        pub params: Option<Value>,
    }

    mod impl_lsp_request {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl LspRequest {
            /// Creates a new outgoing LSP request with specification-mandated JSON-RPC
            /// version.
            #[must_use]
            pub fn new(
                id: JsonRpcRequestId,
                method: impl Into<String>,
                params: Option<Value>,
            ) -> Self {
                Self {
                    jsonrpc: json_rpc::VERSION.to_string(),
                    id,
                    method: method.into(),
                    params,
                }
            }
        }
    }

    /// Incoming response from the rust-analyzer Language Server.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LspResponse {
        /// JSON-RPC version string.
        pub jsonrpc: String,
        /// Request identifier matching the sent request.
        pub id: Option<JsonRpcRequestId>,
        /// Result payload if successful.
        pub result: Option<Value>,
        /// Error payload if unsuccessful.
        pub error: Option<Value>,
    }

    /// Categorized incoming Language Server Protocol (LSP) message parsed from child
    /// process `stdout`.
    #[derive(Debug, Clone, PartialEq)]
    pub enum IncomingLspMessage {
        /// Server notification publishing compiler diagnostics for a file URI.
        PublishDiagnostics {
            /// File URI for which diagnostics were emitted.
            uri: String,
            /// Array of compiler diagnostic JSON objects.
            diagnostics: Vec<Value>,
        },
        /// Server status notification indicating indexing progress or completion.
        ServerStatus(ServerReadiness),
        /// Any other unhandled server notification (e.g. `window/logMessage`).
        OtherNotification {
            /// LSP notification method name.
            method: String,
        },
        /// Correlated request response with a successful result.
        ResponseSuccess {
            /// Request identifier matching the outgoing request.
            id: JsonRpcRequestId,
            /// Result payload.
            result: Value,
        },
        /// Correlated request response with an error payload.
        ResponseError {
            /// Request identifier matching the outgoing request.
            id: JsonRpcRequestId,
            /// Error payload returned by the LSP server.
            error: Value,
        },
    }

    mod impl_incoming_lsp_message {
        #[allow(clippy::wildcard_imports)]
        use super::*;

        impl IncomingLspMessage {
            /// Parses an incoming raw JSON-RPC value into a categorized
            /// [`IncomingLspMessage`].
            ///
            /// Returns `None` if the JSON object is malformed or does not contain
            /// standard JSON-RPC response (`id`) or notification (`method`)
            /// fields.
            #[must_use]
            pub fn parse(json: &Value) -> Option<Self> {
                // 1. Response messages always have a numeric "id".
                if let Some(raw_id) = json.get("id").and_then(Value::as_u64) {
                    let id = JsonRpcRequestId(raw_id);
                    if let Some(error) = json.get("error").filter(|v| !v.is_null()) {
                        return Some(Self::ResponseError {
                            id,
                            error: error.clone(),
                        });
                    }
                    let result = json.get("result").cloned().unwrap_or(Value::Null);
                    return Some(Self::ResponseSuccess { id, result });
                }

                // 2. Notification messages have a "method" but NO "id".
                if let Some(method) = json.get("method").and_then(Value::as_str) {
                    if method == lsp_methods::PUBLISH_DIAGNOSTICS {
                        let params = json.get("params")?;
                        let uri = params.get("uri")?.as_str()?.to_string();
                        let diagnostics = params.get("diagnostics")?.as_array()?.clone();
                        return Some(Self::PublishDiagnostics { uri, diagnostics });
                    }
                    if method == lsp_methods::SERVER_STATUS {
                        let params = json.get("params")?;
                        let readiness: ServerReadiness =
                            serde_json::from_value(params.clone()).ok()?;
                        return Some(Self::ServerStatus(readiness));
                    }
                    return Some(Self::OtherNotification {
                        method: method.to_string(),
                    });
                }

                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{envelope_types::*, primitive_types::*, readiness_types::*,
                table_types::*};
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_spec_type_aliases() {
        let line: LspLineNumber = 0;
        let char_pos: LspCharPosition = 42;
        let req_id: JsonRpcRequestId = JsonRpcRequestId(100);
        let byte_len: LspPayloadByteLength = 1024;
        let mut writer: LspWriter = Box::new(std::io::Cursor::new(Vec::new()));
        assert!(writer.write_all(b"test").is_ok());

        let uri: DocumentUri = "file:///main.rs".to_string();
        let uri_ref: DocumentUriRef<'_> = &uri;
        assert_eq!(uri_ref, "file:///main.rs");
        let diag_list: DiagnosticList = vec![json!({"message": "error"})];
        let mut diag_table = DiagnosticsTable::new();
        diag_table.insert(uri.clone(), diag_list.clone());
        let safe_diag_table: SafeDiagnosticsTable = Arc::new(Mutex::new(diag_table));
        assert_eq!(safe_diag_table.lock().unwrap().get(&uri), Some(&diag_list));

        let (tx, _rx) = std::sync::mpsc::sync_channel::<Value>(1);
        let sender: ResponseSender = tx;
        let mut pending_table = PendingRequestsTable::new();
        pending_table.insert(req_id, sender);
        let safe_pending: SafePendingRequestsTable = Arc::new(Mutex::new(pending_table));
        assert!(safe_pending.lock().unwrap().contains_key(&req_id));

        assert_eq!(line, 0);
        assert_eq!(char_pos, 42);
        assert_eq!(req_id, JsonRpcRequestId(100));
        assert_eq!(req_id.0, 100);
        assert_eq!(byte_len, 1024);
    }

    #[test]
    fn test_json_rpc_request_id_default_and_next() {
        let mut id = JsonRpcRequestId::default();
        assert_eq!(id, JsonRpcRequestId(1));
        assert_eq!(id.next(), JsonRpcRequestId(1));
        assert_eq!(id, JsonRpcRequestId(2));
        assert_eq!(id.next(), JsonRpcRequestId(2));
        assert_eq!(id, JsonRpcRequestId(3));
    }

    #[test]
    fn test_json_rpc_request_id_serde_transparent() {
        let id = JsonRpcRequestId(42);
        let serialized = serde_json::to_string(&id).expect("Failed to serialize");
        assert_eq!(serialized, "42");

        let deserialized: JsonRpcRequestId =
            serde_json::from_str("42").expect("Failed to deserialize");
        assert_eq!(deserialized, JsonRpcRequestId(42));
    }

    #[test]
    fn test_lsp_request_new_constructor() {
        let req = LspRequest::new(
            JsonRpcRequestId(5),
            "textDocument/definition",
            Some(json!({ "uri": "file:///lib.rs" })),
        );
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, JsonRpcRequestId(5));
        assert_eq!(req.method, "textDocument/definition");
        assert_eq!(req.params, Some(json!({ "uri": "file:///lib.rs" })));
    }

    #[test]
    fn test_indexing_status_conversions() {
        assert_eq!(IndexingStatus::from(true), IndexingStatus::Complete);
        assert_eq!(IndexingStatus::from(false), IndexingStatus::InProgress);
        assert!(bool::from(IndexingStatus::Complete));
        assert!(!bool::from(IndexingStatus::InProgress));
    }

    #[test]
    fn test_server_readiness_default() {
        let readiness = ServerReadiness::default();
        assert_eq!(readiness.status, IndexingStatus::InProgress);
        assert_eq!(readiness.health, "ok");
        assert_eq!(readiness.message, None);
    }

    #[test]
    fn test_lsp_request_and_response_serialization() {
        let req = LspRequest {
            jsonrpc: "2.0".to_string(),
            id: JsonRpcRequestId(1),
            method: "textDocument/hover".to_string(),
            params: Some(json!({ "uri": "file:///main.rs" })),
        };
        let serialized_req =
            serde_json::to_string(&req).expect("Failed to serialize LspRequest");
        assert!(serialized_req.contains("textDocument/hover"));

        let resp = LspResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(JsonRpcRequestId(1)),
            result: Some(json!({ "contents": "hover text" })),
            error: None,
        };
        let serialized_resp =
            serde_json::to_string(&resp).expect("Failed to serialize LspResponse");
        assert!(serialized_resp.contains("hover text"));
    }

    #[test]
    fn test_incoming_lsp_message_parse_response_success() {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "result": { "contents": "hover text" }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::ResponseSuccess {
                id: JsonRpcRequestId(10),
                result: json!({ "contents": "hover text" }),
            })
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_response_error() {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 11,
            "error": { "code": -32603, "message": "Internal error" }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::ResponseError {
                id: JsonRpcRequestId(11),
                error: json!({ "code": -32603, "message": "Internal error" }),
            })
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_publish_diagnostics() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": "file:///path/to/main.rs",
                "diagnostics": [
                    { "message": "unused variable" }
                ]
            }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::PublishDiagnostics {
                uri: "file:///path/to/main.rs".to_string(),
                diagnostics: vec![json!({ "message": "unused variable" })],
            })
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_server_status_quiescent() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "experimental/serverStatus",
            "params": {
                "quiescent": true,
                "health": "ok"
            }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::ServerStatus(ServerReadiness {
                status: IndexingStatus::Complete,
                health: "ok".to_string(),
                message: None,
            }))
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_server_status_indexing() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "experimental/serverStatus",
            "params": {
                "quiescent": false,
                "health": "ok",
                "message": "Indexing (34/100)"
            }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::ServerStatus(ServerReadiness {
                status: IndexingStatus::InProgress,
                health: "ok".to_string(),
                message: Some("Indexing (34/100)".to_string()),
            }))
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_other_notification() {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": "window/logMessage",
            "params": {
                "type": 3,
                "message": "Server started"
            }
        });
        let parsed = IncomingLspMessage::parse(&payload);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::OtherNotification {
                method: "window/logMessage".to_string(),
            })
        );
    }

    #[test]
    fn test_incoming_lsp_message_parse_invalid() {
        let payload = json!({
            "jsonrpc": "2.0"
        });
        assert_eq!(IncomingLspMessage::parse(&payload), None);

        let malformed_notif = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": { "uri": "file:///path/to/main.rs" }
        });
        assert_eq!(IncomingLspMessage::parse(&malformed_notif), None);
    }

    #[test]
    fn test_incoming_lsp_message_parse_edge_cases() {
        let resp_with_null_error = json!({
            "jsonrpc": "2.0",
            "id": 42,
            "result": { "data": 1 },
            "error": null
        });
        let parsed = IncomingLspMessage::parse(&resp_with_null_error);
        assert_eq!(
            parsed,
            Some(IncomingLspMessage::ResponseSuccess {
                id: JsonRpcRequestId(42),
                result: json!({ "data": 1 }),
            })
        );

        let resp_empty_obj = json!({
            "jsonrpc": "2.0",
            "id": 99
        });
        let parsed_empty = IncomingLspMessage::parse(&resp_empty_obj);
        assert_eq!(
            parsed_empty,
            Some(IncomingLspMessage::ResponseSuccess {
                id: JsonRpcRequestId(99),
                result: Value::Null,
            })
        );

        let notif_no_params = json!({
            "jsonrpc": "2.0",
            "method": "custom/notification"
        });
        let parsed_custom = IncomingLspMessage::parse(&notif_no_params);
        assert_eq!(
            parsed_custom,
            Some(IncomingLspMessage::OtherNotification {
                method: "custom/notification".to_string(),
            })
        );
    }
}
