// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`rust-analyzer`] Model Context Protocol (MCP) Server
//!
//! This module provides a Model Context Protocol (MCP) server that acts as a lightweight,
//! synchronous translation bridge between an AI / LLM coding agent or IDE client and the
//! [`rust-analyzer`] language server subprocess.
//!
//! # Architecture: A 3-Process UNIX Pipe Bridge
//!
//! The complete interaction workflow consists of five distinct steps:
//!
//! 1. **User Launches Agent**
//!     - The user starts their AI / LLM coding agent (such as Google Antigravity `agy`,
//!       Claude Code, Cursor, or VS Code) in a local folder containing a Rust project.
//! 2. **Agent Configuration**
//!     - The user configures the agent to register `rust-analyzer-mcp-server` as a local
//!       tool provider (via its MCP settings or configuration file).
//! 3. **User Prompts the Agent**
//!     - In an active coding session, the user issues a prompt requiring Rust semantic
//!       analysis (e.g., _"Find all references to `MyStruct` and refactor its
//!       constructor"_).
//! 4. **Agent Spawns Bridge Process**
//!     - To execute the requested tools, the AI / LLM coding agent spawns
//!       `rust-analyzer-mcp-server` as a dedicated 1:1 child process communicating over
//!       `stdio` (`stdin` and `stdout`).
//! 5. **Bridge Translates MCP to LSP**
//!     - The MCP server receives JSON-RPC tool calls over `stdin`, converts them into LSP
//!       queries for `rust-analyzer`, and writes formatted responses to `stdout`:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │          HOST PROCESS: AI / LLM CODING AGENT           │
//! │            (Antigravity / VSCode / Claude)             │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!         [Launches rust-analyzer-mcp-server binary]
//!               (1:1 dedicated child process)
//!                             │
//!            stdin / stdout pipes (MCP JSON-RPC)
//!                             │
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │                rust-analyzer-mcp-server                │
//! │   (3 OS threads: Main, stdout-reader, stderr-reader)   │
//! │                                                        │
//! │  1. Reads MCP JSON-RPC tool calls from stdin           │
//! │  2. Converts tool calls into LSP JSON-RPC queries      │
//! │  3. Writes LSP requests to rust-analyzer stdin         │
//! │  4. Receives LSP responses from background thread      │
//! │  5. Formats and writes MCP responses to stdout         │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!             [Spawns rust-analyzer subprocess]
//!               (1:1 dedicated child process)
//!                             │
//!             stdin / stdout pipes (LSP Framing)
//!                             │
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │       LANGUAGE SERVER: rust-analyzer subprocess        │
//! │               (The Rust Language Server)               │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! # How MCP Layers on Top of Language Servers
//!
//! The Model Context Protocol (MCP) does not define language-server-specific endpoints.
//! Instead, MCP provides generic JSON-RPC primitives:
//!
//! 1. `initialize` (Handshake & capability negotiation)
//! 2. `tools/list` (Dynamic tool discovery via JSON Schemas)
//! 3. `tools/call` (Tool execution request & response)
//!
//! This crate (`rust-analyzer-mcp-server`) serves as an **adapter**. It implements those
//! standard MCP endpoints on the outside and internally translates them into
//! [`rust-analyzer`]'s Language Server Protocol (LSP 3.17) queries.
//!
//! ## Protocol Layering Flow
//!
//! ```text
//! ┌────────────────────────────────────────────────────────┐
//! │               AI / LLM CODING AGENT (agy)              │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!               1. "tools/list" (Standard MCP)
//!                             │
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │                rust-analyzer-mcp-server                │
//! │                                                        │
//! │  Returns list of 10 tool schemas:                      │
//! │   • rust_analyzer_hover                                │
//! │   • rust_analyzer_definition                           │
//! │   • rust_analyzer_references                           │
//! │   • rust_analyzer_diagnostics                          │
//! │   • ...                                                │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!               2. Agent injects tool schemas into LLM
//!                             │
//!               3. LLM decides: call "rust_analyzer_hover"
//!                             │
//!               4. "tools/call" (Standard MCP)
//!                             │
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │                rust-analyzer-mcp-server                │
//! │                                                        │
//! │  Translates tools/call -> textDocument/hover (LSP)     │
//! └───────────────────────────┬────────────────────────────┘
//!                             │
//!               5. LSP JSON-RPC over stdio
//!                             │
//!                             ▼
//! ┌────────────────────────────────────────────────────────┐
//! │                 rust-analyzer subprocess               │
//! └────────────────────────────────────────────────────────┘
//! ```
//!
//! The sections below trace this chronological execution flow across four distinct steps:
//!
//! 1. Discovery (`tools/list`): The AI / LLM coding agent queries capabilities; our MCP
//!    server immediately replies with tool schemas that we define, without invoking
//!    [`rust-analyzer`].
//! 2. Tool Request (`tools/call`): The AI / LLM coding agent dispatches a tool execution
//!    request, e.g., `rust_analyzer_hover` with a file path and coordinates (`line: 42,
//!    character: 10`).
//! 3. Bridge Translation (MCP -> LSP): Our MCP server translates the MCP tool call into
//!    an LSP query (`textDocument/hover`), writes it to [`rust-analyzer`], and awaits the
//!    AST response.
//! 4. Tool Response (`CallToolResult`): [`rust-analyzer`] produces the symbol information
//!    (with hover documentation and type signatures natively formatted in Markdown). Our
//!    MCP server packages this into a standard MCP `CallToolResult` and returns the
//!    JSON-RPC response to the AI / LLM coding agent with the matching request ID.
//!
//! ## 1. Dynamic Tool Discovery (`tools/list`)
//!
//! When the AI / LLM coding agent (e.g. `agy`) spawns our MCP server, it queries
//! available tools by sending an MCP `tools/list` request over `stdin`:
//!
//! ```json
//! {"jsonrpc": "2.0", "id": 1, "method": "tools/list"}
//! ```
//!
//! > [`rust-analyzer`] is not used at all to produce this response. Because
//! > [`rust-analyzer`] is a pure LSP language server with zero awareness of MCP, our
//! > bridge binary (`rust-analyzer-mcp-server`) statically defines and serves all 10 tool
//! > descriptors, documentation, and JSON Schemas directly from its internal catalog
//! > ([`get_tools`]).
//!
//! Our MCP server writes this JSON-RPC response back to the AI / LLM coding agent over
//! `stdout`:
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 1,
//!   "result": {
//!     "tools": [
//!       {
//!         "name": "rust_analyzer_hover",
//!         "description": "Hover documentation, types, and signatures for a symbol.",
//!         "inputSchema": {
//!           "type": "object",
//!           "properties": {
//!             "file_path": { "type": "string", "description": "Absolute file path" },
//!             "line": { "type": "integer", "description": "0-based line number" },
//!             "character": { "type": "integer", "description": "0-based character offset" }
//!           },
//!           "required": ["file_path", "line", "character"]
//!         }
//!       },
//!       {
//!         "name": "rust_analyzer_definition",
//!         "description": "Find symbol definition location.",
//!         "inputSchema": {}
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! The coding agent registers these schemas with the LLM, enabling the model to invoke
//! any of the 10 tools when inspecting code.
//!
//! ## 2. Tool Execution (`tools/call`)
//!
//! When the LLM decides to inspect a symbol at line 42, character 10, the coding agent
//! sends a standard MCP `tools/call` request to our MCP server's `stdin`:
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 2,
//!   "method": "tools/call",
//!   "params": {
//!     "name": "rust_analyzer_hover",
//!     "arguments": {
//!       "file_path": "/path/to/main.rs",
//!       "line": 42,
//!       "character": 10
//!     }
//!   }
//! }
//! ```
//!
//! ## 3. The Bridge Translation: MCP to LSP
//!
//! Our MCP server translates the tool name and arguments into an LSP 3.17 request:
//!
//! | Standard MCP Method / Tool Name                      | Translates To LSP 3.17 Method                |
//! | :--------------------------------------------------- | :------------------------------------------- |
//! | `tools/call` (`rust_analyzer_hover`)                 | `textDocument/hover`                         |
//! | `tools/call` (`rust_analyzer_definition`)            | `textDocument/definition`                    |
//! | `tools/call` (`rust_analyzer_references`)            | `textDocument/references`                    |
//! | `tools/call` (`rust_analyzer_symbols`)               | `textDocument/documentSymbol`                |
//! | `tools/call` (`rust_analyzer_completion`)            | `textDocument/completion`                    |
//! | `tools/call` (`rust_analyzer_format`)                | `textDocument/formatting`                    |
//! | `tools/call` (`rust_analyzer_code_actions`)          | `textDocument/codeAction`                    |
//! | `tools/call` (`rust_analyzer_diagnostics`)           | `textDocument/diagnostic` (or push fallback) |
//! | `tools/call` (`rust_analyzer_workspace_diagnostics`) | `workspace/diagnostic`                       |
//! | `tools/call` (`rust_analyzer_set_workspace`)         | `workspace/didChangeConfiguration`           |
//!
//! ## 4. Returning the Response to the AI / LLM Coding Agent
//!
//! [`rust-analyzer`] answers the LSP query with symbol data (for `textDocument/hover`, it
//! natively formats the Rust signature and doc comments as Markdown in its
//! `MarkupContent` response). Our MCP server packages this into a standard MCP
//! `CallToolResult` text payload and returns it:
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "id": 2,
//!   "result": {
//!     "content": [
//!       {
//!         "type": "text",
//!         "text": "...fn foo() -> bool...Do stuff..."
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! The coding agent passes this text directly back into the LLM context so it can
//! continue reasoning.
//!
//! ## Why Asynchronous Request Multiplexing is Unnecessary Over `stdio`
//!
//! Neither JSON-RPC 2.0 nor the MCP specification dictates the transport layer. You can
//! run JSON-RPC over `WebSockets`, TCP, Unix domain sockets, or `stdio`. Over network
//! sockets (such as TCP or `WebSockets`), async request multiplexing makes complete
//! sense: thousands of remote clients connect to a single daemon, each over its own
//! independent socket connection.
//!
//! However, when an AI / LLM coding agent launches our MCP server locally, communication
//! relies on **OS anonymous pipes** (`stdin` and `stdout`). An OS pipe is a
//! unidirectional, in-memory FIFO buffer in the kernel (typically 64 KiB on Linux).
//! Because these are single serialized byte streams, communication between the coding
//! agent and our server is inherently sequential by design:
//!
//! 1. **`stdin` (Accepting Requests from the Coding Agent):** The coding agent writes MCP
//!    JSON-RPC requests into our MCP server's `stdin` pipe, delimited by newlines (`\n`).
//!    Reading from `stdin` must be serialized: multiple concurrent reader threads would
//!    race to consume chunks from the byte stream, splitting JSON lines in half and
//!    breaking JSON parsing. A single synchronous reader loop in our main thread
//!    naturally consumes incoming requests line-by-line with zero synchronization
//!    overhead.
//!
//! 2. **`stdout` (Returning Responses to the Coding Agent):** Our MCP server writes
//!    JSON-RPC tool responses back to the coding agent over `stdout`. Multiple concurrent
//!    threads or tasks cannot write to `stdout` simultaneously without garbling the
//!    response stream. Even if a multi-threaded async runtime executes tool queries
//!    concurrently, all tasks must ultimately synchronize on a mutex to write their
//!    responses line-by-line. An async runtime cannot provide parallel I/O throughput
//!    over a single pipe; it simply adds mutex contention and task scheduling overhead.
//!
//! 3. **Turn-Based Agent Structure:** LLM workflows operate in discrete conversational
//!    turns. When the LLM generates a tool call, the coding agent writes the request to
//!    our MCP server's `stdin` and waits for the tool output before prompting the model
//!    for the next reasoning step. Because AST tool execution is so fast (1 to 10 ms)
//!    compared to LLM inference (500 to 3,000 ms), sequential execution latency is
//!    completely imperceptible to the user.
//!
//! 4. **The Synchronous Event Loop:** In
//!    [`RustAnalyzerMCPServer::enter_main_event_loop`], the main thread runs a
//!    single-threaded line reader with no lock contention. Each MCP request is consumed,
//!    translated into an LSP request, synchronously resolved against [`rust-analyzer`],
//!    and the response is written back to `stdout` before the next line is read.
//!    - This doesn't add perceptible latency in agent loops. LLM inference (e.g. Claude,
//!      Gemini, etc.) may take 500 ms to 3,000 ms+ per conversational turn to reason and
//!      generate tokens. A tool execution latency of 1 to 10 ms represents less than 1%
//!      of that turn latency. Even if an agent issues a batch of 3 sequential tool calls
//!      (3 × 5 ms = 15 ms total), the execution time is completely imperceptible compared
//!      to the seconds spent in LLM network inference.
//!
//! # Threading Model
//!
//! Because [`rust-analyzer`] communicates asynchronously over `stdio` with
//! out-of-order responses and unprompted notifications (such as compiler diagnostics),
//! the server utilizes exactly 3 threads:
//!
//! 1. Main Thread (`main`):
//!    - Spawned by: OS (Process entry).
//!    - Function: [`RustAnalyzerMCPServer::enter_main_event_loop`].
//!    - Purpose: Drives the MCP event loop: reads newline-delimited MCP requests from
//!      client `stdin`, translates them to LSP requests, writes to [`rust-analyzer`]
//!      `stdin`, waits on reply channels, and writes MCP responses to client `stdout`.
//! 2. Stdout Reader Thread (`lsp-stdout-reader`):
//!    - Spawned by: [`RustAnalyzerClient::start`].
//!    - Function: [`spawn_stdout_reader_thread`].
//!    - Purpose: Continuously reads `Content-Length` framed JSON-RPC from
//!      [`rust-analyzer`]'s `stdout`: dispatches response payloads to the waiting main
//!      thread via `pending_requests`, and stores compiler diagnostics in `diagnostics`.
//! 3. Stderr Reader Thread (`lsp-stderr-reader`):
//!    - Spawned by: [`RustAnalyzerClient::start`].
//!    - Function: [`spawn_stderr_reader_thread`].
//!    - Purpose: Continuously drains [`rust-analyzer`]'s `stderr` pipe to prevent OS
//!      kernel pipe buffer deadlocks (64 KiB limit), and outputs structured logs when
//!      [`DEBUG_LSP_CLIENT`] is active.
//!
//! ## Request-Response Correlation
//!
//! When the main thread sends an LSP request:
//! 1. A single-use [`sync_channel(1)`] pair is created.
//! 2. The transmitter (`SyncSender`) is registered in the `pending_requests` [`HashMap`]
//!    keyed by the unique `request_id`.
//! 3. The request is written to [`rust-analyzer`]'s `stdin`.
//! 4. The main thread blocks waiting on the receiver with a timeout.
//! 5. When [`rust-analyzer`] writes the response to `stdout`, the `lsp-stdout-reader`
//!    thread matches the `request_id`, removes the transmitter from `pending_requests`,
//!    and delivers the payload to unblock the main thread.
//!
//! ## Server Readiness & Background Indexing Synchronization
//!
//! On cold startup or initial workspace loading, [`rust-analyzer`] operates
//! asynchronously, performing background crate indexing, proc-macro expansion, and
//! workspace metadata resolution. During this initial scan:
//!
//! 1. **Eager Subprocess Startup**: The MCP server eagerly spawns and initializes
//!    [`rust-analyzer`] upon main loop entry
//!    ([`RustAnalyzerMCPServer::enter_main_event_loop`]).
//! 2. **Readiness Monitoring**: The server subscribes to `experimental/serverStatus`
//!    notifications and monitors indexing progress via [`ServerReadinessMonitor`].
//! 3. **AST Query Gating**: AST inspection tools (`hover`, `definition`, `references`,
//!    `symbols`, `completion`, `code_actions`) synchronize with indexing status. If a
//!    query returns empty results while [`rust-analyzer`] is still indexing, the server
//!    provides a clear indexing progress notification instead of a false negative ("not
//!    found").
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────────┐
//! │ 1. Eager Startup (RustAnalyzerMCPServer::enter_main_event_loop)                 │
//! │    - Immediately spawn rust-analyzer child process upon server start.           │
//! │    - Perform initialize handshake in parallel with MCP client connection.       │
//! │    - Advertise capability: "experimental": { "serverStatusNotification": true } │
//! └──────────────────────────────────────┬──────────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ 2. Background Status Ingestion (lsp::transport::process_incoming_json)      │
//! │    - Parse incoming "experimental/serverStatus" notifications.              │
//! │    - Atomically update ServerReadinessMonitor (Mutex<ServerReadiness>).     │
//! │    - Call condvar.notify_all() whenever status transitions to Complete.     │
//! └──────────────────────────────────────┬──────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │ 3. AST Tool Readiness Gating (hover, definition, references, symbols, etc.) │
//! │    - Check if server indexing is Complete.                                  │
//! │      ├── If Complete: Execute AST query immediately (0ms overhead).         │
//! │      └── If InProgress: Wait on Condvar (up to 5s warmup timeout).          │
//! │    - After wait:                                                            │
//! │      ├── If Complete: Execute query immediately.                            │
//! │      └── If timeout expired (InProgress):                                   │
//! │          ├── Attempt best-effort query anyway.                              │
//! │          ├── If results returned: Return results (e.g. syntax symbols).     │
//! │          └── If empty/null: Return informative indexing progress message    │
//! │              with live rust-analyzer progress instead of false negative.    │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Organization
//!
//! - [`cli_arg`]: Command-line argument parsing for workspace paths and logging.
//! - [`constants`]: Namespaced timing, protocol, framing, and tool identifier constants.
//! - [`error`]: Strongly-typed error enum ([`McpServerError`]) and diagnostic
//!   conversions.
//! - [`lsp`]: Subprocess management, framing, AST queries, diagnostics, and server
//!   readiness monitor.
//! - [`mcp`]: `stdio` JSON-RPC event loop, tool registry, and MCP wire envelopes.
//! - [`value_ext`]: Extension trait on [`serde_json::Value`] for parameter extraction.
//!
//! [`cli_arg`]: crate::cli_arg
//! [`constants`]: crate::constants
//! [`DEBUG_LSP_CLIENT`]: crate::constants::debug_flags::DEBUG_LSP_CLIENT
//! [`error`]: crate::error
//! [`HashMap`]: std::collections::HashMap
//! [`lsp`]: crate::lsp
//! [`mcp`]: crate::mcp
//! [`McpServerError`]: crate::error::McpServerError
//! [`rust-analyzer`]: https://rust-analyzer.github.io/
//! [`RustAnalyzerClient::start`]: crate::lsp::RustAnalyzerClient::start
//! [`RustAnalyzerMCPServer::enter_main_event_loop`]:
//!     crate::mcp::RustAnalyzerMCPServer::enter_main_event_loop
//! [`ServerReadinessMonitor`]: crate::lsp::ServerReadinessMonitor
//! [`spawn_stderr_reader_thread`]:
//!     crate::lsp::RustAnalyzerClient::spawn_stderr_reader_thread
//! [`spawn_stdout_reader_thread`]:
//!     crate::lsp::RustAnalyzerClient::spawn_stdout_reader_thread
//! [`sync_channel(1)`]: std::sync::mpsc::sync_channel
//! [`value_ext`]: crate::value_ext

// Attach modules.
pub mod cli_arg;
pub mod constants;
pub mod error;
pub mod lsp;
pub mod mcp;
pub mod value_ext;

// Re-export public API for flat module interface.
pub use cli_arg::*;
pub use constants::*;
pub use error::*;
pub use lsp::*;
pub use mcp::*;
pub use value_ext::*;
