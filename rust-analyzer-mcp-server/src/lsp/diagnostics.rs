// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Diagnostics retrieval, cache aggregation, and LSP report classification.

use crate::{constants::{debug_flags::DEBUG_LSP_CLIENT, lsp_framing, lsp_methods},
            error::McpServerError,
            lsp::{client::RustAnalyzerClient,
                  table_types::{DiagnosticsTable, DocumentUriRef}}};
use serde_json::{Value, json};

impl RustAnalyzerClient {
    /// Retrieves diagnostics for a specific file URI from cache or fallback pull model.
    ///
    /// # Errors
    ///
    /// Returns an error if the LSP diagnostic request fails.
    pub fn diagnostics(
        &mut self,
        uri: DocumentUriRef<'_>,
    ) -> Result<Value, McpServerError> {
        if let Ok(diag_guard) = self.diagnostics.lock()
            && let Some(diags) = diag_guard.get(uri)
        {
            DEBUG_LSP_CLIENT.then(|| {
                // % is Display, ? is Debug.
                tracing::info! {
                    message = "RustAnalyzerClient::diagnostics_cache_hit",
                    uri = %uri,
                    diagnostics_count = diags.len(),
                };
            });

            return Ok(json!(diags));
        }

        let params = json!({
            "textDocument": { "uri": uri },
            "identifier": lsp_framing::RUST_ANALYZER_BINARY
        });

        let response = self.send_request(lsp_methods::DIAGNOSTIC, Some(params))?;

        let Some(items) = response.get("items") else {
            return Ok(json!([]));
        };

        Ok(items.clone())
    }

    /// Aggregates compiler diagnostics from the local push notification cache and open
    /// documents.
    pub fn collect_cached_workspace_diagnostics(&mut self) -> DiagnosticsTable {
        let mut aggregated = DiagnosticsTable::new();

        // 1. Ingest all non-empty diagnostics cached from textDocument/publishDiagnostics
        //    notifications.
        if let Ok(diag_guard) = self.diagnostics.lock() {
            for (uri, diags) in diag_guard.iter() {
                if !diags.is_empty() {
                    aggregated.insert(uri.clone(), diags.clone());
                }
            }
        }

        // 2. Query open documents not yet present in the aggregated cache.
        let open_docs = self.open_documents.clone();
        for doc_uri in &open_docs {
            if aggregated.contains_key(doc_uri) {
                continue;
            }
            if let Ok(doc_diags) = self.diagnostics(doc_uri)
                && let Some(diag_arr) = doc_diags.as_array()
                && !diag_arr.is_empty()
            {
                aggregated.insert(doc_uri.clone(), diag_arr.clone());
            }
        }

        aggregated
    }

    /// Retrieves all workspace diagnostics across open documents or workspace endpoints.
    ///
    /// # Errors
    ///
    /// Returns an error if diagnostics collection fails.
    pub fn workspace_diagnostics(&mut self) -> Result<Value, McpServerError> {
        let params = json!({
            "identifier": lsp_framing::RUST_ANALYZER_BINARY,
            "previousResultId": null
        });

        let pull_response = self.send_request("workspace/diagnostic", Some(params));

        let report = RawWorkspaceDiagnosticsResult::classify(pull_response)
            .unwrap_or_else_fallback(|| self.collect_cached_workspace_diagnostics());

        Ok(report)
    }
}

/// Classification of a raw LSP `workspace/diagnostic` response payload.
#[derive(Debug, Clone, PartialEq)]
pub enum RawWorkspaceDiagnosticsResult {
    /// A valid report payload containing an `items` array or non-empty file mapping.
    ValidReport(Value),
    /// An empty, null, or erroneous response requiring local cache aggregation fallback.
    NeedsCacheFallback,
}

impl RawWorkspaceDiagnosticsResult {
    /// Classifies an LSP response payload into a valid report or a cache-fallback
    /// requirement.
    #[must_use]
    pub fn classify(response: Result<Value, McpServerError>) -> Self {
        let Ok(Value::Object(map)) = response else {
            return Self::NeedsCacheFallback;
        };

        // Format 1: LSP 3.17 WorkspaceDiagnosticReport: { "items": [ ... ] }
        if map.contains_key("items") {
            return Self::ValidReport(Value::Object(map));
        }

        // Format 2: Direct file URI mapping: { "file:///...": [ ... ] }
        if !map.is_empty() {
            return Self::ValidReport(Value::Object(map));
        }

        // Format 3: Empty object or unhandled: fallback to cached diagnostics
        Self::NeedsCacheFallback
    }

    /// Returns the valid LSP report payload, or lazily computes fallback diagnostics from
    /// the supplier.
    #[must_use]
    pub fn unwrap_or_else_fallback<F>(self, fallback_supplier: F) -> Value
    where
        F: FnOnce() -> DiagnosticsTable,
    {
        match self {
            Self::ValidReport(payload) => payload,
            Self::NeedsCacheFallback => json!(fallback_supplier()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_valid_workspace_report() {
        let payload = json!({
            "items": [
                {
                    "uri": "file:///src/main.rs",
                    "items": []
                }
            ]
        });
        assert_eq!(
            RawWorkspaceDiagnosticsResult::classify(Ok(payload.clone())),
            RawWorkspaceDiagnosticsResult::ValidReport(payload)
        );
    }

    #[test]
    fn test_classify_valid_file_map() {
        let payload = json!({
            "file:///src/main.rs": [
                { "severity": 1, "message": "error" }
            ]
        });
        assert_eq!(
            RawWorkspaceDiagnosticsResult::classify(Ok(payload.clone())),
            RawWorkspaceDiagnosticsResult::ValidReport(payload)
        );
    }

    #[test]
    fn test_classify_needs_fallback() {
        assert_eq!(
            RawWorkspaceDiagnosticsResult::classify(Ok(json!({}))),
            RawWorkspaceDiagnosticsResult::NeedsCacheFallback
        );
        assert_eq!(
            RawWorkspaceDiagnosticsResult::classify(Ok(Value::Null)),
            RawWorkspaceDiagnosticsResult::NeedsCacheFallback
        );
        assert_eq!(
            RawWorkspaceDiagnosticsResult::classify(Err(
                McpServerError::RequestCancelled
            )),
            RawWorkspaceDiagnosticsResult::NeedsCacheFallback
        );
    }

    #[test]
    fn test_unwrap_or_else_fallback() {
        let valid = RawWorkspaceDiagnosticsResult::ValidReport(json!({ "items": [] }));
        let fallback_called = std::cell::Cell::new(false);
        let resolved = valid.unwrap_or_else_fallback(|| {
            fallback_called.set(true);
            DiagnosticsTable::new()
        });
        assert_eq!(resolved, json!({ "items": [] }));
        assert!(!fallback_called.get());

        let needs_fallback = RawWorkspaceDiagnosticsResult::NeedsCacheFallback;
        let mut sample_map = DiagnosticsTable::new();
        sample_map.insert(
            "file:///test.rs".to_string(),
            vec![json!({ "severity": 1 })],
        );
        let resolved_fallback = needs_fallback.unwrap_or_else_fallback(|| sample_map);
        assert_eq!(
            resolved_fallback,
            json!({ "file:///test.rs": [{ "severity": 1 }] })
        );
    }
}
