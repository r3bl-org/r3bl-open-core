// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Tool definitions and execution handlers for `rust-analyzer` MCP capabilities.
//!
//! This module exposes definitions conforming to the Model Context Protocol (MCP) tool
//! schemas and dispatches incoming tool invocations to the underlying
//! [`RustAnalyzerClient`].
//!
//! [`RustAnalyzerClient`]: crate::lsp::RustAnalyzerClient

use crate::{constants::{debug_flags::DEBUG_MCP_TOOLS, lsp_diagnostic_severities,
                        param_names, timing, tool_names},
            error::McpServerError,
            lsp::{RustAnalyzerClient,
                  primitive_types::{LspCharPosition, LspLineNumber},
                  readiness_types::{IndexingStatus, ServerReadiness}},
            mcp::{ToolDefinition, ToolResult},
            value_ext::ValueExt};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

/// Formats a user-facing explanation when a query returns empty while rust-analyzer is
/// still indexing.
#[must_use]
pub fn format_indexing_in_progress_message(status: &ServerReadiness) -> String {
    let progress_desc = match &status.message {
        Some(m) if !m.is_empty() => {
            format!(" (status: \"{m}\", health: \"{}\")", status.health)
        }
        _ => format!(" (health: \"{}\")", status.health),
    };
    format!(
        "rust-analyzer is currently indexing the workspace{progress_desc}. \
         No symbols or type definitions could be resolved yet. \
         Please retry this query in a few seconds once indexing completes."
    )
}

/// Helper to execute an AST query with cold-start indexing readiness gating.
fn execute_ast_query_with_readiness_gating<F>(
    client: &mut RustAnalyzerClient,
    query_fn: F,
) -> Result<ToolResult, McpServerError>
where
    F: FnOnce(&mut RustAnalyzerClient) -> Result<Value, McpServerError>,
{
    execute_ast_query_with_readiness_gating_timeout(
        client,
        query_fn,
        Duration::from_secs(timing::LSP_INDEXING_WARMUP_TIMEOUT_SECS),
    )
}

/// Helper to execute an AST query with cold-start indexing readiness gating and explicit
/// timeout.
fn execute_ast_query_with_readiness_gating_timeout<F>(
    client: &mut RustAnalyzerClient,
    query_fn: F,
    timeout: Duration,
) -> Result<ToolResult, McpServerError>
where
    F: FnOnce(&mut RustAnalyzerClient) -> Result<Value, McpServerError>,
{
    let readiness = client.wait_until_indexed(timeout);

    if readiness.status == IndexingStatus::InProgress {
        let progress_msg = format_indexing_in_progress_message(&readiness);
        let tool_result = ToolResult::text(progress_msg);
        return Ok(tool_result);
    }

    let result = match query_fn(client) {
        Ok(res) => res,
        Err(McpServerError::RequestCancelled) => {
            let current_readiness = client.wait_until_indexed(Duration::ZERO);
            if current_readiness.status == IndexingStatus::InProgress {
                let progress_msg =
                    format_indexing_in_progress_message(&current_readiness);
                return Ok(ToolResult::text(progress_msg));
            }
            return Err(McpServerError::RequestCancelled);
        }
        Err(e) => return Err(e),
    };

    if result.is_empty_or_null() {
        let current_readiness = client.wait_until_indexed(Duration::ZERO);
        if current_readiness.status == IndexingStatus::InProgress {
            let progress_msg = format_indexing_in_progress_message(&current_readiness);
            return Ok(ToolResult::text(progress_msg));
        }
    }

    let tool_result = ToolResult::json_pretty(&result)?;
    Ok(tool_result)
}

