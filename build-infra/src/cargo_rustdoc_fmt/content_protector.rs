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

static CODE_FENCE_START_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Matches ``` optionally followed by language spec (e.g., rust, rust,ignore, text)
    // Uses [^\s]* to match any non-whitespace, handling commas in rust,ignore etc.
    Regex::new(r"^```[^\s]*$").expect("Invalid code fence start regex")
});

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

            // Check for HTML comment start
            if line.contains("<!--") {
                // If comment closes on same line, protect just this line
                if line.contains("-->") {
                    let placeholder = self.create_placeholder(line);
                    result.push(placeholder);
                    i += 1;
                    continue;
                }

                // Multi-line comment: collect until closing -->
                let mut comment_lines = vec![line];
                i += 1;
                while i < lines.len() {
                    let comment_line = lines[i];
                    comment_lines.push(comment_line);
                    if comment_line.contains("-->") {
                        i += 1;
                        break;
                    }
                    i += 1;
                }

                let original = comment_lines.join("\n");
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
    fn test_protect_code_fence_with_comma_language_tag() {
        // Tests language tags with commas (e.g., rust,ignore, rust,no_run).
        // These are common in rustdoc and must be correctly recognized as fence starts.
        let mut protector = ContentProtector::new();
        let input =
            "Before\n```rust,ignore\nfn example() {}\n```\nAfter [ref].\n[ref]: target";
        let protected = protector.protect(input);

        // Should replace code fence with placeholder.
        assert!(
            protected.contains("___PROTECTED_CONTENT_"),
            "Code fence should be protected"
        );
        assert!(
            !protected.contains("fn example()"),
            "Code inside fence should be hidden"
        );

        // Content after fence should NOT be protected.
        assert!(
            protected.contains("After [ref]."),
            "Content after fence should remain visible"
        );
        assert!(
            protected.contains("[ref]: target"),
            "Reference after fence should remain visible"
        );

        // Should restore original.
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_code_fence_various_language_tags() {
        // Test various language tag formats that should all be recognized.
        let test_cases = [
            "```rust\ncode\n```",
            "```rust,ignore\ncode\n```",
            "```rust,no_run\ncode\n```",
            "```text\ncode\n```",
            "```\ncode\n```", // No language tag.
        ];

        for input in test_cases {
            let mut protector = ContentProtector::new();
            let protected = protector.protect(input);

            assert!(
                protected.contains("___PROTECTED_CONTENT_"),
                "Failed for input: {input}"
            );
            assert!(
                !protected.contains("code"),
                "Code should be hidden for input: {input}"
            );

            let restored = protector.restore(&protected);
            assert_eq!(restored, input, "Restore failed for input: {input}");
        }
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

    #[test]
    fn test_protect_html_comment_single_line() {
        let mut protector = ContentProtector::new();
        let input = "Text <!-- comment --> more";
        let protected = protector.protect(input);

        // Should replace HTML comment line with placeholder
        assert!(protected.contains("___PROTECTED_CONTENT_"));
        assert!(!protected.contains("<!--"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_html_comment_multiline() {
        let mut protector = ContentProtector::new();
        let input = "Text\n<!--\nMulti-line\ncomment\n-->\nMore text";
        let protected = protector.protect(input);

        // Should replace HTML comment with placeholder
        assert!(protected.contains("___PROTECTED_CONTENT_"));
        assert!(!protected.contains("<!--"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_protect_html_comment_with_links() {
        let mut protector = ContentProtector::new();
        // Links inside HTML comments should be preserved unchanged
        let input = "See [link](url)\n<!-- This [link](url) should be preserved -->\nMore [text](url2)";
        let protected = protector.protect(input);

        // The lines with links (not in HTML comment) should remain
        assert!(protected.contains("See [link](url)"));
        assert!(protected.contains("More [text](url2)"));

        // The HTML comment line should be protected
        assert!(!protected.contains("<!-- This"));

        // Should restore original
        let restored = protector.restore(&protected);
        assert_eq!(restored, input);
    }
}
