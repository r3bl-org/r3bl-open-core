// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Type definitions for rustdoc formatting.

use std::path::PathBuf;

/// Configuration options for formatting operations.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FormatOptions {
    /// Format markdown tables
    pub format_tables: bool,
    /// Convert inline links to reference-style
    pub convert_links: bool,
    /// Only check formatting, don't modify files
    pub check_only: bool,
    /// Print verbose output
    pub verbose: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            format_tables: true,
            convert_links: true,
            check_only: false,
            verbose: false,
        }
    }
}

/// Result of processing a single file.
#[derive(Debug)]
pub struct ProcessingResult {
    /// Path to the processed file
    pub file_path: PathBuf,
    /// Whether the file was modified
    pub modified: bool,
    /// Any errors encountered
    pub errors: Vec<String>,
}

impl ProcessingResult {
    /// Create a new processing result.
    #[must_use]
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            modified: false,
            errors: Vec::new(),
        }
    }

    /// Mark this result as modified.
    pub fn mark_modified(&mut self) { self.modified = true; }

    /// Add an error to this result.
    pub fn add_error(&mut self, error: String) { self.errors.push(error); }
}

/// Type of rustdoc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentType {
    /// Inner doc comment: `//!`
    Inner,
    /// Outer doc comment: `///`
    Outer,
}

/// A block of rustdoc comments extracted from source code.
#[derive(Debug, Clone)]
pub struct RustdocBlock {
    /// Type of comment (`///` or `//!`)
    pub comment_type: CommentType,
    /// Starting line number (0-indexed)
    pub start_line: usize,
    /// Ending line number (0-indexed, inclusive)
    pub end_line: usize,
    /// Content lines (without comment markers or indentation)
    pub lines: Vec<String>,
    /// Original indentation to preserve
    pub indentation: String,
}

/// Result type for formatter operations.
pub type FormatterResult<T> = miette::Result<T>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_options_default() {
        let opts = FormatOptions::default();
        assert!(opts.format_tables);
        assert!(opts.convert_links);
        assert!(!opts.check_only);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_processing_result() {
        let mut result = ProcessingResult::new(PathBuf::from("test.rs"));
        assert!(!result.modified);
        assert!(result.errors.is_empty());

        result.mark_modified();
        assert!(result.modified);

        result.add_error("test error".to_string());
        assert_eq!(result.errors.len(), 1);
    }
}
