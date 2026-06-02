[![CI](https://github.com/dexwritescode/rust-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/dexwritescode/rust-mcp/actions/workflows/ci.yml)

# Rust MCP Server

A comprehensive Model Context Protocol (MCP) server that provides rust-analyzer integration for LLM-assisted Rust development. This server enables AI tools like Claude to work with Rust code idiomatically through rust-analyzer's Language Server Protocol capabilities, avoiding string manipulation and providing intelligent code analysis and refactoring.

## Quick Start

1. **Build**: `cargo build --release`
2. **Configure** your MCP client to use `target/release/rustmcp`
3. **Use** through AI assistants with natural language prompts like "Generate a User struct with Debug and Clone derives"

## Features - Complete Tool Suite (19 Tools)

### Code Analysis (4 tools)
- `find_definition` - Navigate to symbol definitions
- `find_references` - Find all symbol uses  
- `get_diagnostics` - Get compiler errors/warnings with fixes
- `workspace_symbols` - Search project symbols

### Code Generation (4 tools)
- `generate_struct` - Create structs with derives and constructors
- `generate_enum` - Create enums with variants
- `generate_trait_impl` - Generate trait implementations with stubs
- `generate_tests` - Create unit or integration test templates

### Refactoring (5 tools)
- `rename_symbol` - Rename with scope awareness
- `extract_function` - Extract code into functions
- `inline_function` - Inline function calls
- `organize_imports` - Sort and organize use statements
- `format_code` - Apply rustfmt formatting

### Quality Assurance (2 tools)
- `apply_clippy_suggestions` - Apply clippy automatic fixes
- `validate_lifetimes` - Check lifetime and borrow checker issues

### Project Management (2 tools)
- `analyze_manifest` - Parse and analyze Cargo.toml
- `run_cargo_check` - Execute cargo check with error parsing

### Advanced Features (4 tools)
- `get_type_hierarchy` - Get type relationships for symbols
- `suggest_dependencies` - Recommend crates based on code patterns
- `create_module` - Create new Rust modules with visibility control
- `move_items` - Move code items between files

### Additional Advanced Tools
- `change_signature` - Modify function signatures safely

## Prerequisites

- Rust toolchain (1.70+)
- rust-analyzer installed (defaults to `~/.cargo/bin/rust-analyzer`)
- An MCP-compatible client (Claude, Roo, etc.)

## Installation

1. Clone this repository:
```bash
git clone <repository-url>
cd rust-mcp
```

2. Build the server:
```bash
cargo build --release
```

3. The server binary will be available at `target/release/rustmcp`

## Configuration

### Environment Variables

The server supports the following environment variables:

- `RUST_ANALYZER_PATH` - Path to rust-analyzer binary (default: `~/.cargo/bin/rust-analyzer`)

You can set this when running the server:
```bash
RUST_ANALYZER_PATH=/usr/local/bin/rust-analyzer ./target/release/rustmcp
```

Or set it in your MCP client configuration (see examples below).

### Claude Desktop

Add the following to your Claude Desktop MCP configuration file:

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "rust-analyzer": {
      "command": "/path/to/rust-mcp/target/release/rustmcp",
      "args": [],
      "env": {
        "RUST_ANALYZER_PATH": "/custom/path/to/rust-analyzer"
      }
    }
  }
}
```

If rust-analyzer is in the default location (`~/.cargo/bin/rust-analyzer`), you can omit the `env` section:

```json
{
  "mcpServers": {
    "rust-analyzer": {
      "command": "/path/to/rust-mcp/target/release/rustmcp",
      "args": []
    }
  }
}
```

### Roo Configuration

Add to your Roo configuration file (typically `~/.roo/config.json`):

```json
{
  "mcp_servers": [
    {
      "name": "rust-analyzer",
      "command": "/path/to/rust-mcp/target/release/rustmcp",
      "args": [],
      "env": {
        "RUST_ANALYZER_PATH": "/custom/path/to/rust-analyzer"
      }
    }
  ]
}
```

For default rust-analyzer location, you can use an empty env object:

```json
{
  "mcp_servers": [
    {
      "name": "rust-analyzer",
      "command": "/path/to/rust-mcp/target/release/rustmcp",
      "args": [],
      "env": {}
    }
  ]
}
```

### Other MCP Clients

For any MCP-compatible client, configure it to run:
```bash
/path/to/rust-mcp/target/release/rustmcp
```

The server uses stdio transport and will be ready to accept MCP protocol messages.

## Usage Examples

Once configured, you can use the tools through your AI assistant. Here are some example prompts:

### Code Analysis
```
"Find all references to the `Config` struct in this Rust project"
"Show me the definition of the `parse_args` function"
"Check for compiler errors in src/main.rs"
"Search for all symbols matching 'user' in the workspace"
```

### Code Generation
```
"Generate a struct called `User` with fields: name (String), age (u32), email (String), with Debug and Clone derives"
"Create an enum called `HttpStatus` with variants: Ok, NotFound, ServerError"
"Generate unit tests for the `calculate_total` function"
"Generate a Display trait implementation for the User struct"
```

### Refactoring
```
"Rename the variable `data` to `user_input` throughout the codebase"
"Extract this code block into a separate function called `validate_input`"
"Inline the `helper_function` call on line 42"
"Organize all import statements in src/lib.rs"
"Format all the code in src/lib.rs"
```

### Quality Assurance
```
"Run clippy and apply all automatic fixes to improve code quality"
"Check for any lifetime or borrow checker issues in src/auth.rs"
```

### Project Management
```
"Analyze the Cargo.toml file and show dependency information"
"Run cargo check and report any compilation errors"
```

### Advanced Features
```
"Show me the type hierarchy for the symbol at line 15, character 8 in src/main.rs"
"Suggest crate dependencies for HTTP client functionality in this workspace"
"Create a new public module called 'auth' in src/auth.rs"
"Move the User struct and validate_user function from src/main.rs to src/user.rs"
"Change the signature of the process_data function to accept a reference instead of ownership"
```

## Architecture

The server is built with a modular architecture:

- **`src/main.rs`** - Entry point and server initialization
- **`src/lib.rs`** - Module declarations
- **`src/server/`** - MCP server implementation
  - `handler.rs` - Tool handlers and MCP server logic using rmcp crate
  - `parameters.rs` - Parameter type definitions for all tools
- **`src/analyzer/`** - rust-analyzer LSP client integration
  - `client.rs` - LSP client implementation and protocol handling
- **`src/tools/`** - Modular tool implementations
  - `types.rs` - Tool dispatcher and definitions
  - `analysis.rs` - Code analysis tools (find_definition, find_references, etc.)
  - `generation.rs` - Code generation tools (generate_struct, generate_enum, etc.)
  - `refactoring.rs` - Refactoring tools (rename_symbol, extract_function, etc.)
  - `formatting.rs` - Code formatting tools
  - `quality.rs` - Quality assurance tools (clippy, lifetimes)
  - `cargo.rs` - Project management tools
  - `navigation.rs` - Navigation tools (workspace_symbols)
  - `advanced.rs` - Advanced features (type hierarchy, dependencies, modules)

## Development

### Running in Development
```bash
cargo run
```

### Testing Individual Tools
The server exposes all tools through the MCP protocol. For debugging, you can:

1. Run the server: `cargo run`
2. Send MCP messages via stdin (JSON-RPC format)
3. Check server logs and responses

### Adding New Tools

1. Create the tool implementation function in the appropriate `src/tools/*.rs` file
2. Add parameter struct to `src/server/parameters.rs`
3. Add the tool to the `execute_tool` match statement in `src/tools/types.rs`
4. Add tool definition to `get_tools()` function in `src/tools/types.rs`
5. Add the corresponding `#[tool]` method to `RustMcpServer` in `src/server/handler.rs`
6. Add analyzer client method to `src/analyzer/client.rs` if needed

## Troubleshooting

### rust-analyzer Not Found
Ensure rust-analyzer is installed and accessible. The server will look for rust-analyzer in the following order:

1. The path specified by the `RUST_ANALYZER_PATH` environment variable
2. Default location: `~/.cargo/bin/rust-analyzer`

To use a custom path, set the environment variable:
```bash
export RUST_ANALYZER_PATH=/custom/path/to/rust-analyzer
```

Or configure it in your MCP client configuration (see Configuration section above).

### MCP Connection Issues
- Verify the server binary path in your MCP client configuration
- Check that the binary has execute permissions: `chmod +x target/release/rustmcp`
- Ensure no other processes are using the same MCP server name

### LSP Communication Errors
- Verify rust-analyzer works independently: `rust-analyzer --version`
- Check that your Rust project has a valid `Cargo.toml`
- Ensure the workspace path is correct when calling tools

## Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your changes with tests
4. Submit a pull request