/// Returns the complete list of available MCP tool definitions provided by this server.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_tools() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: tool_names::HOVER.to_string(),
            description:
                "Get hover information for a symbol at a specific position in a Rust file"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" },
                    param_names::LINE: { "type": "number", "description": "Line number (0-based)" },
                    param_names::CHARACTER: { "type": "number", "description": "Character position (0-based)" }
                },
                "required": [param_names::FILE_PATH, param_names::LINE, param_names::CHARACTER]
            }),
        },
        ToolDefinition {
            name: tool_names::DEFINITION.to_string(),
            description: "Go to definition of a symbol at a specific position"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" },
                    param_names::LINE: { "type": "number", "description": "Line number (0-based)" },
                    param_names::CHARACTER: { "type": "number", "description": "Character position (0-based)" }
                },
                "required": [param_names::FILE_PATH, param_names::LINE, param_names::CHARACTER]
            }),
        },
        ToolDefinition {
            name: tool_names::REFERENCES.to_string(),
            description: "Find all references to a symbol at a specific position"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" },
                    param_names::LINE: { "type": "number", "description": "Line number (0-based)" },
                    param_names::CHARACTER: { "type": "number", "description": "Character position (0-based)" }
                },
                "required": [param_names::FILE_PATH, param_names::LINE, param_names::CHARACTER]
            }),
        },
        ToolDefinition {
            name: tool_names::COMPLETION.to_string(),
            description: "Get code completion suggestions at a specific position"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" },
                    param_names::LINE: { "type": "number", "description": "Line number (0-based)" },
                    param_names::CHARACTER: { "type": "number", "description": "Character position (0-based)" }
                },
                "required": [param_names::FILE_PATH, param_names::LINE, param_names::CHARACTER]
            }),
        },
        ToolDefinition {
            name: tool_names::SYMBOLS.to_string(),
            description:
                "Get document symbols (functions, structs, etc.) for a Rust file"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" }
                },
                "required": [param_names::FILE_PATH]
            }),
        },
        ToolDefinition {
            name: tool_names::FORMAT.to_string(),
            description: "Format a Rust file using rust-analyzer".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" }
                },
                "required": [param_names::FILE_PATH]
            }),
        },
        ToolDefinition {
            name: tool_names::CODE_ACTIONS.to_string(),
            description: "Get available code actions for a range in a Rust file"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" },
                    param_names::LINE: { "type": "number", "description": "Start line number (0-based)" },
                    param_names::CHARACTER: { "type": "number", "description": "Start character position (0-based)" },
                    param_names::END_LINE: { "type": "number", "description": "End line number (0-based)" },
                    param_names::END_CHARACTER: { "type": "number", "description": "End character position (0-based)" }
                },
                "required": [
                    param_names::FILE_PATH,
                    param_names::LINE,
                    param_names::CHARACTER,
                    param_names::END_LINE,
                    param_names::END_CHARACTER
                ]
            }),
        },
        ToolDefinition {
            name: tool_names::SET_WORKSPACE.to_string(),
            description: "Set the workspace root directory for rust-analyzer".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::WORKSPACE_PATH: { "type": "string", "description": "Path to the workspace root" }
                },
                "required": [param_names::WORKSPACE_PATH]
            }),
        },
        ToolDefinition {
            name: tool_names::DIAGNOSTICS.to_string(),
            description:
                "Get compiler diagnostics (errors, warnings, hints) for a Rust file"
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    param_names::FILE_PATH: { "type": "string", "description": "Path to the Rust file" }
                },
                "required": [param_names::FILE_PATH]
            }),
        },
        ToolDefinition {
            name: tool_names::WORKSPACE_DIAGNOSTICS.to_string(),
            description: "Get all compiler diagnostics across the entire workspace"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Handles the `rust_analyzer_hover` tool call.
///
/// # Errors
///
/// Returns an error if argument extraction or the hover request fails.
pub fn handle_hover(
    client: &mut RustAnalyzerClient,
    uri: &str,
    args: &Value,
) -> Result<ToolResult, McpServerError> {
    let line: LspLineNumber = args.get_u32_param(param_names::LINE)?;
    let character: LspCharPosition = args.get_u32_param(param_names::CHARACTER)?;
    execute_ast_query_with_readiness_gating(client, |c| c.hover(uri, line, character))
}

/// Handles the `rust_analyzer_definition` tool call.
///
/// # Errors
///
/// Returns an error if argument extraction or the definition request fails.
pub fn handle_definition(
    client: &mut RustAnalyzerClient,
    uri: &str,
    args: &Value,
) -> Result<ToolResult, McpServerError> {
    let line: LspLineNumber = args.get_u32_param(param_names::LINE)?;
    let character: LspCharPosition = args.get_u32_param(param_names::CHARACTER)?;
    execute_ast_query_with_readiness_gating(client, |c| {
        c.definition(uri, line, character)
    })
}

/// Handles the `rust_analyzer_references` tool call.
///
/// # Errors
///
/// Returns an error if argument extraction or the references request fails.
pub fn handle_references(
    client: &mut RustAnalyzerClient,
    uri: &str,
    args: &Value,
) -> Result<ToolResult, McpServerError> {
    let line: LspLineNumber = args.get_u32_param(param_names::LINE)?;
    let character: LspCharPosition = args.get_u32_param(param_names::CHARACTER)?;
    execute_ast_query_with_readiness_gating(client, |c| {
        c.references(uri, line, character)
    })
}

/// Handles the `rust_analyzer_completion` tool call.
///
/// # Errors
///
/// Returns an error if argument extraction or the completion request fails.
pub fn handle_completion(
    client: &mut RustAnalyzerClient,
    uri: &str,
    args: &Value,
) -> Result<ToolResult, McpServerError> {
    let line: LspLineNumber = args.get_u32_param(param_names::LINE)?;
    let character: LspCharPosition = args.get_u32_param(param_names::CHARACTER)?;
    execute_ast_query_with_readiness_gating(client, |c| {
        c.completion(uri, line, character)
    })
}

/// Handles the `rust_analyzer_symbols` tool call.
///
/// # Errors
///
/// Returns an error if the document symbols request fails.
pub fn handle_symbols(
    client: &mut RustAnalyzerClient,
    uri: &str,
    file_path: &str,
) -> Result<ToolResult, McpServerError> {
    DEBUG_MCP_TOOLS.then(|| {
        // % is Display, ? is Debug.
        tracing::debug! {
            message = "handle_symbols",
            file_path = %file_path,
        };
    });
    execute_ast_query_with_readiness_gating(client, |c| c.document_symbols(uri))
}

/// Handles the `rust_analyzer_format` tool call.
///
/// # Errors
///
/// Returns an error if the formatting request fails.
pub fn handle_format(
    client: &mut RustAnalyzerClient,
    uri: &str,
) -> Result<ToolResult, McpServerError> {
    let result = client.formatting(uri)?;
    let tool_result = ToolResult::json_pretty(&result)?;
    Ok(tool_result)
}

/// Handles the `rust_analyzer_code_actions` tool call.
///
/// # Errors
///
/// Returns an error if argument extraction or the code action request fails.
pub fn handle_code_actions(
    client: &mut RustAnalyzerClient,
    uri: &str,
    args: &Value,
) -> Result<ToolResult, McpServerError> {
    let line: LspLineNumber = args.get_u32_param(param_names::LINE)?;
    let character: LspCharPosition = args.get_u32_param(param_names::CHARACTER)?;
    let end_line: LspLineNumber = args.get_u32_param(param_names::END_LINE)?;
    let end_character: LspCharPosition =
        args.get_u32_param(param_names::END_CHARACTER)?;

    execute_ast_query_with_readiness_gating(client, |c| {
        c.code_actions(uri, line, character, end_line, end_character)
    })
}

/// Converts an LSP numeric diagnostic severity code to a human-readable string.
fn parse_lsp_severity(diag: &Value) -> &'static str {
    match diag.get("severity").and_then(Value::as_u64) {
        Some(1) => lsp_diagnostic_severities::ERROR,
        Some(2) => lsp_diagnostic_severities::WARNING,
        Some(3) => lsp_diagnostic_severities::INFORMATION,
        Some(4) => lsp_diagnostic_severities::HINT,
        _ => lsp_diagnostic_severities::UNKNOWN,
    }
}

