# Task: Create `rust-analyzer-mcp-server` in `build-infra`

## Overview

Provide a native Model Context Protocol (MCP) server for `rust-analyzer` within the
`build-infra` crate (`r3bl-build-infra`). This enables AI coding assistants (such as
Claude Desktop, Antigravity, and Cursor) to interact with `rust-analyzer` directly over
standard I/O (JSON-RPC) for semantic code intelligence, AST navigation, compiler
diagnostics, and automated quick-fixes.

### Why Build a Custom Server in `build-infra`?

1. **Defects in `zeenix/rust-analyzer-mcp` (v0.2.0):**
    - **Protocol Violation on Notifications:** It improperly returned a JSON-RPC error
      (`-32600`) for notification messages without an `id` (such as
      `notifications/initialized`), breaking standard MCP client handshakes.
    - **Heavy Async Runtime Overhead:** It required a full `tokio` async runtime with
      async lock contention and task lifecycle fragility for what is fundamentally a
      sequential standard I/O pipe filter.
    - **Feature Limitations:** It lacked dynamic workspace switching (`set_workspace`),
      on-save diagnostic polling, and robust fallback path canonicalization.

2. **Defects in Previous Vendored Crate (`dexwritescode/rust-mcp`):**
    - **100% CPU Infinite Busy Loop on EOF:** Recreating `BufReader` on each request
      caused dropped stream bytes and spun on 0-byte reads when `rust-analyzer` exited,
      pegging CPU cores.
    - **Mock/Placeholder Tools:** 14 out of 19 tools were non-functional dummy stubs
      rather than true compiler-backed operations.

3. **Advantages of Our Native `build-infra` Implementation:**
    - **Zero Async Runtime:** Pure standard library concurrency (`std::process`,
      `std::thread`, `std::sync::mpsc::sync_channel(1)`).
    - **Deadlock-Free 3-Thread Architecture:** Main event loop, stdout response/diagnostic
      parser, and background stderr drainer preventing OS kernel pipe buffer deadlocks (64
      KB limit).
    - **Full First-Class Toolset:** 10 fully implemented tools with dynamic workspace
      switching, structured tracing, and robust error recovery.

The server exposes 10 semantic code analysis tools: AST symbol search, hover inspection,
go to definition, find references, code completions, formatting, code actions,
diagnostics, workspace diagnostics, and dynamic workspace configuration.

The server operates as a synchronous 3-process UNIX pipe bridge using standard library
primitives (`std::process`, `std::thread`, `std::sync::mpsc`):

1. **CLI Argument Parsing (`cli_arg.rs`)**: Flexible workspace path resolution, log level
   filtering, and optional file logging with structured tracing.
2. **Namespaced Constants (`constants.rs`)**: Protocol constants, timeouts, and debug
   flags grouped into inner modules (`debug_flags`, `mcp_protocol`, `mcp_methods`,
   `tool_names`, `param_names`, `lsp_methods`, `lsp_framing`, `timing`).
3. **Type Definitions (`types.rs`)**: Strongly typed JSON-RPC message schemas, MCP tool
   definitions, and structured `McpServerError` types.
4. **LSP Subprocess Management (`lsp_client.rs`)**: Subprocess management using
   `std::process`, header framing, synchronous request-response correlation via
   `std::sync::mpsc::sync_channel(1)`, and background compiler diagnostics caching.
5. **JSON-RPC Server Dispatch (`server.rs`)**: Synchronous standard I/O event loop,
   handshake lifecycle, tool execution routing, and graceful shutdown.
6. **Tool Handlers (`tools.rs`)**: Full schema declarations and parameter extraction for
   all 10 MCP tools.
7. **End-to-End Tests (`mcp_server_inspector_tests.rs`)**: Integration testing via
   official `@modelcontextprotocol/inspector` CLI.
8. **Documentation & Workspace Integration**: Comprehensive documentation across
   `build-infra/README.md`, root `README.md`, and `build-infra/AGENTS.md`.

## Implementation plan

