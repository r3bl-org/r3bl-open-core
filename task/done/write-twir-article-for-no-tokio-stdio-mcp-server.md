<!-- cspell:words rmcp ciresnave zeenix dexwritescode rust-analyzer-mcp -->

# Plan: Technical Article on Synchronous MCP Server Architecture

## Objective

Write and publish an in-depth technical article detailing why `tokio` was deliberately
omitted in `rust-analyzer-mcp-server` in favor of a clean, synchronous, thread-and-channel
pipeline over stdio. Submit the article directly to _This Week in Rust_ (TWiR) via GitHub
Pull Request under the **Observations/Thoughts** or **Walkthroughs** section.

Title of this article is "To async or not to async"

---

## Target Narrative & Core Themes

- **Pragmatism over Ideology (Fit for Purpose):** Clarify that this is not an anti-async
  dogma. I built `r3bl_tui`, which is fully async from the ground up (including custom
  async readline primitives and event loops where async excels). Emphasize using the right
  tool for the job: fit for purpose over convention or ideological defaults.
- **Challenging "Async-by-Default":** Why stdio over Tokio delegates to blocking OS
  threads anyway (`spawn_blocking`).
- **Subprocess & Pipe Mechanics:** Managing bidirectional stdio JSON-RPC streams between
  LLM host and `rust-analyzer` child process.
- **Deterministic Lifetimes & Teardown:** Clean EOF propagation on stdio disconnect
  without dangling futures or async cancellation leaks.
- **Systems Trade-offs:** Fast cold starts, tiny binary footprint, trivial stack traces,
  and zero runtime thread-pool overhead.

---

## Comparative Analysis & Counter-Examples (The Async Baseline)

### The Root Cause of "Async-by-Default" & The Primary Hypothesis

When Anthropic introduced the Model Context Protocol, the official Rust SDK released was
`rmcp` (`modelcontextprotocol/rust-sdk`). Because `rmcp` was built fundamentally on the
Tokio runtime (`#[tokio::main]`, `tokio::io::{stdin, stdout}`, and async service traits),
it established a pervasive "async-by-default" convention across the entire Rust MCP
ecosystem. Virtually every developer writing a Rust MCP server or `rust-analyzer` bridge
reached for Tokio reflexively, following the official SDK pattern without questioning if
an async runtime is the right tool for standard input/output streams.

**The Primary Hypothesis to Debunk in this Article:** The assumption that a stdio-based
MCP server in Rust requires, or even benefits from, an asynchronous runtime like Tokio is
flawed. In reality, wrapping synchronous stdio streams and subprocess pipes inside an
async runtime introduces accidental complexity: Tokio cannot poll stdio via OS
epoll/kqueue and delegates to blocking thread-pools anyway, while adding runtime overhead,
startup latency, cancellation hazards, and complex debugging state machines. A lean,
synchronous pipeline using standard library threads (`std::thread`) and channels
(`std::sync::mpsc`) provides superior performance, deterministic teardown, and simpler
code.

### Current Open Source Tokio / Async MCP Ecosystem

