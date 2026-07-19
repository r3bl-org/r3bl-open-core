// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Strongly-typed error enum and diagnostic conversions for the MCP server.

use miette::Diagnostic;
use thiserror::Error;

/// Strongly-typed error enum for MCP server operations.
#[derive(Debug, Error, Diagnostic)]
pub enum McpServerError {
    /// Failed to spawn rust-analyzer process.
    #[error("Failed to spawn rust-analyzer process: {0}")]
    ProcessSpawn(String),

    /// Failed to capture stdin, stdout, or stderr.
    #[error("Failed to capture I/O pipe for rust-analyzer: {0}")]
    ProcessPipe(String),

    /// Request timed out waiting for rust-analyzer response.
    #[error("rust-analyzer request timed out after {0}s")]
    RequestTimeout(u64),

    /// Request was cancelled.
    #[error("rust-analyzer request was cancelled")]
    RequestCancelled,

    /// Language server client is not available or not running.
    #[error("Language server client is not available")]
    ClientNotAvailable,

    /// Requested tool name is not recognized.
    #[error("Unknown tool requested: '{0}'")]
    UnknownTool(String),

    /// Requested JSON-RPC method is not recognized.
    #[error("Method not found: '{0}'")]
    MethodNotFound(String),

    /// Parameter validation failed.
    #[error("Missing or invalid argument for parameter '{key}': {reason}")]
    InvalidParam {
        /// Parameter key name.
        key: String,
        /// Description of why validation failed.
        reason: String,
    },

    /// Communication channel between tasks was closed.
    #[error("Communication channel closed: {0}")]
    ChannelClosed(String),

    /// `stdio` error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Custom error message.
    #[error("{0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_server_error_display() {
        let err = McpServerError::ProcessSpawn("ra not found".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to spawn rust-analyzer process: ra not found"
        );

        let err2 = McpServerError::UnknownTool("invalid_tool".to_string());
        assert_eq!(err2.to_string(), "Unknown tool requested: 'invalid_tool'");

        let err3 = McpServerError::InvalidParam {
            key: "line".to_string(),
            reason: "must be positive".to_string(),
        };
        assert_eq!(
            err3.to_string(),
            "Missing or invalid argument for parameter 'line': must be positive"
        );
    }
}
