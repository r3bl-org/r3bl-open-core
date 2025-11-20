// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Protect content that should not be modified during formatting.
//!
//! This module handles preservation of:
//! - HTML blocks (inline and multi-line)
//! - Code fences (triple backticks with language tags like `rust`, `text`, `ignore`,
//!   `no_run`, etc.)
//! - Blockquotes (lines starting with `>`)
//!
//! The protection works by:
//! 1. Extracting protected content and replacing with placeholders
//! 2. Processing the text with placeholders
//! 3. Restoring the original protected content

use regex::Regex;
use std::sync::LazyLock;

/// Placeholder prefix used to mark protected content
const PLACEHOLDER_PREFIX: &str = "___PROTECTED_CONTENT_";
const PLACEHOLDER_SUFFIX: &str = "___";

/// Regular expressions for detecting protected content
static HTML_TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<[^>]+>").expect("Invalid HTML regex"));

static CODE_FENCE_START_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^```\w*$").expect("Invalid code fence start regex"));

/// Protected content and its placeholder
#[derive(Debug, Clone)]
struct ProtectedRegion {
    placeholder: String,
    original: String,
}

/// Context for protecting and restoring content
#[derive(Debug)]
pub struct ContentProtector {
    regions: Vec<ProtectedRegion>,
}

impl ContentProtector {
    /// Create a new content protector
    #[must_use]
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Protect content in the text, returning text with placeholders.
    ///
    /// Protects entire lines that contain:
    /// - HTML tags
    /// - Code fence blocks
    /// - Blockquotes
    #[must_use]
    pub fn protect(&mut self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut result = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check for code fence start
            if CODE_FENCE_START_REGEX.is_match(line.trim()) {
                let mut fence_lines = vec![line];
                i += 1;

                // Collect all lines until closing fence
                while i < lines.len() {
                    let fence_line = lines[i];
                    fence_lines.push(fence_line);

                    if fence_line.trim() == "```" {
                        i += 1;
                        break;
                    }
                    i += 1;
                }

                // Protect the entire code fence block as one unit
                let original = fence_lines.join("\n");
                let placeholder = self.create_placeholder(&original);
                result.push(placeholder);
                continue;
            }

            // Check if line starts with > (blockquote marker)
            let trimmed = line.trim_start();
            if trimmed.starts_with('>') {
                let mut blockquote_lines = vec![line];
                i += 1;

                // Collect consecutive blockquote lines
                while i < lines.len() {
                    let next_line = lines[i];
                    let next_trimmed = next_line.trim_start();

                    if next_trimmed.starts_with('>') || next_trimmed.is_empty() {
                        blockquote_lines.push(next_line);
                        i += 1;

                        // Stop at empty line (end of blockquote)
                        if next_trimmed.is_empty() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let original = blockquote_lines.join("\n");
                let placeholder = self.create_placeholder(&original);
                result.push(placeholder);
                continue;
            }

            // Check if line contains HTML tags - protect entire line
            if HTML_TAG_REGEX.is_match(line) {
                let placeholder = self.create_placeholder(line);
                result.push(placeholder);
                i += 1;
                continue;
            }

            // No protection needed for this line
            result.push(line.to_string());
            i += 1;
        }

        result.join("\n")
    }

    /// Restore protected content from placeholders
    #[must_use]
    pub fn restore(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Restore in reverse order to handle nested content correctly
        for region in self.regions.iter().rev() {
            result = result.replace(&region.placeholder, &region.original);
        }

        result
    }

    /// Create a unique placeholder for protected content
    fn create_placeholder(&mut self, original: &str) -> String {
        let index = self.regions.len();
        let placeholder = format!("{PLACEHOLDER_PREFIX}{index}{PLACEHOLDER_SUFFIX}");

        self.regions.push(ProtectedRegion {
            placeholder: placeholder.clone(),
            original: original.to_string(),
        });

        placeholder
    }
}

impl Default for ContentProtector {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protect_code_fence() {
        let mut protector = ContentProtector::new();
        let input = "Some text\n```rust\nlet x = 5;\n```\nMore text";
        let protected = protector.protect(input);

        // Should replace code fence with placeholder
        assert!(protected.contains("___PROTECTED_CONTENT_"));
        assert!(!protected.contains("let x = 5"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_html() {
        let mut protector = ContentProtector::new();
        let input = "Text <span style=\"color:red\">red</span> more";
        let protected = protector.protect(input);

        // Should replace HTML with placeholders
        assert!(protected.contains("___PROTECTED_CONTENT_"));
        assert!(!protected.contains("<span"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_blockquote() {
        let mut protector = ContentProtector::new();
        let input = "Text\n> Note: This is a note\n> Continued\nMore text";
        let protected = protector.protect(input);

        // Should replace blockquote with placeholder
        assert!(protected.contains("___PROTECTED_CONTENT_"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_multiple_types() {
        let mut protector = ContentProtector::new();
        let input = "Text\n```rust\ncode\n```\n<span>html</span>\n> quote\nEnd";
        let protected = protector.protect(input);

        // Should have multiple placeholders
        assert!(protected.matches("___PROTECTED_CONTENT_").count() >= 2);

        // Should restore all
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_no_protection_needed() {
        let mut protector = ContentProtector::new();
        let input = "Just plain text\nwith multiple lines";
        let protected = protector.protect(input);

        assert_eq!(protected, input);
        assert_eq!(protector.restore(&protected), input);
    }
}
