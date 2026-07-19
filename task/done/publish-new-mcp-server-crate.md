# Task: Shard `rust-analyzer-mcp-server` from `build-infra` into standalone repo and crate

Plan to create standalone `r3bl-org/r3bl-rust-analyzer-mcp-server` repo & remove it from
`ROC` mono repo. This will allow publishing `1.0.0` to crates.io today without blocking on
ROC's large `v0.8.0` feature branch, and the MCP server is a standalone, domain-agnostic
developer utility for the entire global Rust + AI ecosystem.

## Overview

Extract the Model Context Protocol (MCP) server for `rust-analyzer` from the
`r3bl-open-core` (ROC) monorepo into its own dedicated, standalone GitHub repository named
`r3bl-org/r3bl-rust-analyzer-mcp-server` (published to [crates.io](https://crates.io) as
`r3bl-rust-analyzer-mcp-server`).

### Rationale & Domain Decoupling

1. **Domain Decoupling:** `r3bl-open-core` is focused on terminal user interfaces,
   parsers, and terminal emulation engines. `rust-analyzer-mcp-server` is a 100%
   universal, domain-agnostic developer utility for the entire global Rust + AI ecosystem.
2. **Immediate Independent Release:** Extracting it into its own repository allows
   publishing `1.0.0` directly to crates.io today without blocking on ROC's large `v0.8.0`
   feature branch.
3. **R3BL Ecosystem Integration:** The standalone crate depends on `r3bl_tui` (v0.7.7 on
   crates.io) for idiomatic `ok!` macro returns and R3BL's structured logging framework
   (`r3bl_tui::log`).
4. **Focused Repository & Community:** Dedicated GitHub repository
   (`https://github.com/r3bl-org/r3bl-rust-analyzer-mcp-server`) with its own issue
   tracker, CI workflows, and setup documentation for AI/LLM coding agents (Claude,
   Antigravity, Cursor, etc.).

### Standalone Crate Specifications & crates.io Metadata

```toml
[package]
name = "r3bl-rust-analyzer-mcp-server"
version = "1.0.0"
edition = "2024"
readme = "README.md"
homepage = "https://r3bl.com"
license = "Apache-2.0"
repository = "https://github.com/r3bl-org/r3bl-rust-analyzer-mcp-server"
authors = ["Nazmul Idris <idris@developerlife.com>"]
documentation = "https://docs.rs/r3bl-rust-analyzer-mcp-server"
description = """
Model Context Protocol (MCP) server for rust-analyzer. Built with Rust \
standard library threads to provide fast AST navigation, hover, definitions, \
code actions, and compiler diagnostics for AI/LLM coding agents \
(Claude, Antigravity, Cursor, etc.).\
"""
keywords = ["mcp", "rust-analyzer", "ai", "llm", "lsp"]
categories = ["development-tools", "command-line-utilities", "compilers"]

[[bin]]
name = "rust-analyzer-mcp-server"
path = "src/main.rs"

[lib]
name = "r3bl_rust_analyzer_mcp_server"
path = "src/lib.rs"

[dependencies]
r3bl_tui = "0.7.7"
clap = { version = "4.5", features = ["derive"] }
miette = { version = "7.6", features = ["fancy"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
tracing = "0.1"
which = "6.0"
```

- **Target Repository:** `r3bl-org/r3bl-rust-analyzer-mcp-server`
  (`/home/nazmul/github/r3bl-rust-analyzer-mcp-server`)
- **Key Architectural Features:**
    - **No async runtime overhead:** Built with standard library threads and channels
      (`std::process`, `std::thread`, `std::sync::mpsc`) for instant startup and low
      memory usage.
    - **3-thread design preventing deadlocks:** Dedicated main event loop, stdout response
      parser, and background stderr drainer so OS pipe buffers never fill up and hang.
    - **10 semantic tools:** Real compiler-backed operations for hover, definitions,
      references, document symbols, completions, formatting, code actions, diagnostics
      with summary report, workspace diagnostics, and dynamic workspace root switching.
    - **R3BL Primitives & Logging:** Uses `r3bl_tui` for `ok!` and structured file
      tracing.
- **Development & Testing Prerequisites:**
    - `rust-analyzer` in `PATH` (for runtime LSP execution).
    - Node.js & `npx` (for running `@modelcontextprotocol/inspector` integration tests).

## Implementation plan

### Phase 1: Package Standalone Repository Structure

- [x] Export the standalone crate structure to target repo directory
      (`/home/nazmul/github/r3bl-rust-analyzer-mcp-server`):
    - [x] `Cargo.toml` (package `r3bl-rust-analyzer-mcp-server`, binary
          `rust-analyzer-mcp-server`, dependency `r3bl_tui = "0.7.7"`)
    - [x] `README.md` containing:
        - **Why a dedicated bridge is needed:** Details on handling process exit, buffer
          management, and avoiding incomplete responses.
        - **Specific technical highlights:**
            - **Deadlock-Free 3-Thread Architecture:** Main event loop, stdout reader, and
              background stderr drainer preventing 64 KB OS kernel buffer deadlocks.
            - **No Tokio overhead:** Pure standard library threading (`std::process`,
              `std::thread`, `std::sync::mpsc::sync_channel(1)`), near-instantaneous
              startup (<2ms), and low memory usage.
            - **ADT Type Safety Mandate (Making Illegal States Unrepresentable):**
                - _Stream Framing & State Machine:_ `IncomingLspMessage` and
                  `HandshakeStatus` prevent stream corruption and enforce valid handshake
                  transitions during `Content-Length` framing and diagnostic collection.
                - _Spec Compliance & Numeric Safety:_ `McpResponse`, `McpServerError`, and
                  provenance-linked type aliases (`LspLineNumber`, `LspCharPosition`,
                  `JsonRpcRequestId`) eliminate coordinate errors with zero unchecked raw
                  numeric casts.
            - **190+ Automated Tests:** Verified against live compiler instances and the
              official `@modelcontextprotocol/inspector` CLI (100% real compiler-backed
              tools).
        - **Architecture:** Description of the 3-thread design for managing stdin, stdout,
          and stderr pipes with ASCII diagram.
        - **Available tools:** Reference for the 10 code intelligence tools (hover,
          definition, references, symbols, completions, formatting, code actions,
          diagnostics, workspace diagnostics, and workspace switching).
        - **Configuration:** Setup examples for Claude Code, Antigravity, and standard MCP
          clients.
        - **Prerequisites:** `rust-analyzer` on the PATH and Node.js with `npx` for
          integration tests.
    - [x] `CHANGELOG.md` (initial v1.0.0 release log documenting full 10-tool suite and
          architecture)
    - [x] `LICENSE` (Apache-2.0)
    - [x] `src/lib.rs` (flat public API)
    - [x] `src/main.rs` (binary entry point)
    - [x] `src/cli_arg.rs` (uses `r3bl_tui::log` for structured tracing setup)
    - [x] `src/constants.rs`
    - [x] `src/lsp_client.rs` (uses `r3bl_tui::ok!`)
    - [x] `src/server.rs` (uses `r3bl_tui::ok!`)
    - [x] `src/tools.rs`
    - [x] `src/types.rs`
    - [x] `src/mcp_conformance_tests.rs`
- [x] Verify standalone package compiles, tests, and passes clippy.

### Phase 2: Publish Standalone Repository to GitHub & crates.io

- [x] Initialize Git repository in `/home/nazmul/github/r3bl-rust-analyzer-mcp-server`:
    - [x] `git init` and create `.gitignore` (ignoring `/target/`, `.DS_Store`, etc.).
    - [x] Create initial commit with all files.
    - [x] Create remote GitHub repository `r3bl-org/r3bl-rust-analyzer-mcp-server` via
          `gh repo create r3bl-org/r3bl-rust-analyzer-mcp-server --public --source=. --remote=origin --push`.
- [x] Run dry-run verification:
    - [x] `cargo publish --dry-run` to verify package archive, license, readme, and
          metadata with zero warnings.
- [x] Tag and create GitHub Release:
    - [x] Create git tag `v1.0.0`: `git tag -a v1.0.0 -m "Release v1.0.0"`
    - [x] Push tags to GitHub: `git push origin main --tags`
    - [x] Draft release notes (`/tmp/RELEASE_NOTES_v1.0.0.md`) incorporating:
        - **Motivation:** Describes the requirement for a stable connection to
          rust-analyzer that manages process output and compiler diagnostics correctly.
        - **Key technical highlights:**
            - **Deadlock-Free 3-Thread Architecture:** Main event loop, stdout reader, and
              background stderr drainer preventing 64 KB OS kernel buffer deadlocks.
            - **No Tokio overhead:** Pure standard library threading (`std::process`,
              `std::thread`, `std::sync::mpsc::sync_channel(1)`), near-instantaneous
              startup (<2ms), and low memory usage.
            - **ADT Type Safety Mandate (Making Illegal States Unrepresentable):**
                - _Stream Framing & State Machine:_ `IncomingLspMessage` and
                  `HandshakeStatus` prevent stream corruption and enforce valid handshake
                  transitions during `Content-Length` framing and diagnostic collection.
                - _Spec Compliance & Numeric Safety:_ `McpResponse`, `McpServerError`, and
                  provenance-linked type aliases (`LspLineNumber`, `LspCharPosition`,
                  `JsonRpcRequestId`) eliminate coordinate errors with zero unchecked raw
                  numeric casts.
            - **190+ Automated Tests:** Verified against live compiler instances and the
              official `@modelcontextprotocol/inspector` CLI (100% real compiler-backed
              tools).
        - **Supported tools:** Summary of the 10 included code intelligence tools.
        - **Setup instructions:** Configuration steps for AI coding assistants.
    - [x] Create GitHub Release via:
          `gh release create v1.0.0 --title "v1.0.0" --notes-file /tmp/RELEASE_NOTES_v1.0.0.md`
- [x] Publish to crates.io:
    - [x] Run `cargo publish` from `/home/nazmul/github/r3bl-rust-analyzer-mcp-server`.
    - [x] Verify published package on
          [crates.io/crates/r3bl-rust-analyzer-mcp-server](https://crates.io/crates/r3bl-rust-analyzer-mcp-server).
- [x] Verification of published crate:
    - [x] Test installation via `cargo install r3bl-rust-analyzer-mcp-server --force`.
    - [x] Verify installed binary `~/.cargo/bin/rust-analyzer-mcp-server --version` or via
          inspector.

### Phase 3: Announce in This Week in Rust (TWiR)

- [x] Navigate to local clone `/home/nazmul/github/this-week-in-rust`.
- [x] Pull latest changes from upstream `this-week-in-rust` main branch.
- [x] Create a feature branch `add-r3bl-rust-analyzer-mcp-server`.
- [x] Draft submission blurb for the upcoming TWiR issue under "Updates from Rust
      Community" / "Project/Tooling Updates":
    - Entry:
      `* [r3bl-rust-analyzer-mcp-server 1.0.0: Fast, simple MCP server for rust-analyzer](https://github.com/r3bl-org/r3bl-rust-analyzer-mcp-server/releases/tag/v1.0.0)`
- [x] Commit, push branch, and submit PR to `rust-lang/this-week-in-rust` (PR #8614).

### Phase 4: Clean Up `build-infra` in ROC Monorepo & Update Setup Scripts

- [x] Remove `build-infra/src/rust_analyzer_mcp_server/` directory.
- [x] Remove `build-infra/src/bin/rust-analyzer-mcp-server.rs`.
- [x] Update `build-infra/Cargo.toml` to remove `[[bin]] rust-analyzer-mcp-server` and
      unused MCP dependencies (`which`).
- [x] Update `build-infra/src/lib.rs` to keep only `cargo_rustdoc_fmt` and shared
      workspace utilities.
- [x] Update `build-infra/README.md` and `build-infra/AGENTS.md` (remove MCP server
      sections, keep `cargo-rustdoc-fmt`).
- [x] Update `run.fish`:
    - [x] Add `"r3bl-rust-analyzer-mcp-server"` to `set cargo_tools` in
          `install-cargo-tools` function so `run.fish install-cargo-tools` automatically
          installs it.
    - [x] Update `install-build-infra` function description and output to reflect
          `cargo-rustdoc-fmt`.
- [x] Update workspace root `README.md` to reference `r3bl-rust-analyzer-mcp-server` and
      standalone repository link.
- [x] Update `.agents/mcp_config.json` and `.gemini/settings.json` (if needed) to ensure
      they reference the installed `rust-analyzer-mcp-server` binary.

### Phase 5: Full Quality Verification in ROC

- [x] Run `./check.fish --fmt` on ROC.
- [x] Run `./check.fish --quick-doc` on ROC.
- [x] Run `./check.fish --clippy` on ROC.
- [x] Run `./check.fish --test` on ROC.

### Phase 6: Mandatory Manual Review

- [x] **Mandatory manual review:** Verify every modified file in ROC.
    - [x] `README.md`
    - [x] `run.fish`
    - [x] `build-infra/Cargo.toml`
    - [x] `build-infra/README.md`
    - [x] `build-infra/AGENTS.md`
    - [x] `build-infra/src/lib.rs`
