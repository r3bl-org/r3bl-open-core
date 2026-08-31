# r3bl-rust-analyzer-mcp-server

Model Context Protocol (MCP) server for [`rust-analyzer`][ra]. Built with Rust standard
library threads to provide fast AST navigation, hover, definitions, code actions, and
compiler diagnostics for AI / LLM coding agents (Claude, Antigravity, Cursor, etc.).

## Why Yet Another rust-analyzer MCP Server?

At R3BL, we needed a rock-solid [`rust-analyzer`][ra] MCP server for daily
pair-programming with AI / LLM coding agents. Many existing crates in the ecosystem often
suffer from 100% CPU infinite loops on process exit, kernel pipe buffer deadlocks when
reading large diagnostics, async lock contention, incomplete mock implementations, and
needless complexity from async runtimes like Tokio.

We built this native bridge from the ground up with these design goals in mind:

- **Deadlock-Free 3-Thread Architecture:** Main event loop, `stdout` reader, and
  background `stderr` drainer preventing 64 KiB OS kernel buffer deadlocks.
- **Zero Tokio Overhead:** Pure standard library threading (`std::process`, `std::thread`,
  `std::sync::mpsc::sync_channel(1)`), sub-millisecond cold starts (<2ms), and low memory
  usage with zero lock contention.
- **ADT Type Safety Mandate:** Stream framing and state machines that make illegal states
  unrepresentable with zero unchecked raw numeric casts.
- **Comprehensive Test Suite:** Unit and integration tests verified against live
  [`rust-analyzer`][ra] instances and the official `@modelcontextprotocol/inspector` CLI.

For an in-depth exploration of the synchronous 3-thread architecture and design
trade-offs, see [To Async or Not to Async: Building a Rust MCP Server for
rust-analyzer][article].

## Installation

Install directly via `cargo`:

```bash
cargo install r3bl-rust-analyzer-mcp-server
```

This installs the binary `rust-analyzer-mcp-server` into `~/.cargo/bin`.

## Prerequisites

Ensure [`rust-analyzer`][ra] is installed and accessible in your shell:

```bash
rustup component add rust-analyzer
```

Node.js and `npx` are only required if running integration tests with
`@modelcontextprotocol/inspector`.

## Configuration Examples

This server implements standard Model Context Protocol stdio transport and has been
verified for live pair-programming workflows with **Google Antigravity (`agy` CLI &
IDE)**, **OpenCode**, and **Claude Code**.

### Google Antigravity (CLI & IDE)

Add to project scope (`.agents/mcp_config.json`) or global scope
(`~/.gemini/config/mcp_config.json`):

```json
{
    "mcpServers": {
        "rust-analyzer": {
            "command": "rust-analyzer-mcp-server"
        }
    }
}
```

### OpenCode

Add to project config (`opencode.json`) or global config
(`~/.config/opencode/opencode.json`):

```json
{
    "mcpServers": {
        "rust-analyzer": {
            "command": "rust-analyzer-mcp-server"
        }
    }
}
```

### Claude Code

Add directly using the Claude CLI:

```bash
claude mcp add rust-analyzer -- rust-analyzer-mcp-server
```

## CLI Options

```text
Usage: rust-analyzer-mcp-server [OPTIONS]

Options:
  -w, --workspace <PATH>   Root directory of the Rust workspace [default: current directory]
  -l, --log-level <LEVEL>  Logging verbosity (off, error, warn, info, debug, trace) [default: off]
  -f, --log-file <PATH>    File path for structured log output [default: stderr]
  -h, --help               Print help
  -V, --version            Print version
```

## Available Tools (10)

- **`rust_analyzer_hover`** (`file_path`, `line`, `character`): Hover documentation and
  type signatures.
- **`rust_analyzer_definition`** (`file_path`, `line`, `character`): Go to symbol
  definition.
- **`rust_analyzer_references`** (`file_path`, `line`, `character`): Find all references
  across workspace.
- **`rust_analyzer_symbols`** (`file_path`): Document symbols and AST hierarchy.
- **`rust_analyzer_completion`** (`file_path`, `line`, `character`): Context-aware code
  completions.
- **`rust_analyzer_format`** (`file_path`): Format file using `rustfmt`.
- **`rust_analyzer_code_actions`** (`file_path`, `line`, `character`, `end_line`,
  `end_character`): Code actions, quick-fixes, refactors.
- **`rust_analyzer_diagnostics`** (`file_path`): Compiler diagnostics for a file (LSP 3.17
  pull & push fallback).
- **`rust_analyzer_workspace_diagnostics`**: Workspace compiler diagnostics (LSP 3.17
  `workspace/diagnostic`).
- **`rust_analyzer_set_workspace`** (`workspace_path`): Dynamically switch workspace root.

**Note on Protocol & Indexing:** All tools conform to the [Language Server Protocol 3.17
Specification][lsp-spec]. All line numbers (`line`, `end_line`) and character positions
(`character`, `end_character`) in tool arguments use **0-based indexing**, conforming to
the [LSP Position Specification][lsp-pos].

## Running Tests

Run the test suite:

```bash
cargo test --all-targets
```

Run end-to-end integration tests using the MCP inspector CLI:

```bash
npx @modelcontextprotocol/inspector --cli rust-analyzer-mcp-server --method tools/list
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

<!-- prettier-ignore-start -->
[ra]: https://rust-analyzer.github.io/
[lsp-spec]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
[lsp-pos]: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#position
[article]: https://developerlife.com/2026/08/22/to-async-or-not-to-async-rust-mcp-server/
<!-- prettier-ignore-end -->