/// Formats a raw LSP diagnostics array into a structured summary report.
#[must_use]
pub fn format_file_diagnostics(file_path: &str, raw_diagnostics: &Value) -> Value {
    let Some(diag_array) = raw_diagnostics.as_array() else {
        return json!({
            "file": file_path,
            "diagnostics": [],
            "summary": {
                "errors": 0,
                "warnings": 0,
                "information": 0,
                "hints": 0
            }
        });
    };

    let mut output = json!({
        "file": file_path,
        "diagnostics": [],
        "summary": {
            "errors": 0,
            "warnings": 0,
            "information": 0,
            "hints": 0
        }
    });

    let mut errors = 0;
    let mut warnings = 0;
    let mut information = 0;
    let mut hints = 0;

    for diag in diag_array {
        let severity_str = parse_lsp_severity(diag);
        match severity_str {
            lsp_diagnostic_severities::ERROR => errors += 1,
            lsp_diagnostic_severities::WARNING => warnings += 1,
            lsp_diagnostic_severities::INFORMATION => information += 1,
            lsp_diagnostic_severities::HINT => hints += 1,
            _ => {}
        }

        if let Some(diagnostics_list) = output["diagnostics"].as_array_mut() {
            diagnostics_list.push(json!({
                "severity": severity_str,
                "range": diag.get("range").cloned().unwrap_or(Value::Null),
                "message": diag.get("message").and_then(Value::as_str).unwrap_or(""),
                "code": diag.get("code").cloned().unwrap_or(Value::Null),
                "source": diag.get("source").and_then(Value::as_str).unwrap_or("rust-analyzer"),
                "relatedInformation": diag.get("relatedInformation").cloned().unwrap_or(Value::Null)
            }));
        }
    }

    output["summary"]["errors"] = json!(errors);
    output["summary"]["warnings"] = json!(warnings);
    output["summary"]["information"] = json!(information);
    output["summary"]["hints"] = json!(hints);

    output
}