### Phase 1: Core Library Architecture & Type System

- [x] Create `build-infra/src/rust_analyzer_mcp_server/mod.rs` with architectural rustdocs
      and module exports.
- [x] Create `build-infra/src/rust_analyzer_mcp_server/constants.rs` containing namespaced
      protocol constants, timeouts, and debug tracing flags.
- [x] Create `build-infra/src/rust_analyzer_mcp_server/types.rs` defining MCP JSON-RPC
      schemas, tool structures, and `McpServerError`.
- [x] Create `build-infra/src/rust_analyzer_mcp_server/cli_arg.rs` with `clap::Parser`
      support for workspace paths, log levels, and log file destinations.

### Phase 2: Synchronous LSP Client & Subprocess Communication

- [x] Refactor `build-infra/src/rust_analyzer_mcp_server/lsp_client.rs` to manage
      `rust-analyzer` child process over standard I/O pipes using `std::process`.
- [x] Implement synchronous LSP JSON-RPC message framing with `Content-Length` headers in
      `lsp_client.rs`.
- [x] Implement synchronous request-response tracking using monotonically increasing
      request IDs and `std::sync::mpsc::sync_channel(1)`.
- [x] Implement document synchronization (`textDocument/didOpen`, `textDocument/didSave`)
      and background published diagnostic caching.

### Phase 3: Synchronous Server Event Loop & Tool Dispatch

- [x] Refactor `build-infra/src/rust_analyzer_mcp_server/tools.rs` to synchronous handlers
      for all 10 MCP tools:
    - `rust_analyzer_hover`
    - `rust_analyzer_definition`
    - `rust_analyzer_references`
    - `rust_analyzer_symbols`
    - `rust_analyzer_completion`
    - `rust_analyzer_format`
    - `rust_analyzer_code_actions`
    - `rust_analyzer_diagnostics`
    - `rust_analyzer_workspace_diagnostics`
    - `rust_analyzer_set_workspace`
- [x] Refactor `build-infra/src/rust_analyzer_mcp_server/server.rs` to synchronous
      standard I/O message loop and JSON-RPC dispatch.
- [x] Refactor `build-infra/src/bin/rust-analyzer-mcp-server.rs` into a synchronous entry
      point without Tokio runtime.
- [x] Re-export `rust_analyzer_mcp_server` module in `build-infra/src/lib.rs`.

### Phase 4: Integration Testing & Tooling Verification

- [x] Update `build-infra/src/rust_analyzer_mcp_server/mcp_server_inspector_tests.rs` with
      synchronous integration tests.
- [x] Verify tool listing and AST inspection responses with live `rust-analyzer` instances
      via `@modelcontextprotocol/inspector` CLI.

### Phase 5: Crate Configuration & Documentation

- [x] Update `build-infra/Cargo.toml` dependencies and package metadata.
- [x] Update `build-infra/AGENTS.md` with project directory structure and reinstall
      instructions.
- [x] Update `build-infra/README.md` with features, installation guides, MCP client
      configuration examples, and supported tools reference.
- [x] Update workspace root `README.md` to document `rust-analyzer-mcp-server` under build
      infrastructure, `run.fish install-cargo-tools`, and `run.fish install-build-infra`.

### Phase 6: Mandatory Manual Review

- [x] **Mandatory manual review:** Verify every file for correct implementation.
    - [x] `.github/assets/star-history.svg`
    - [x] `.vscode/settings.json`
    - [x] `Cargo.lock`
    - [x] `README.md`
    - [x] `build-infra/AGENTS.md`
    - [x] `build-infra/Cargo.toml`
    - [x] `build-infra/README.md`
    - [x] `build-infra/src/lib.rs`
    - [x] `build-infra/src/bin/rust-analyzer-mcp-server.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/cli_arg.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/constants.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/lsp_client.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/mod.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/server.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/tools.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/types.rs`
    - [x] `build-infra/src/rust_analyzer_mcp_server/mcp_server_inspector_tests.rs`
