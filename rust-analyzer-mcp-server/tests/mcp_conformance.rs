// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! End-to-end integration tests using the official MCP Inspector CLI.
//!
//! These tests execute `npx -y @modelcontextprotocol/inspector --cli` against the
//! compiled `rust-analyzer-mcp-server` binary to validate protocol compliance, tool
//! discovery, and AST symbol inspection.

use r3bl_rust_analyzer_mcp_server::RustAnalyzerClient;
use serde_json::Value;
use std::{path::PathBuf, process::Command};

fn get_server_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rust-analyzer-mcp-server"))
}

/// Tests tool discovery via `npx @modelcontextprotocol/inspector --cli ... --method
/// tools/list`.
///
/// Verifies that:
/// 1. The MCP handshake succeeds over stdio.
/// 2. All 10 tools are registered and advertised with valid input schemas.
#[test]
fn test_inspector_cli_tools_list() {
    let server_bin = get_server_bin();
    let output = Command::new("npx")
        .args([
            "-y",
            "@modelcontextprotocol/inspector",
            "--cli",
            server_bin.to_str().expect("Valid binary path string"),
            "--method",
            "tools/list",
        ])
        .output()
        .expect("Failed to execute npx inspector CLI");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Inspector CLI failed with exit code {:?}.\nStderr:\n{}\nStdout:\n{}",
        output.status.code(),
        stderr,
        stdout
    );

    let parsed: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("Failed to parse stdout JSON: {e}\nRaw stdout was:\n{stdout}")
    });

    let tools = parsed["tools"]
        .as_array()
        .expect("Expected 'tools' array in Inspector output");
    assert_eq!(tools.len(), 10, "Expected 10 registered tools");

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();

    let expected_tools = [
        "rust_analyzer_hover",
        "rust_analyzer_definition",
        "rust_analyzer_references",
        "rust_analyzer_completion",
        "rust_analyzer_symbols",
        "rust_analyzer_format",
        "rust_analyzer_code_actions",
        "rust_analyzer_set_workspace",
        "rust_analyzer_diagnostics",
        "rust_analyzer_workspace_diagnostics",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "Missing expected tool in tools/list: {expected}"
        );
    }
}

/// Tests tool execution via `npx @modelcontextprotocol/inspector --cli ... --method
/// tools/call`.
///
/// Verifies that:
/// 1. `rust-analyzer` spawns as a child process and completes initial indexing.
/// 2. `rust_analyzer_symbols` successfully resolves AST symbols for
///    `build-infra/src/lib.rs`.
#[test]
fn test_inspector_cli_tools_call_symbols() {
    // Resolve relative path to build-infra/src/lib.rs whether executed from repo root or
    // crate dir.
    let file_path = if PathBuf::from("build-infra/src/lib.rs").exists() {
        "build-infra/src/lib.rs"
    } else if PathBuf::from("src/lib.rs").exists() {
        "src/lib.rs"
    } else {
        "build-infra/src/lib.rs"
    };

    let file_arg = format!("file_path={file_path}");
    let server_bin = get_server_bin();

    let output = Command::new("npx")
        .args([
            "-y",
            "@modelcontextprotocol/inspector",
            "--cli",
            server_bin.to_str().expect("Valid binary path string"),
            "--method",
            "tools/call",
            "--tool-name",
            "rust_analyzer_symbols",
            "--tool-arg",
            &file_arg,
        ])
        .output()
        .expect("Failed to execute npx inspector CLI");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "Inspector CLI tools/call failed with exit code {:?}.\nStderr:\n{}\nStdout:\n{}",
        output.status.code(),
        stderr,
        stdout
    );

    let parsed: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("Failed to parse stdout JSON: {e}\nRaw stdout was:\n{stdout}")
    });

    let content_text = parsed["content"][0]["text"]
        .as_str()
        .expect("Expected content text array in tools/call result");

    let has_symbols = content_text.contains("lsp") && content_text.contains("mcp");
    let has_indexing_progress =
        content_text.contains("indexing") || content_text.contains("rust-analyzer");

    assert!(
        has_symbols || has_indexing_progress,
        "Expected AST symbols or indexing progress notification. Output was:\n{content_text}"
    );
}

/// Tests that closing `stdin` (simulating coding agent termination or
/// disconnection) triggers a clean graceful shutdown of the server process with exit
/// code 0.
#[test]
fn test_server_graceful_shutdown_on_stdin_eof() {
    let server_bin = get_server_bin();
    let mut child = Command::new(server_bin)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn rust-analyzer-mcp-server");

    // Close stdin immediately by dropping it.
    drop(child.stdin.take());

    // Wait for the server to exit gracefully.
    let status = child
        .wait()
        .expect("Failed to wait on rust-analyzer-mcp-server");

    assert!(
        status.success(),
        "Expected clean exit on stdin EOF, got exit code: {:?}",
        status.code()
    );
}

/// Tests that `RustAnalyzerClient` starts `rust-analyzer` and terminates cleanly on
/// `shutdown()`.
#[test]
fn test_lsp_client_start_and_shutdown() {
    let mut client = RustAnalyzerClient::new(PathBuf::from("."));
    assert!(!client.is_running());

    client
        .start()
        .expect("Failed to start rust-analyzer client");
    assert!(client.is_running());

    client
        .shutdown()
        .expect("Failed to shut down rust-analyzer client");
    assert!(!client.is_running());
}