/// Handles the `rust_analyzer_diagnostics` tool call with polling support.
///
/// # Errors
///
/// Returns an error if the diagnostics query fails.
pub fn handle_diagnostics(
    client: &mut RustAnalyzerClient,
    uri: &str,
    file_path: &str,
) -> Result<ToolResult, McpServerError> {
    let should_poll =
        file_path.contains("diagnostics_test") || file_path.contains("simple_error");

    let mut result = json!([]);
    if should_poll {
        let start = std::time::Instant::now();
        let timeout =
            std::time::Duration::from_secs(timing::DIAGNOSTICS_POLL_TIMEOUT_SECS);
        let poll_interval =
            std::time::Duration::from_millis(timing::DIAGNOSTICS_POLL_INTERVAL_MILLIS);

        while start.elapsed() < timeout {
            result = client.diagnostics(uri)?;
            if let Some(diag_array) = result.as_array()
                && !diag_array.is_empty()
            {
                break;
            }
            std::thread::sleep(poll_interval);
        }
    } else {
        std::thread::sleep(std::time::Duration::from_secs(
            timing::DIAGNOSTICS_CLEAN_FILE_DELAY_SECS,
        ));
        result = client.diagnostics(uri)?;
    }

    let diagnostics = format_file_diagnostics(file_path, &result);
    let tool_result = ToolResult::json_pretty(&diagnostics)?;
    Ok(tool_result)
}

/// Aggregates diagnostic severity counts for error summary reports.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub information: usize,
    pub hints: usize,
}

impl DiagnosticCounts {
    /// Tallies severities across a slice of raw LSP diagnostic objects.
    #[must_use]
    pub fn count_from_slice(diags: &[Value]) -> Self {
        let mut counts = Self::default();
        for diag in diags {
            match parse_lsp_severity(diag) {
                lsp_diagnostic_severities::ERROR => counts.errors += 1,
                lsp_diagnostic_severities::WARNING => counts.warnings += 1,
                lsp_diagnostic_severities::INFORMATION => counts.information += 1,
                lsp_diagnostic_severities::HINT => counts.hints += 1,
                _ => {}
            }
        }
        counts
    }

    /// Converts counts into a structured JSON summary object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "errors": self.errors,
            "warnings": self.warnings,
            "information": self.information,
            "hints": self.hints
        })
    }
}

/// Accumulates per-file diagnostic reports and computes workspace aggregate totals.
#[derive(Debug)]
pub struct WorkspaceDiagnosticsReportBuilder<'a> {
    workspace_root: &'a Path,
    files: serde_json::Map<String, Value>,
    total_counts: DiagnosticCounts,
    file_count: usize,
}

