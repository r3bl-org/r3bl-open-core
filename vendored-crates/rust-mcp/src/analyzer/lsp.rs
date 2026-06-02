// LSP types and utilities for rust-analyzer integration
// This module can contain type definitions, LSP message parsing, etc.

use serde_json::Value;

pub struct LspMessage {
    pub content_length: usize,
    pub content: Value,
}

pub fn parse_lsp_message(raw_content: &[u8]) -> anyhow::Result<LspMessage> {
    let content: Value = serde_json::from_slice(raw_content)?;
    Ok(LspMessage {
        content_length: raw_content.len(),
        content,
    })
}
