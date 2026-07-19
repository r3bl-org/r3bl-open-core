# Claude Code & Antigravity Instructions for r3bl-rust-analyzer-mcp-server

This crate provides the `rust-analyzer-mcp-server` MCP binary for the R3BL workspace.
After making code changes, you **must** reinstall the binary for the changes to take effect in your AI agents / MCP clients.

## Important: Binary Installation Workflow

This crate provides a command-line MCP server binary installed to `~/.cargo/bin`:

- `rust-analyzer-mcp-server`: Model Context Protocol server bridge for `rust-analyzer`

### After Making Code Changes

**Always run this command to install the updated binary:**

```bash
cargo install --path . --force
```

Or from the workspace root:

```bash
cargo install --path rust-analyzer-mcp-server --force
# Or using the run script:
fish run.fish install-rust-analyzer-mcp-server
```

### Why This Matters

- The binaries in `~/.cargo/bin` are **separate files** from your source code
- Running `cargo build` or `cargo check` only compiles code in `target/`
- Without `cargo install`, the old binary in `~/.cargo/bin` will continue to be executed by MCP hosts
- Always reinstall the binary when testing or modifying MCP server behavior

### Testing Workflow

When working on changes to this crate, follow this workflow:

1. **Make code changes**
2. **Run tests to verify logic:**
   ```bash
   cargo test -p r3bl-rust-analyzer-mcp-server --all-targets
   ```
3. **Run clippy:**
   ```bash
   cargo clippy -p r3bl-rust-analyzer-mcp-server --all-targets
   ```
4. **Install the updated binary:**
   ```bash
   cargo install --path . --force
   ```