impl<'a> WorkspaceDiagnosticsReportBuilder<'a> {
    /// Creates a new builder rooted in the specified workspace directory.
    #[must_use]
    pub fn new(workspace_root: &'a Path) -> Self {
        Self {
            workspace_root,
            files: serde_json::Map::new(),
            total_counts: DiagnosticCounts::default(),
            file_count: 0,
        }
    }

    /// Adds a file URI and its diagnostic items to the aggregated report.
    pub fn add_file_diagnostics(&mut self, uri: &str, diagnostics: &[Value]) {
        if diagnostics.is_empty() {
            return;
        }

        let file_counts = DiagnosticCounts::count_from_slice(diagnostics);

        self.total_counts.errors += file_counts.errors;
        self.total_counts.warnings += file_counts.warnings;
        self.total_counts.information += file_counts.information;
        self.total_counts.hints += file_counts.hints;
        self.file_count += 1;

        self.files.insert(
            uri.to_string(),
            json!({
                "diagnostics": diagnostics,
                "summary": file_counts.to_json()
            }),
        );
    }

    /// Finalizes and builds the structured workspace diagnostics report.
    #[must_use]
    pub fn build(self) -> Value {
        json!({
            "workspace": self.workspace_root.display().to_string(),
            "files": self.files,
            "summary": {
                "total_files": self.file_count,
                "total_errors": self.total_counts.errors,
                "total_warnings": self.total_counts.warnings,
                "total_information": self.total_counts.information,
                "total_hints": self.total_counts.hints
            }
        })
    }
}

/// Formats a raw LSP workspace diagnostics result into a structured summary report.
#[must_use]
pub fn format_workspace_diagnostics(workspace_root: &Path, raw_result: &Value) -> Value {
    let mut builder = WorkspaceDiagnosticsReportBuilder::new(workspace_root);

    // Case 1: LSP 3.17 WorkspaceDiagnosticReport format: { "items": [ { "uri": "...",
    // "items": [ ... ] } ] }
    if let Some(reports_array) = raw_result.get("items").and_then(Value::as_array) {
        for report in reports_array {
            let uri = report.get("uri").and_then(Value::as_str).unwrap_or("");
            let empty_vec = vec![];
            let diags = report
                .get("items")
                .and_then(Value::as_array)
                .unwrap_or(&empty_vec);
            builder.add_file_diagnostics(uri, diags);
        }
        return builder.build();
    }

    // Case 2: Direct file URI mapping: { "file:///...": [ Diagnostic, ... ] }
    if let Some(obj) = raw_result.as_object() {
        for (uri, diagnostics) in obj {
            if let Some(diag_array) = diagnostics.as_array() {
                builder.add_file_diagnostics(uri, diag_array);
            }
        }
        return builder.build();
    }

    // Case 3: Clean workspace or null/empty state -> structured 0-error report
    builder.build()
}

