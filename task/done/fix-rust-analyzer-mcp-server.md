# Task: Replace Broken rust-refactor MCP Server with rust-analyzer-mcp

## Overview

The previous `rust-refactor` MCP server (vendored from
[`dexwritescode/rust-mcp`](https://github.com/dexwritescode/rust-mcp) in
`vendored-crates/rust-mcp`, installed as `rustmcp`) had critical transport defects:

1. **100% CPU infinite busy loop on EOF:** When `rust-analyzer` exited or closed stdout,
   `read_response` spun indefinitely on `read_line` returning 0 bytes, pegging CPU cores
   and leaving zombie `<defunct>` processes.
2. **Buffer loss:** Recreating `BufReader` on every request dropped unconsumed stream
   bytes.
3. **Mock implementations:** 14 out of 19 tools were non-functional placeholder string
   generators rather than true compiler-backed refactoring actions.

This task removes the broken vendored crate, adopts `rust-analyzer-mcp` (crates.io /
`zeenix/rust-analyzer-mcp`), renames the MCP server identifier to `rust-analyzer`, and
updates developer setup scripts (`run.fish`, `bootstrap.sh`) and workspace MCP
configurations (`.gemini/settings.json`, `.agents/mcp_config.json`).

### Tool Selection & Maintenance Justification: `zeenix/rust-analyzer-mcp`

`zeenix/rust-analyzer-mcp` (v0.2.0 released on crates.io) is selected for the following
reasons:

1. **Pure LSP Bridge Architecture:** Unlike complex applications, `rust-analyzer-mcp` is a
   thin, focused native Rust bridge between the Model Context Protocol and the local
   `rust-analyzer` binary. Because LSP (Language Server Protocol) and MCP stdio transports
   are stable, the bridge does not require frequent code churn once the core protocol
   mapping is in place.
2. **Author & Implementation Quality:** Authored by Zeeshan Ali (`zeenix`, prominent Rust
   developer and author of `zbus`), it utilizes proper Tokio async stream handling rather
   than naive line loops, preventing the EOF 100% CPU spin and stream desynchronization
   defects present in `dexwritescode/rust-mcp`.
3. **Available Tool Surface:** It implements real LSP operations delegated directly to
   `rust-analyzer`:
    - `rust_analyzer_definition` (Go to definition)
    - `rust_analyzer_references` (Find all references)
    - `rust_analyzer_hover` (Type information and doc comments)
    - `rust_analyzer_symbols` (Document symbols)
    - `rust_analyzer_code_actions` (Compiler quick-fixes and refactorings)
    - `rust_analyzer_format` (Formatting text edits)
    - `rust_analyzer_diagnostics` (Compiler diagnostics)
    - `rust_analyzer_completion` (Code completions)

### Upstream Bug Report & Reproduction Steps (zeenix/rust-analyzer-mcp v0.2.0)

#### 1. Problem Description

Per JSON-RPC 2.0 and MCP specification, notifications (such as
`notifications/initialized`) do not contain an `id` and MUST NOT produce a response or
error on standard output.

In `zeenix/rust-analyzer-mcp` v0.2.0, when the client sends `notifications/initialized`
following `initialize`, the server returns:

```json
{
    "jsonrpc": "2.0",
    "error": {
        "code": -32601,
        "message": "Method not found: notifications/initialized",
        "data": null
    }
}
```

This desynchronizes MCP clients expecting the response to their next request (e.g.
`tools/list` with ID 2).

#### 2. Exact Reproduction Steps in Antigravity CLI (`agy`)

1. Install `zeenix/rust-analyzer-mcp` v0.2.0 from crates.io:
    ```bash
    cargo install rust-analyzer-mcp --force
    ```
2. Configure MCP server in `.gemini/settings.json`:
    ```json
    {
        "mcpServers": {
            "rust-analyzer": {
                "command": "rust-analyzer-mcp",
                "args": []
            }
        }
    }
    ```
3. Open an `agy` session and run the `/mcp` slash command.
4. **Observed Error:**
    ```text
    MCP Servers

    Plugins (~/.gemini/config/plugins)
    >  ✗ rust-analyzer  error: failed to get tools: calling "tools/list": invalid request
        [Restart]   Disable
    ```

#### 3. Root Cause

In `src/main.rs:handle_request`, all unhandled methods (including notifications where
`request.id` is `None`) hit the wildcard arm
`_ => MCPResponse::Error { code: -32601, ... }` and are written to standard output.

#### 4. The Fix

Silently process notifications (messages where `request.id.is_none()` or
`request.method.starts_with("notifications/")`) in `handle_request`, returning `None`
instead of writing an error response to standard output.

## Implementation plan

### Phase 1: Process Cleanup and Binary Installation

- [x] Terminate runaway `rustmcp` processes pinning CPU cores.
- [x] Uninstall legacy `rustmcp` binary: `cargo uninstall rustmcp`.
- [x] Install `rust-analyzer-mcp` from crates.io: `cargo install rust-analyzer-mcp`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.

### Phase 2: Remove Vendored Crate

- [x] Remove `vendored-crates/rust-mcp/` directory and `vendored-crates/` parent
      directory.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `vendored-crates/`

### Phase 3: Update MCP Configuration Files

- [x] Update `.gemini/settings.json` to configure `rust-analyzer` using
      `rust-analyzer-mcp`.
- [x] Update `.agents/mcp_config.json` to configure `rust-analyzer` using
      `rust-analyzer-mcp`.
- [x] Update global configuration `~/.gemini/config/mcp_config.json` to configure
      `rust-analyzer`.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `.gemini/settings.json`
    - [x] `.agents/mcp_config.json`

### Phase 4: Update Environment Setup and Maintenance Scripts

- [x] Update `run.fish` `install-cargo-tools` to ensure `rust-analyzer-mcp` is installed
      with `--force` (using `cargo install rust-analyzer-mcp --force`).
- [x] Remove legacy `rustmcp` install commands and references from `run.fish`.
- [x] Remove deprecated `gemini mcp add` invocation and `gemini` checks from `run.fish`.
- [x] Remove `cargo install --path vendored-crates/rust-mcp --force` from
      `update-cargo-tools` in `run.fish`.
- [x] Update `bootstrap.sh` comments and tool lists if necessary.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `run.fish`
    - [ ] `bootstrap.sh`

### Phase 5: Update Documentation & Agent Instructions

- [x] Update `AGENTS.md` under "A. Semantic Rust Tools (AST-Aware MCP)" to reference
      `rust-analyzer` and `rust-analyzer-mcp`.
- [x] Update `README.md` to replace deprecated `Gemini CLI` / `gemini` references with
      `Antigravity CLI` (`agy`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `AGENTS.md`
    - [ ] `README.md`

### Phase 6: Verification and Validation

- [x] Validate `rust-analyzer-mcp` via automated JSON-RPC test (stdio handshake, tool
      listing, symbol/definition resolution).
- [x] Run `./check.fish --check` to ensure full repository compilation and integrity.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/fix-rust-analyzer-mcp-server.md`

### Phase 7: Vendor rust-analyzer-mcp with Upstream-Ready MCP Notification Fix

- [x] Create `vendored-crates/rust-analyzer-mcp/` containing `Cargo.toml` (isolated
      workspace) and `src/main.rs`.
- [x] Apply notification handling fix in `src/main.rs` (silently process
      `notifications/initialized` and messages where `id.is_none()` without emitting
      JSON-RPC error responses to stdout).
- [x] Install patched binary:
      `cargo install --path vendored-crates/rust-analyzer-mcp --force`.
- [x] Update `run.fish` (`install-cargo-tools` and `update-cargo-tools`) to build and
      install from `vendored-crates/rust-analyzer-mcp`.
- [x] Verify `agy-cli` handshake and tool discovery (`✓ rust-analyzer`).
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `vendored-crates/rust-analyzer-mcp/Cargo.toml`
    - [x] `vendored-crates/rust-analyzer-mcp/src/main.rs`
    - [x] `run.fish`

### Phase 8: Create and Submit Upstream Pull Request to zeenix/rust-analyzer-mcp

- [x] Fork and clone `zeenix/rust-analyzer-mcp` to temporary workspace
      `/tmp/rust-analyzer-mcp-upstream`.
- [x] Create branch `fix-mcp-notifications`.
- [x] Apply notification handling fix and add regression test in
      `tests/unit/protocol/request_tests.rs`.
- [x] Run `cargo test` and `cargo clippy --all-targets` on upstream repository.
- [x] Commit with gitmoji convention
      (`:bug: fix(protocol): silently handle MCP notifications without emitting error responses`)
      and push to fork `nazmulidris/rust-analyzer-mcp`.
- [x] Submit Pull Request via `gh pr create` to `zeenix/rust-analyzer-mcp` with the
      documented problem statement, reproduction steps, and root cause analysis:
      [PR #22](https://github.com/zeenix/rust-analyzer-mcp/pull/22).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/fix-rust-analyzer-mcp-server.md`
