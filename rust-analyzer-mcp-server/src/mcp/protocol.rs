// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Model Context Protocol (MCP) data structures, JSON-RPC wire envelopes, and tool
//! schemas.

use crate::{constants::{json_rpc, mcp_content_types},
            error::McpServerError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An incoming Model Context Protocol request or notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version string (always `"2.0"`).
    pub jsonrpc: String,
    /// Request identifier. `None` for notifications.
    pub id: Option<Value>,
    /// Method name being invoked (e.g. `"tools/call"`).
    pub method: String,
    /// Optional parameter object or array.
    pub params: Option<Value>,
}

/// An outgoing Model Context Protocol response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResponse {
    /// Successful JSON-RPC response.
    Success {
        /// JSON-RPC version string (always `"2.0"`).
        jsonrpc: String,
        /// Request identifier matching the incoming request.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<Value>,
        /// Result payload.
        result: Value,
    },
    /// Erroneous JSON-RPC response.
    Error {
        /// JSON-RPC version string (always `"2.0"`).
        jsonrpc: String,
        /// Request identifier matching the incoming request.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<Value>,
        /// Error payload describing the failure.
        error: JsonRpcErrorPayload,
    },
}

impl McpResponse {
    /// Creates a successful MCP response for a given request ID.
    #[must_use]
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self::Success {
            jsonrpc: json_rpc::VERSION.to_string(),
            id,
            result,
        }
    }

    /// Creates an error MCP response for a given request ID.
    #[must_use]
    pub fn error(id: Option<Value>, error: JsonRpcErrorPayload) -> Self {
        Self::Error {
            jsonrpc: json_rpc::VERSION.to_string(),
            id,
            error,
        }
    }
}

/// Standard or application-defined error code defined by the [JSON-RPC 2.0 Error
/// Specification] (e.g. `-32600` through `-32700`, `-1`).
///
/// [JSON-RPC 2.0 Error Specification]: https://www.jsonrpc.org/specification#error_object
pub type JsonRpcErrorCode = i32;

/// JSON-RPC 2.0 error payload for wire transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorPayload {
    /// Standard or custom error code.
    pub code: JsonRpcErrorCode,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured error data.
    pub data: Option<Value>,
}

impl JsonRpcErrorPayload {
    /// Creates a new generic error response.
    #[must_use]
    pub fn custom(message: impl Into<String>) -> Self {
        Self {
            code: -1,
            message: message.into(),
            data: None,
        }
    }

    /// Creates an invalid params error response (code -32602).
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a method not found error response (code -32601).
    #[must_use]
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }
}

impl From<&McpServerError> for JsonRpcErrorPayload {
    fn from(err: &McpServerError) -> Self {
        match err {
            McpServerError::InvalidParam { key, reason } => {
                Self::invalid_params(format!("Invalid parameter '{key}': {reason}"))
            }
            McpServerError::MethodNotFound(method) => Self::method_not_found(method),
            _ => Self::custom(err.to_string()),
        }
    }
}

impl From<McpServerError> for JsonRpcErrorPayload {
    fn from(err: McpServerError) -> Self { Self::from(&err) }
}

/// Schema and metadata definition for an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique name of the tool.
    pub name: String,
    /// Description of the tool's behavior and utility.
    pub description: String,
    /// JSON Schema describing expected arguments.
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Result of executing an MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content items produced by the tool execution.
    pub content: Vec<ContentItem>,
}

impl ToolResult {
    /// Convenience helper to create a plain text [`ToolResult`].
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ContentItem {
                content_type: mcp_content_types::TEXT.to_string(),
                text: text.into(),
            }],
        }
    }

    /// Convenience helper to create a pretty-printed JSON [`ToolResult`].
    ///
    /// # Errors
    ///
    /// Returns [`serde_json::Error`] if JSON serialization fails.
    pub fn json_pretty(val: &impl Serialize) -> Result<Self, serde_json::Error> {
        let json_text = serde_json::to_string_pretty(val)?;
        let tool_result = Self::text(json_text);
        Ok(tool_result)
    }
}

/// An individual content item in a [`ToolResult`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    /// Content MIME type or format (e.g. [`mcp_content_types::TEXT`]).
    #[serde(rename = "type")]
    pub content_type: String,
    /// Payload text content.
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_request_deserialization() {
        let raw = r#"{
        "jsonrpc": "2.0",
        "id": 42,
        "method": "tools/call",
        "params": {
            "name": "rust_analyzer_hover",
            "arguments": {
                "file_path": "src/lib.rs",
                "line": 10,
                "character": 5
            }
        }
    }"#;

        let req: McpRequest =
            serde_json::from_str(raw).expect("Failed to deserialize request");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(json!(42)));
        assert_eq!(req.method, "tools/call");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_mcp_response_serialization() {
        let success = McpResponse::success(Some(json!(1)), json!({ "status": "ok" }));

        let serialized =
            serde_json::to_string(&success).expect("Failed to serialize success");
        assert!(serialized.contains(r#""result":{"status":"ok"}"#));

        let error = McpResponse::error(
            Some(json!(2)),
            JsonRpcErrorPayload::custom("Something went wrong"),
        );

        let serialized_err =
            serde_json::to_string(&error).expect("Failed to serialize error");
        assert!(serialized_err.contains(r#""message":"Something went wrong""#));
        assert!(serialized_err.contains(r#""code":-1"#));
    }

    #[test]
    fn test_mcp_server_error_conversion() {
        let err = McpServerError::InvalidParam {
            key: "file_path".to_string(),
            reason: "not found".to_string(),
        };
        let payload = JsonRpcErrorPayload::from(&err);
        assert_eq!(payload.code, -32602);
        assert!(payload.message.contains("file_path"));

        let method_err = McpServerError::MethodNotFound("foo".to_string());
        let payload2 = JsonRpcErrorPayload::from(&method_err);
        assert_eq!(payload2.code, -32601);
    }

    #[test]
    fn test_tool_result_construction() {
        let result = ToolResult::text("Hello world");
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].content_type, mcp_content_types::TEXT);
        assert_eq!(result.content[0].text, "Hello world");

        let json_val = json!({ "key": "value" });
        let json_result = ToolResult::json_pretty(&json_val).unwrap();
        assert_eq!(json_result.content.len(), 1);
        assert_eq!(json_result.content[0].content_type, mcp_content_types::TEXT);
        assert!(json_result.content[0].text.contains("\"key\": \"value\""));
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param": { "type": "string" }
                }
            }),
        };

        let serialized = serde_json::to_string(&tool).expect("Failed to serialize");
        assert!(serialized.contains("test_tool"));
        assert!(serialized.contains("inputSchema"));
    }
}