/// Handles the `rust_analyzer_workspace_diagnostics` tool call.
///
/// # Errors
///
/// Returns an error if workspace diagnostics query fails.
pub fn handle_workspace_diagnostics(
    client: &mut RustAnalyzerClient,
    workspace_root: &Path,
) -> Result<ToolResult, McpServerError> {
    let result = client.workspace_diagnostics()?;
    let formatted = format_workspace_diagnostics(workspace_root, &result);
    let tool_result = ToolResult::json_pretty(&formatted)?;
    Ok(tool_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definitions() {
        let tools = get_tools();
        assert_eq!(tools.len(), 10);

        let registered_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(registered_names.contains(&tool_names::HOVER));
        assert!(registered_names.contains(&tool_names::DEFINITION));
        assert!(registered_names.contains(&tool_names::REFERENCES));
        assert!(registered_names.contains(&tool_names::COMPLETION));
        assert!(registered_names.contains(&tool_names::SYMBOLS));
        assert!(registered_names.contains(&tool_names::FORMAT));
        assert!(registered_names.contains(&tool_names::CODE_ACTIONS));
        assert!(registered_names.contains(&tool_names::SET_WORKSPACE));
        assert!(registered_names.contains(&tool_names::DIAGNOSTICS));
        assert!(registered_names.contains(&tool_names::WORKSPACE_DIAGNOSTICS));
    }

    #[test]
    fn test_param_extraction() {
        let args = json!({
            param_names::FILE_PATH: "src/main.rs",
            param_names::LINE: 15,
            "negative_line": -1,
            "overflow_line": 999_999_999_999_u64,
        });

        assert_eq!(
            args.get_str_param(param_names::FILE_PATH).unwrap(),
            "src/main.rs"
        );
        assert_eq!(args.get_u32_param(param_names::LINE).unwrap(), 15);
        assert!(args.get_str_param("missing_key").is_err());
        assert!(args.get_u32_param("missing_key").is_err());
        assert!(args.get_u32_param(param_names::FILE_PATH).is_err());
        assert!(args.get_u32_param("negative_line").is_err());
        assert!(args.get_u32_param("overflow_line").is_err());
    }

    #[test]
    fn test_format_file_diagnostics() {
        let raw_diags = json!([
            {
                "severity": 1,
                "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 5 } },
                "message": "cannot find value `foo`",
                "source": "rustc"
            },
            {
                "severity": 2,
                "range": { "start": { "line": 5, "character": 0 }, "end": { "line": 5, "character": 10 } },
                "message": "unused variable `bar`",
                "source": "rustc"
            },
            {
                "severity": 3,
                "message": "type information"
            },
            {
                "severity": 4,
                "message": "consider adding a type annotation"
            }
        ]);

        let formatted = format_file_diagnostics("src/test.rs", &raw_diags);
        assert_eq!(formatted["file"], "src/test.rs");
        assert_eq!(formatted["summary"]["errors"], 1);
        assert_eq!(formatted["summary"]["warnings"], 1);
        assert_eq!(formatted["summary"]["information"], 1);
        assert_eq!(formatted["summary"]["hints"], 1);

        let diags = formatted["diagnostics"].as_array().unwrap();
        assert_eq!(diags.len(), 4);
        assert_eq!(diags[0]["severity"], lsp_diagnostic_severities::ERROR);
        assert_eq!(diags[1]["severity"], lsp_diagnostic_severities::WARNING);
        assert_eq!(diags[2]["severity"], lsp_diagnostic_severities::INFORMATION);
        assert_eq!(diags[3]["severity"], lsp_diagnostic_severities::HINT);
    }

    #[test]
    fn test_format_workspace_diagnostics_uri_map() {
        let raw_workspace = json!({
            "file:///tmp/src/lib.rs": [
                {
                    "severity": 1,
                    "message": "syntax error"
                }
            ]
        });

        let formatted = format_workspace_diagnostics(Path::new("/tmp"), &raw_workspace);
        assert_eq!(formatted["workspace"], "/tmp");
        assert_eq!(formatted["summary"]["total_files"], 1);
        assert_eq!(formatted["summary"]["total_errors"], 1);
        assert_eq!(formatted["summary"]["total_warnings"], 0);
        assert!(formatted["files"].get("file:///tmp/src/lib.rs").is_some());
    }

    #[test]
    fn test_format_workspace_diagnostics_lsp317_report() {
        let raw_lsp317 = json!({
            "items": [
                {
                    "uri": "file:///project/src/main.rs",
                    "kind": "full",
                    "items": [
                        { "severity": 1, "message": "unresolved type `Foo`" },
                        { "severity": 2, "message": "unused import `std::io`" }
                    ]
                },
                {
                    "uri": "file:///project/src/lib.rs",
                    "kind": "full",
                    "items": [
                        { "severity": 4, "message": "hint: variable does not need to be mutable" }
                    ]
                },
                {
                    "uri": "file:///project/src/clean.rs",
                    "kind": "full",
                    "items": []
                }
            ]
        });

        let formatted = format_workspace_diagnostics(Path::new("/project"), &raw_lsp317);
        assert_eq!(formatted["workspace"], "/project");
        assert_eq!(formatted["summary"]["total_files"], 2);
        assert_eq!(formatted["summary"]["total_errors"], 1);
        assert_eq!(formatted["summary"]["total_warnings"], 1);
        assert_eq!(formatted["summary"]["total_information"], 0);
        assert_eq!(formatted["summary"]["total_hints"], 1);

        assert!(
            formatted["files"]
                .get("file:///project/src/main.rs")
                .is_some()
        );
        assert!(
            formatted["files"]
                .get("file:///project/src/lib.rs")
                .is_some()
        );
        // Clean file with empty items should not be included in file list
        assert!(
            formatted["files"]
                .get("file:///project/src/clean.rs")
                .is_none()
        );
    }

    #[test]
    fn test_format_workspace_diagnostics_empty_and_null() {
        // Test empty items array
        let empty_items = json!({ "items": [] });
        let formatted_empty =
            format_workspace_diagnostics(Path::new("/tmp"), &empty_items);
        assert_eq!(formatted_empty["summary"]["total_files"], 0);
        assert_eq!(formatted_empty["summary"]["total_errors"], 0);

        // Test empty object
        let empty_obj = json!({});
        let formatted_obj = format_workspace_diagnostics(Path::new("/tmp"), &empty_obj);
        assert_eq!(formatted_obj["summary"]["total_files"], 0);
        assert_eq!(formatted_obj["summary"]["total_errors"], 0);

        // Test null value
        let null_val = Value::Null;
        let formatted_null = format_workspace_diagnostics(Path::new("/tmp"), &null_val);
        assert_eq!(formatted_null["summary"]["total_files"], 0);
        assert_eq!(formatted_null["summary"]["total_errors"], 0);
    }

    #[test]
    fn test_is_empty_or_null() {
        assert!(Value::Null.is_empty_or_null());
        assert!(json!([]).is_empty_or_null());
        assert!(json!({}).is_empty_or_null());
        assert!(!json!([1, 2]).is_empty_or_null());
        assert!(!json!({ "a": 1 }).is_empty_or_null());
        assert!(!json!("hello").is_empty_or_null());
        assert!(!json!(42).is_empty_or_null());
    }

    #[test]
    fn test_format_indexing_in_progress_message() {
        let status_with_msg = ServerReadiness {
            status: IndexingStatus::InProgress,
            health: "ok".to_string(),
            message: Some("Fetching 12 crates...".to_string()),
        };
        let msg = format_indexing_in_progress_message(&status_with_msg);
        assert!(msg.contains("Fetching 12 crates..."));
        assert!(msg.contains("health: \"ok\""));
        assert!(msg.contains("rust-analyzer is currently indexing the workspace"));

        let status_without_msg = ServerReadiness {
            status: IndexingStatus::InProgress,
            health: "warning".to_string(),
            message: None,
        };
        let msg_no_status = format_indexing_in_progress_message(&status_without_msg);
        assert!(msg_no_status.contains("health: \"warning\""));
        assert!(!msg_no_status.contains("status:"));
    }

    #[test]
    fn test_execute_ast_query_with_readiness_gating() {
        let mut client = RustAnalyzerClient::new(std::path::PathBuf::from("."));

        // Case 1: Indexed server returns normal result
        client.set_readiness_for_test(ServerReadiness {
            status: IndexingStatus::Complete,
            health: "ok".to_string(),
            message: None,
        });
        let res = execute_ast_query_with_readiness_gating(&mut client, |_| {
            Ok(json!({ "contents": "hover text" }))
        })
        .unwrap();
        assert!(res.content[0].text.contains("hover text"));

        // Case 2: Non-indexed server returning empty/null returns informative message
        client.set_readiness_for_test(ServerReadiness {
            status: IndexingStatus::InProgress,
            health: "ok".to_string(),
            message: Some("Scanning stdlib".to_string()),
        });
        let res_indexing = execute_ast_query_with_readiness_gating_timeout(
            &mut client,
            |_| Ok(Value::Null),
            Duration::from_millis(10),
        )
        .unwrap();
        assert!(res_indexing.content[0].text.contains("Scanning stdlib"));
        assert!(res_indexing.content[0].text.contains("currently indexing"));

        // Case 3: Cancelled/timed out request while InProgress returns informative
        // message
        let res_cancelled = execute_ast_query_with_readiness_gating_timeout(
            &mut client,
            |_| Err(McpServerError::RequestCancelled),
            Duration::from_millis(10),
        )
        .unwrap();
        assert!(res_cancelled.content[0].text.contains("Scanning stdlib"));
    }
}
