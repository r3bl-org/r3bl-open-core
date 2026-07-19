// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Global constants, protocol definitions, and configuration defaults for the MCP server.
//!
//! This module organizes all timing thresholds, protocol metadata, method names, tool
//! names, parameter keys, and debug flags into namespaced inner modules for type safety
//! and clarity.

/// Boolean flags gating structured tracing throughout the MCP server subsystems.
pub mod debug_flags {
    /// Controls structured tracing for the MCP JSON-RPC stdio server and request router.
    pub const DEBUG_MCP_SERVER: bool = true;

    /// Controls structured tracing for the underlying rust-analyzer LSP child process and
    /// stream framing.
    pub const DEBUG_LSP_CLIENT: bool = true;

    /// Controls structured tracing for individual MCP tool executions and diagnostic
    /// formatting.
    pub const DEBUG_MCP_TOOLS: bool = true;
}

/// JSON-RPC protocol specification constants.
pub mod json_rpc {
    /// Supported JSON-RPC specification version.
    pub const VERSION: &str = "2.0";

    /// Initial request ID for monotonically increasing JSON-RPC requests.
    pub const INITIAL_REQUEST_ID: u64 = 1;
}

/// Model Context Protocol (MCP) specification metadata.
pub mod mcp_protocol {
    /// Supported Model Context Protocol specification version.
    pub const VERSION: &str = "2024-11-05";

    /// MCP server implementation name.
    pub const SERVER_NAME: &str = "rust-analyzer-mcp-server";

    /// MCP server implementation version.
    pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
}

/// MCP JSON-RPC protocol method names.
pub mod mcp_methods {
    /// MCP initialization handshake request method.
    pub const INITIALIZE: &str = "initialize";

    /// MCP initialized notification method.
    pub const NOTIFICATIONS_INITIALIZED: &str = "notifications/initialized";

    /// MCP tool discovery request method.
    pub const TOOLS_LIST: &str = "tools/list";

    /// MCP tool execution request method.
    pub const TOOLS_CALL: &str = "tools/call";
}

/// Model Context Protocol (MCP) content types and payload formats.
pub mod mcp_content_types {
    /// Plain text content type descriptor for MCP tool call responses.
    pub const TEXT: &str = "text";
}

/// Names of MCP tools exposed by this server.
pub mod tool_names {
    /// AST hover inspection tool name.
    pub const HOVER: &str = "rust_analyzer_hover";

    /// Go to definition tool name.
    pub const DEFINITION: &str = "rust_analyzer_definition";

    /// Find symbol references tool name.
    pub const REFERENCES: &str = "rust_analyzer_references";

    /// Code completion suggestions tool name.
    pub const COMPLETION: &str = "rust_analyzer_completion";

    /// Document and workspace AST symbols tool name.
    pub const SYMBOLS: &str = "rust_analyzer_symbols";

    /// Document code formatting tool name.
    pub const FORMAT: &str = "rust_analyzer_format";

    /// Code actions and quick fixes tool name.
    pub const CODE_ACTIONS: &str = "rust_analyzer_code_actions";

    /// Dynamic workspace configuration tool name.
    pub const SET_WORKSPACE: &str = "rust_analyzer_set_workspace";

    /// Single file diagnostics tool name.
    pub const DIAGNOSTICS: &str = "rust_analyzer_diagnostics";

    /// Workspace-wide diagnostics tool name.
    pub const WORKSPACE_DIAGNOSTICS: &str = "rust_analyzer_workspace_diagnostics";
}

/// Parameter keys expected in MCP tool invocation payloads.
pub mod param_names {
    /// MCP tool name parameter key in `tools/call`.
    pub const NAME: &str = "name";

    /// MCP tool arguments dictionary key in `tools/call`.
    pub const ARGUMENTS: &str = "arguments";

    /// Target file path relative to workspace or absolute.
    pub const FILE_PATH: &str = "file_path";

    /// 0-indexed line number.
    pub const LINE: &str = "line";

    /// 0-indexed character/column offset.
    pub const CHARACTER: &str = "character";

    /// 0-indexed ending line number for range selection.
    pub const END_LINE: &str = "end_line";

    /// 0-indexed ending character/column offset for range selection.
    pub const END_CHARACTER: &str = "end_character";

