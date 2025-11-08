// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extract rustdoc comment blocks from Rust source code.

use crate::cargo_rustdoc_fmt::types::{CommentType, RustdocBlock};

/// Extract all rustdoc comment blocks from source code.
///
/// Returns blocks for both `///` (outer) and `//!` (inner) style comments.
#[must_use] 
pub fn extract_rustdoc_blocks(source: &str) -> Vec<RustdocBlock> {
    let mut blocks = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        if let Some(block) = try_extract_block(&lines, &mut i) {
            blocks.push(block);
        } else {
            i += 1;
        }
    }

    blocks
}

/// Try to extract a rustdoc block starting at the current line.
fn try_extract_block(lines: &[&str], index: &mut usize) -> Option<RustdocBlock> {
    let line = lines[*index];

    // Detect comment type and indentation
    let (comment_type, comment_marker, indentation) = detect_rustdoc_comment(line)?;

    let start_line = *index;
    let mut block_lines = Vec::new();

    // Collect consecutive rustdoc lines
    while *index < lines.len() {
        let current_line = lines[*index];

        // Check if this line continues the block
        if let Some(content) = extract_comment_content(current_line, &comment_marker, &indentation)
        {
            block_lines.push(content.to_string());
            *index += 1;
        } else {
            // End of block (blank lines without marker end the block)
            break;
        }
    }

    if block_lines.is_empty() {
        return None;
    }

    Some(RustdocBlock {
        comment_type,
        start_line,
        end_line: *index - 1,
        lines: block_lines,
        indentation,
    })
}

/// Detect if a line is a rustdoc comment and return its type and indentation.
fn detect_rustdoc_comment(line: &str) -> Option<(CommentType, String, String)> {
    let trimmed = line.trim_start();
    let indentation = line[..line.len() - trimmed.len()].to_string();

    if trimmed.starts_with("//!") {
        Some((CommentType::Inner, "//!".to_string(), indentation))
    } else if trimmed.starts_with("///") {
        Some((CommentType::Outer, "///".to_string(), indentation))
    } else {
        None
    }
}

/// Extract comment content, removing the marker and leading spaces.
fn extract_comment_content<'a>(
    line: &'a str,
    marker: &str,
    _expected_indent: &str,
) -> Option<&'a str> {
    let trimmed = line.trim_start();

    if !trimmed.starts_with(marker) {
        return None;
    }

    let after_marker = &trimmed[marker.len()..];

    // Remove leading space if present
    if let Some(stripped) = after_marker.strip_prefix(' ') {
        Some(stripped)
    } else {
        Some(after_marker)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_outer_comments() {
        let source = "/// This is a doc comment\n/// With multiple lines\nfn foo() {}";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].comment_type, CommentType::Outer);
        assert_eq!(blocks[0].lines.len(), 2);
    }

    #[test]
    fn test_extract_inner_comments() {
        let source = "//! Module documentation\n//! Continued here\n\nfn foo() {}";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].comment_type, CommentType::Inner);
    }

    #[test]
    fn test_preserves_indentation() {
        let source = "    /// Indented comment";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].indentation, "    ");
    }

    #[test]
    fn test_handles_empty_lines() {
        let source = "/// First\n///\n/// Third";
        let blocks = extract_rustdoc_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lines.len(), 3);
    }
}
