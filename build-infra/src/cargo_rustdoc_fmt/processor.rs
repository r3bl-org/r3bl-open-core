// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words Blockquotes

//! Orchestrate rustdoc formatting for files.

use crate::cargo_rustdoc_fmt::{extractor, link_converter, table_formatter,
                               types::{CommentType, FormatOptions, ProcessingResult,
                                       RustdocBlock}};
use std::path::{Path, PathBuf};

/// Check if a file contains the `rustfmt_skip` attribute.
///
/// Files with `#![cfg_attr(rustfmt, rustfmt_skip)]` will be skipped entirely
/// to respect the user's intent to preserve manual formatting.
fn has_rustfmt_skip(source: &str) -> bool {
    source.contains("#![cfg_attr(rustfmt, rustfmt_skip)]")
        || source.contains("#![ cfg_attr(rustfmt, rustfmt_skip) ]")
        || source.contains("#![ cfg_attr( rustfmt , rustfmt_skip ) ]")
}

/// Processes Rust files to format their rustdoc comments.
#[derive(Debug)]
pub struct FileProcessor {
    options: FormatOptions,
}

impl FileProcessor {
    /// Create a new file processor with the given options.
    #[must_use]
    pub fn new(options: FormatOptions) -> Self { Self { options } }

    /// Process a single file.
    #[must_use]
    pub fn process_file(&self, path: &Path) -> ProcessingResult {
        let mut result = ProcessingResult::new(path.to_path_buf());

        // Read file
        let source = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                result.add_error(format!("Failed to read file: {e}"));
                return result;
            }
        };

        // Skip files with rustfmt_skip attribute
        if has_rustfmt_skip(&source) {
            return result; // Return early, file unchanged
        }

        // Extract rustdoc blocks
        let mut blocks = extractor::extract_rustdoc_blocks(&source);

        // Process blocks
        let mut modified = false;
        for block in &mut blocks {
            if process_rustdoc_block(block, &self.options) {
                modified = true;
            }
        }

        // If modified, reconstruct and write
        if modified && !self.options.check_only {
            let new_source = reconstruct_source(&source, &blocks);
            if let Err(e) = std::fs::write(path, new_source) {
                result.add_error(format!("Failed to write file: {e}"));
            } else {
                result.mark_modified();
            }
        } else if modified {
            result.mark_modified();
        }

        result
    }

    /// Process multiple files.
    #[must_use]
    pub fn process_files(&self, paths: &[PathBuf]) -> Vec<ProcessingResult> {
        paths.iter().map(|p| self.process_file(p)).collect()
    }
}

/// Process a single rustdoc block, applying formatters.
/// Returns true if the block was modified.
fn process_rustdoc_block(block: &mut RustdocBlock, options: &FormatOptions) -> bool {
    let original = block.lines.join("\n");
    let mut modified = original.clone();

    // Table formatting is safe even with protected content (HTML comments, etc.)
    // Tables are self-contained and don't interact with link references.
    if options.format_tables {
        modified = table_formatter::format_tables(&modified);
    }

    // Link conversion should skip blocks with protected content because:
    // - HTML comments might contain special directives
    // - Blockquotes can be mangled by markdown parsers
    // - HTML tags will be corrupted
    if options.convert_links && !has_protected_content(&original) {
        modified = link_converter::convert_links(&modified);
        modified = link_converter::aggregate_existing_references(&modified);
    }

    if modified == original {
        false
    } else {
        block.lines = modified.lines().map(String::from).collect();
        true
    }
}

/// Check if text contains content that should not be modified.
///
/// Currently protects:
/// - HTML comments (`<!-- ... -->`) - used for cspell directives and code block
///   explanations
/// - HTML tags (will be mangled by markdown parsers)
/// - Blockquotes (will be removed by markdown parsers)
///
/// Note: Code fences are generally handled correctly by markdown parsers,
/// but if you have complex examples with reference-style links INSIDE code
/// fences (like documentation about the formatter itself), use
/// `#![cfg_attr(rustfmt, rustfmt_skip)]` to skip the entire file.
fn has_protected_content(text: &str) -> bool {
    // Check for HTML comments (e.g., <!-- cspell:disable -->, <!-- explanation -->)
    // These are used for spell-checker directives and to explain code block attributes
    if text.contains("<!--") {
        return true;
    }

    // Check for HTML tags (will be mangled by markdown parsers)
    if text.contains('<') && text.contains('>') {
        // Simple check for HTML-like content
        if text.contains("</")
            || text.contains("/>")
            || text.contains("style=")
            || text.contains("src=")
        {
            return true;
        }
    }

    // Check for blockquotes (will be removed by markdown parsers)
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('>') && !trimmed.starts_with(">=") {
            // It's a blockquote marker, not a comparison operator
            return true;
        }
    }

    false
}

/// Reconstruct source file with modified rustdoc blocks.
fn reconstruct_source(original: &str, blocks: &[RustdocBlock]) -> String {
    let original_lines: Vec<&str> = original.lines().collect();
    let mut result = String::new();
    let mut block_idx = 0;
    let mut line_idx = 0;

    while line_idx < original_lines.len() {
        if block_idx < blocks.len() && line_idx == blocks[block_idx].start_line {
            // Replace block lines
            let block = &blocks[block_idx];
            for (i, block_line) in block.lines.iter().enumerate() {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(&block.indentation);
                if block.comment_type == CommentType::Inner {
                    result.push_str("//!");
                } else {
                    result.push_str("///");
                }
                if !block_line.is_empty() {
                    result.push(' ');
                    result.push_str(block_line);
                }
            }
            result.push('\n');
            line_idx = block.end_line + 1;
            block_idx += 1;
        } else {
            result.push_str(original_lines[line_idx]);
            result.push('\n');
            line_idx += 1;
        }
    }

    // Remove trailing newline if original didn't have it
    if !original.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options);
        assert!(!processor.options.check_only);
    }

    #[test]
    fn test_process_nonexistent_file() {
        let options = FormatOptions::default();
        let processor = FileProcessor::new(options);
        let result = processor.process_file(Path::new("/nonexistent/file.rs"));
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_has_protected_content_html_comments() {
        // cspell disable/enable pairs
        assert!(has_protected_content("<!-- cspell:disable -->"));
        assert!(has_protected_content("<!-- cspell:enable -->"));

        // Explanation comments before code blocks
        assert!(has_protected_content(
            "<!-- It is ok to use ignore here - demonstrates usage -->"
        ));

        // Multi-line content with HTML comment
        let text_with_comment = r"Some text
<!-- cspell:disable -->
[`SomeType`]: crate::SomeType
<!-- cspell:enable -->";
        assert!(has_protected_content(text_with_comment));

        // Regular content without HTML comments should not be protected
        assert!(!has_protected_content("Just regular text"));
        assert!(!has_protected_content("[link]: https://example.com"));
    }

    #[test]
    fn test_has_protected_content_html_tags() {
        // Closing tags
        assert!(has_protected_content("<span>text</span>"));
        // Self-closing tags
        assert!(has_protected_content("<br/>"));
        // Tags with attributes
        assert!(has_protected_content("<div style=\"color:red\">"));
        assert!(has_protected_content("<img src=\"img.png\">"));
    }

    #[test]
    fn test_has_protected_content_blockquotes() {
        assert!(has_protected_content("> This is a quote"));
        assert!(has_protected_content("text\n> quote"));
        // Comparison operators should not trigger
        assert!(!has_protected_content("if x >= 5"));
    }
}