    /// Absolute or relative workspace path to switch active directory to.
    pub const WORKSPACE_PATH: &str = "workspace_path";
}

/// Standardized human-readable severity descriptors for LSP diagnostics.
pub mod lsp_diagnostic_severities {
    /// Error diagnostic severity.
    pub const ERROR: &str = "error";

    /// Warning diagnostic severity.
    pub const WARNING: &str = "warning";

    /// Informational diagnostic severity.
    pub const INFORMATION: &str = "information";

    /// Hint diagnostic severity.
    pub const HINT: &str = "hint";

    /// Unknown / unrecognized diagnostic severity.
    pub const UNKNOWN: &str = "unknown";
}

/// Language Server Protocol (LSP) method and notification names.
pub mod lsp_methods {
    /// LSP server initialization request method.
    pub const INITIALIZE: &str = "initialize";

    /// LSP initialized notification method.
    pub const INITIALIZED: &str = "initialized";

    /// LSP server shutdown request method.
    pub const SHUTDOWN: &str = "shutdown";

    /// LSP server exit notification method.
    pub const EXIT: &str = "exit";

    /// LSP document opened notification method.
    pub const DID_OPEN: &str = "textDocument/didOpen";

    /// LSP document saved notification method.
    pub const DID_SAVE: &str = "textDocument/didSave";

    /// LSP hover query request method.
    pub const HOVER: &str = "textDocument/hover";

    /// LSP go to definition request method.
    pub const DEFINITION: &str = "textDocument/definition";

    /// LSP find references request method.
    pub const REFERENCES: &str = "textDocument/references";

    /// LSP document symbols request method.
    pub const DOCUMENT_SYMBOL: &str = "textDocument/documentSymbol";

    /// LSP code completion request method.
    pub const COMPLETION: &str = "textDocument/completion";

    /// LSP document formatting request method.
    pub const FORMATTING: &str = "textDocument/formatting";

    /// LSP code action request method.
    pub const CODE_ACTION: &str = "textDocument/codeAction";

    /// LSP pull diagnostics request method.
    pub const DIAGNOSTIC: &str = "textDocument/diagnostic";

    /// LSP push diagnostics notification method.
    pub const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";

    /// LSP experimental server status notification method.
    pub const SERVER_STATUS: &str = "experimental/serverStatus";
}

/// LSP protocol framing, binary names, and language identifiers.
pub mod lsp_framing {
    /// URI scheme prefix for local file paths.
    pub const FILE_URI_PREFIX: &str = "file://";

    /// Header prefix for HTTP-style Content-Length framing.
    pub const CONTENT_LENGTH_HEADER_PREFIX: &str = "Content-Length: ";

    /// Default binary executable name for rust-analyzer.
    pub const RUST_ANALYZER_BINARY: &str = "rust-analyzer";

    /// Language identifier for Rust source files in LSP messages.
    pub const RUST_LANGUAGE_ID: &str = "rust";
}

/// Timeouts and timing intervals for LSP client and diagnostics.
pub mod timing {
    /// Timeout duration in seconds for LSP requests before failing.
    pub const LSP_REQUEST_TIMEOUT_SECS: u64 = 30;

    /// Delay in milliseconds after opening or saving a document to allow rust-analyzer
    /// indexing.
    pub const DOCUMENT_OPEN_DELAY_MILLIS: u64 = 200;

    /// Maximum duration in seconds to poll for diagnostics in test/error scenarios.
    pub const DIAGNOSTICS_POLL_TIMEOUT_SECS: u64 = 8;

    /// Interval in milliseconds between diagnostic poll attempts.
    pub const DIAGNOSTICS_POLL_INTERVAL_MILLIS: u64 = 500;

    /// Delay in seconds before querying diagnostics for clean files.
    pub const DIAGNOSTICS_CLEAN_FILE_DELAY_SECS: u64 = 2;

    /// Default tab size used for rust-analyzer formatting requests.
    pub const DEFAULT_TAB_SIZE: u32 = 4;

    /// Maximum duration in seconds to wait for initial workspace indexing to complete
    /// before executing AST queries.
    pub const LSP_INDEXING_WARMUP_TIMEOUT_SECS: u64 = 5;
}
