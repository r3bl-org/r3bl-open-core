// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! AST navigation, hover, completion, definitions, references, and code actions queries.

use crate::{constants::{lsp_framing, lsp_methods, timing},
            error::McpServerError,
            lsp::{client::RustAnalyzerClient,
                  primitive_types::{LspCharPosition, LspLineNumber},
                  table_types::DocumentUriRef}};
use r3bl_tui::ok;
use serde_json::{Value, json};

impl RustAnalyzerClient {
    /// Opens a document in the `rust-analyzer` session via `textDocument/didOpen`.
    ///
    /// If the file is not currently tracked, reads its content from disk and sends the
    /// initial open notification.
    ///
    /// # Errors
    ///
    /// Returns [`McpServerError`] if reading the file from disk or sending the
    /// notification fails.
    pub fn open_document(
        &mut self,
        uri: DocumentUriRef<'_>,
        content: &str,
    ) -> Result<(), McpServerError> {
        if self.open_documents.contains(&uri.to_string()) {
            return ok!();
        }

        let params = json!({
            "textDocument": {
                "uri": uri,
                "languageId": lsp_framing::RUST_LANGUAGE_ID,
                "version": 1,
                "text": content
            }
        });

        self.send_notification(lsp_methods::DID_OPEN, Some(params))?;
        self.open_documents.push(uri.to_string());

        ok!()
    }

    /// Requests hover type signatures and documentation for a specific file position.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn hover(
        &mut self,
        uri: DocumentUriRef<'_>,
        line: LspLineNumber,
        character: LspCharPosition,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request(lsp_methods::HOVER, Some(params))
    }

    /// Requests symbol definition locations for a specific file position.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn definition(
        &mut self,
        uri: DocumentUriRef<'_>,
        line: LspLineNumber,
        character: LspCharPosition,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request(lsp_methods::DEFINITION, Some(params))
    }

    /// Requests all references to the symbol at a given position across the workspace.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn references(
        &mut self,
        uri: DocumentUriRef<'_>,
        line: LspLineNumber,
        character: LspCharPosition,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character },
            "context": { "includeDeclaration": true }
        });

        self.send_request(lsp_methods::REFERENCES, Some(params))
    }

    /// Requests context-aware code completion items for a given file position.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn completion(
        &mut self,
        uri: DocumentUriRef<'_>,
        line: LspLineNumber,
        character: LspCharPosition,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        });

        self.send_request(lsp_methods::COMPLETION, Some(params))
    }

    /// Requests document outline symbols (functions, structs, traits, modules).
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn document_symbols(
        &mut self,
        uri: DocumentUriRef<'_>,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri }
        });

        self.send_request(lsp_methods::DOCUMENT_SYMBOL, Some(params))
    }

    /// Requests document formatting edits from `rust-analyzer`.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP request fails.
    pub fn formatting(
        &mut self,
        uri: DocumentUriRef<'_>,
    ) -> Result<Value, McpServerError> {
        let params = json!({
            "textDocument": { "uri": uri },
            "options": {
                "tabSize": timing::DEFAULT_TAB_SIZE,
                "insertSpaces": true
            }
        });

        self.send_request(lsp_methods::FORMATTING, Some(params))
    }

    /// Checks whether an LSP diagnostic intersects the given selection line range.
    pub fn diagnostic_intersects_line_range(
        diag: &Value,
        start_line: LspLineNumber,
        end_line: LspLineNumber,
    ) -> bool {
        let Some(range) = diag.get("range") else {
            return false;
        };
        let Some(diag_start) = range
            .get("start")
            .and_then(|s| s.get("line"))
            .and_then(Value::as_u64)
        else {
            return false;
        };
        let Some(diag_end) = range
            .get("end")
            .and_then(|e| e.get("line"))
            .and_then(Value::as_u64)
        else {
            return false;
        };

        let Ok(diag_start_line) = LspLineNumber::try_from(diag_start) else {
            return false;
        };
        let Ok(diag_end_line) = LspLineNumber::try_from(diag_end) else {
            return false;
        };

        diag_start_line <= end_line && diag_end_line >= start_line
    }

    /// Requests available code actions for a given text range.
    ///
    /// # Errors
    ///
    /// Returns an error if the code actions request fails.
    pub fn code_actions(
        &mut self,
        uri: DocumentUriRef<'_>,
        start_line: LspLineNumber,
        start_char: LspCharPosition,
        end_line: LspLineNumber,
        end_char: LspCharPosition,
    ) -> Result<Value, McpServerError> {
        let diagnostics = self.diagnostics(uri).unwrap_or_else(|_| json!([]));

        let filtered_diagnostics: Vec<Value> = diagnostics
            .as_array()
            .map(|diags| {
                diags
                    .iter()
                    .filter(|d| {
                        Self::diagnostic_intersects_line_range(d, start_line, end_line)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        let params = json!({
            "textDocument": { "uri": uri },
            "range": {
                "start": { "line": start_line, "character": start_char },
                "end": { "line": end_line, "character": end_char }
            },
            "context": {
                "diagnostics": filtered_diagnostics,
                "only": [
                    "quickfix",
                    "refactor",
                    "refactor.extract",
                    "refactor.inline",
                    "refactor.rewrite",
                    "source"
                ]
            }
        });

        self.send_request(lsp_methods::CODE_ACTION, Some(params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_intersects_line_range_contained() {
        let diag = json!({
            "range": {
                "start": { "line": 10, "character": 0 },
                "end": { "line": 12, "character": 5 }
            }
        });
        assert!(RustAnalyzerClient::diagnostic_intersects_line_range(
            &diag, 5, 15
        ));
    }

    #[test]
    fn test_diagnostic_intersects_line_range_overlapping() {
        let diag = json!({
            "range": {
                "start": { "line": 8, "character": 0 },
                "end": { "line": 12, "character": 5 }
            }
        });
        // Selection starts at 10 (inside the diag)
        assert!(RustAnalyzerClient::diagnostic_intersects_line_range(
            &diag, 10, 20
        ));
        // Selection ends at 10 (inside the diag)
        assert!(RustAnalyzerClient::diagnostic_intersects_line_range(
            &diag, 0, 10
        ));
    }

    #[test]
    fn test_diagnostic_intersects_line_range_disjoint() {
        let diag = json!({
            "range": {
                "start": { "line": 10, "character": 0 },
                "end": { "line": 12, "character": 5 }
            }
        });
        // Disjoint before
        assert!(!RustAnalyzerClient::diagnostic_intersects_line_range(
            &diag, 0, 9
        ));
        // Disjoint after
        assert!(!RustAnalyzerClient::diagnostic_intersects_line_range(
            &diag, 13, 20
        ));
    }

    #[test]
    fn test_diagnostic_intersects_line_range_malformed() {
        assert!(!RustAnalyzerClient::diagnostic_intersects_line_range(
            &json!({}),
            0,
            10
        ));
        assert!(!RustAnalyzerClient::diagnostic_intersects_line_range(
            &json!({ "range": null }),
            0,
            10
        ));
        assert!(!RustAnalyzerClient::diagnostic_intersects_line_range(
            &json!({ "range": { "start": {} } }),
            0,
            10
        ));
    }
}
