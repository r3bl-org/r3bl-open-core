// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Binary entry point for the [`rust-analyzer`] Model Context Protocol (MCP) server.
//!
//! Exposes semantic code analysis, symbol navigation, diagnostics, and code actions from
//! [`rust-analyzer`] over the standard Model Context Protocol (MCP).
//!
//! [`rust-analyzer`]: https://rust-analyzer.github.io/

use clap::Parser;
use miette::Result;
use r3bl_rust_analyzer_mcp_server::{CLIArg, RustAnalyzerMCPServer};
use r3bl_tui::{log::try_initialize_logging_global, ok};

fn main() -> Result<()> {
    // Parse command line arguments.
    let cli_arg = CLIArg::parse();

    // Initialize structured tracing writing to stderr or file based on CLI args.
    // Note: stdout is exclusively reserved for JSON-RPC MCP message transport.
    let tracing_config = cli_arg.to_tracing_config();
    let _log_guard = try_initialize_logging_global(tracing_config);

    // Resolve workspace directory path.
    let workspace_path = cli_arg.resolve_workspace_path();

    // Create and run the MCP server over stdio.
    RustAnalyzerMCPServer::start(workspace_path)?;

    ok!()
}
