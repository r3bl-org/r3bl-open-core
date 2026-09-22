// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Strip redundant `rust` language tags from code fence openings in rustdoc comments.
//!
//! Rustdoc treats unlabeled code fences as Rust by default. This module normalizes:
//!
//! ```text
//! - ```rust        -> ```
//! - ```rust,<attr> -> ```<attr> (for example, ```rust,ignore -> ```ignore)
//! ```
//!
//! Non-Rust fences (such as `text`, `sh`, or bare fences) are left unchanged.

use crate::cargo_rustdoc_fmt::{constants::{CODE_FENCE_DELIMITER, FENCE_ATTR_COMMA,
                                           RUST_FENCE_PREFIX},
                               cst::DocNode,
                               types::FormatStatus};

/// If `fence_trimmed` (such as ```` ```rust ```` or ```` ```rust,ignore ````) has a
/// redundant `rust` language tag, returns the normalized opening fence (```` ``` ```` or
/// ```` ```ignore ````). Returns `None` if no change is needed.
/// 
/// Anchored to [`RUST_FENCE_PREFIX`] so tags like ```` ```rustic ```` or ```` ```text ````
/// do not match.
fn normalize_opening_fence_str(fence_trimmed: &str) -> Option<String> {
    let after_rust = fence_trimmed.strip_prefix(RUST_FENCE_PREFIX)?;
    if after_rust.is_empty() {
        Some(CODE_FENCE_DELIMITER.to_string())
    } else if let Some(attr) = after_rust.strip_prefix(FENCE_ATTR_COMMA)
        && !attr.is_empty()
    {
        Some(format!("{CODE_FENCE_DELIMITER}{attr}"))
    } else {
        None
    }
}

/// Normalizes the opening fence line of a [`DocNode::CodeFence`] if it carries a
/// redundant `rust` tag.
///
/// Preserves the leading comment prefix and indentation on `lines[0]` and sets
/// `*status = FormatStatus::Modified` when a replacement occurs.
/// Returns [`FormatStatus::Modified`] if the node was modified, or
/// [`FormatStatus::Unchanged`] otherwise.
pub fn normalize_code_fence_node(node: &mut DocNode) -> FormatStatus {
    if let DocNode::CodeFence { lines, status, .. } = node
        && let Some(first_line) = lines.first_mut()
        && let Some(fence_idx) = first_line.find(CODE_FENCE_DELIMITER)
    {
        let prefix = &first_line[..fence_idx];
        let fence_trimmed = first_line[fence_idx..].trim_end();
        if let Some(normalized_fence) = normalize_opening_fence_str(fence_trimmed) {
            *first_line = format!("{prefix}{normalized_fence}");
            *status = FormatStatus::Modified;
            return FormatStatus::Modified;
        }
    }
    FormatStatus::Unchanged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cargo_rustdoc_fmt::{FileChunk, SourceFileCst};

    #[test]
    fn test_normalize_opening_fence_str_cases() {
        assert_eq!(
            normalize_opening_fence_str("```rust"),
            Some("```".to_string())
        );
        assert_eq!(
            normalize_opening_fence_str("```rust,ignore"),
            Some("```ignore".to_string())
        );
        assert_eq!(
            normalize_opening_fence_str("```rust,no_run"),
            Some("```no_run".to_string())
        );
        // Trailing comma without attribute or non-rust tags must not match.
        assert_eq!(normalize_opening_fence_str("```rust,"), None);
        assert_eq!(normalize_opening_fence_str("```rustic"), None);
        assert_eq!(normalize_opening_fence_str("```text"), None);
        assert_eq!(normalize_opening_fence_str("```"), None);
    }

    #[test]
    fn test_cst_node_mutation_modified_and_unchanged() {
        let source = "/// ```rust\n/// let a = 1;\n/// ```\n///\n/// ```rust,ignore\n/// let b = 2;\n/// ```\n///\n/// ```text\n/// plain text\n/// ```\n///\n/// ```\n/// let c = 3;\n/// ```\nfn demo() {}\n";
        let mut cst = SourceFileCst::parse(source);

        let FileChunk::DocBlock(ref mut block) = cst.chunks[0] else {
            unreachable!();
        };

        // Node 0: ```rust -> Modified.
        assert_eq!(
            normalize_code_fence_node(&mut block.nodes[0]),
            FormatStatus::Modified
        );
        assert_eq!(block.nodes[0].status(), FormatStatus::Modified);

        // Idempotency: second normalization on already normalized node returns
        // Unchanged.
        assert_eq!(
            normalize_code_fence_node(&mut block.nodes[0]),
            FormatStatus::Unchanged
        );

        // Node 2: ```rust,ignore -> Modified.
        assert_eq!(
            normalize_code_fence_node(&mut block.nodes[2]),
            FormatStatus::Modified
        );

        // Node 4: ```text -> Unchanged.
        assert_eq!(
            normalize_code_fence_node(&mut block.nodes[4]),
            FormatStatus::Unchanged
        );
        assert_eq!(block.nodes[4].status(), FormatStatus::Unchanged);

        // Node 6: bare ``` -> Unchanged.
        assert_eq!(
            normalize_code_fence_node(&mut block.nodes[6]),
            FormatStatus::Unchanged
        );
        assert_eq!(block.nodes[6].status(), FormatStatus::Unchanged);

        let expected = "/// ```\n/// let a = 1;\n/// ```\n///\n/// ```ignore\n/// let b = 2;\n/// ```\n///\n/// ```text\n/// plain text\n/// ```\n///\n/// ```\n/// let c = 3;\n/// ```\nfn demo() {}\n";
        assert_eq!(cst.reconstruct(), expected);
    }
}