- **[rmcp (Official Rust SDK)](https://github.com/modelcontextprotocol/rust-sdk)**
  (`crates.io/crates/rmcp`): The official MCP Rust SDK built fundamentally on Tokio
  (`#[tokio::main]`, `tokio::io::{stdin, stdout}`). While not a `rust-analyzer` server
  itself, it represents the upstream root cause of the async default in Rust MCP
  development.
- **[rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk)**
  (`crates.io/crates/rust-mcp-sdk`): Community async MCP SDK built on Tokio.
- **[zeenix/rust-analyzer-mcp](https://github.com/zeenix/rust-analyzer-mcp)**
  (`crates.io/crates/rust-analyzer-mcp`): A `rust-analyzer` MCP bridge written in Rust
  using the Tokio async runtime.
- **[ciresnave/rust-analyzer-mcp-server](https://github.com/ciresnave/rust-analyzer-mcp-server)**:
  Another `rust-analyzer` MCP server variant using async runtime integration.
- **[dexwritescode/rust-mcp](https://github.com/dexwritescode/rust-mcp)**: An async MCP
  server providing a wide suite of Rust developer tools on Tokio.
- **[lsp-mcp](https://crates.io/crates/lsp-mcp)**: Generic async Tokio-based LSP-to-MCP
  gateway on crates.io.

### Key Architectural Contrasts to Highlight in the Article

1. **Stdio I/O Mechanics:** Tokio's `tokio::io::stdin()` cannot be polled via
   `epoll`/`kqueue` on Unix, so it delegates to `spawn_blocking` threads internally.
   Wrapping stdio in an async runtime adds event loop machinery without non-blocking
   benefits.
2. **Subprocess Pipes:** `tokio::process::Command` with async streams vs. standard OS
   threads and blocking `std::io::BufReader`/`std::io::BufWriter` pumps.
3. **Shutdown & EOF Lifecycle:** How synchronous EOF unwinds cleanly and deterministically
   when the host disconnects, compared to cancellation token propagation in async task
   trees.
4. **Binary Footprint & Cold Starts:** Eliminating Tokio runtime initialization and
   dependencies, minimizing latency when spawned per-session by AI agents (e.g. Claude,
   Antigravity, Cursor).

---

## Execution Checklist

### Phase 1: Article Outline & Technical Framing

- [x] **Establish my perspective and philosophy:**
    - Contextualize my experience building `r3bl_tui` (fully async down to readline
      primitives).
    - Reframe discussion around pragmatic systems engineering vs. dogmatic defaults.
- [x] **Define the problem space:**
    - MCP protocol over `stdin`/`stdout`.
    - Wrapping `rust-analyzer` LSP client via subprocess pipes.
    - The reflex to use `#[tokio::main]` vs. the reality of stdio I/O.
- [x] **Frame the primary hypothesis and ecosystem root cause:**
    - Trace the "async-by-default" convention to the official `rmcp` SDK and early
      ecosystem patterns.
    - Explicitly state the hypothesis being debunked: that async runtimes are beneficial
      for stdio-based MCP servers.
- [x] **Survey the existing landscape of async MCP servers:**
    - Reference the bucket of existing Tokio-based open source MCP servers and SDKs
      (`zeenix/rust-analyzer-mcp`, `ciresnave/rust-analyzer-mcp-server`,
      `dexwritescode/rust-mcp`, `lsp-mcp`).
    - Frame them as the standard "async-by-default" status quo.
    - Analyze where Tokio adds value vs. where it adds accidental complexity for stdio.
- [x] **Isolate the thread & channel architecture:**
    - Thread 1: Blocking `stdin` reader parsing MCP JSON-RPC messages.
    - Thread 2: Subprocess `stdout` reader parsing LSP responses and routing back.
    - Thread 3: Subprocess `stderr` drainer preventing 64 KB pipe deadlock.
    - Synchronous coordination (single-use `sync_channel(1)` vs. async locks).
- [x] **Document systems benefits:**
    - Startup latency comparison (cold start when spawned by LLM agents).
    - Debuggability (clear OS-thread backtraces vs. async state-machine polls).
    - Dependency reduction and compile times.

### Phase 2: Code Artifacts & Diagrams

- [x] Extract a minimal, readable code snippet showing the synchronous stdio pump loop.
- [x] Extract a snippet showing clean EOF / graceful shutdown handling when the host
      closes `stdin`.
- [x] Create an ASCII architecture diagram showing the 3-process and 3-thread layout.
- [x] Update crate documentation in `rust-analyzer-mcp-server/README.md`.
- [x] Draft the complete article in
      `/home/nazmul/github/developerlife.com/_posts/2026-08-22-to-async-or-not-to-async-rust-mcp-server.md`.

### Phase 3: Mandatory Manual Review

- [x] **Mandatory manual review**
    - [x] `rust-analyzer-mcp-server/README.md`
    - [x] `rust-analyzer-mcp-server/lib.rs`
    - [x] `/home/nazmul/github/developerlife.com/_posts/2026-08-22-to-async-or-not-to-async-rust-mcp-server.md`
