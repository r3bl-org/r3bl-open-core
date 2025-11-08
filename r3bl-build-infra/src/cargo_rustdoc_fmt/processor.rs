// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Orchestrate rustdoc formatting for files.

use crate::cargo_rustdoc_fmt::{
    extractor, link_converter, table_formatter,
    types::{CommentType, FormatOptions, ProcessingResult, RustdocBlock},
};
use std::path::{Path, PathBuf};

/// Processes Rust files to format their rustdoc comments.
#[derive(Debug)]
pub struct FileProcessor {
    options: FormatOptions,
}

impl FileProcessor {
    /// Create a new file processor with the given options.
    #[must_use] 
    pub fn new(options: FormatOptions) -> Self {
        Self { options }
    }

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

    if options.format_tables {
        modified = table_formatter::format_tables(&modified);
    }

    if options.convert_links {
        modified = link_converter::convert_links(&modified);
    }

    if modified == original {
        false
    } else {
        block.lines = modified.lines().map(String::from).collect();
        true
    }
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
}
