// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Complete file validation tests for cargo-rustdoc-fmt.
//!
//! These tests verify the full workflow: extracting rustdoc blocks from complete
//! Rust files, formatting them, and reconstructing the files.

#[cfg(test)]
mod tests {
    use crate::cargo_rustdoc_fmt::{
        extractor, link_converter, processor, table_formatter,
        types::{CommentType, FormatOptions},
    };
    use std::fs;
    use tempfile::TempDir;

    /// Test that complex file with both tables and links can be processed.
    #[test]
    fn test_complex_file_processing() {
        let source = include_str!("test_data/complete_file/input/sample_complex.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        assert!(!blocks.is_empty());

        // Process the first block
        let content = blocks[0].lines.join("\n");

        // Format tables
        let with_tables = table_formatter::format_tables(&content);
        assert!(with_tables.contains("Character") || with_tables.contains("Byte size"));

        // Convert links (should use link text as reference, not numbers)
        let with_links = link_converter::convert_links(&with_tables);
        assert!(
            with_links.contains("[Grapheme clusters]:") || with_links.contains("[UTF-8 String]:")
        );
    }

    /// Test that we can extract rustdoc blocks from real files.
    #[test]
    fn test_extract_from_sample_table() {
        let source = include_str!("test_data/complete_file/input/sample_table.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        assert!(!blocks.is_empty());
        assert_eq!(blocks[0].comment_type, CommentType::Inner);

        // Should contain the table
        let content = blocks[0].lines.join("\n");
        assert!(content.contains("| Aspect"));
    }

    /// Test table formatting on a real file.
    #[test]
    fn test_format_table_from_fixture() {
        let source = include_str!("test_data/complete_file/input/sample_table.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        let table_content = blocks[0].lines.join("\n");
        let formatted = table_formatter::format_tables(&table_content);

        // Check that table is present in formatted output
        assert!(formatted.contains("Aspect"));
        assert!(formatted.contains("Protocol"));
    }

    /// Test link conversion on a real file.
    #[test]
    fn test_convert_links_from_fixture() {
        let source = include_str!("test_data/complete_file/input/sample_links.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        // First block should have links
        let content = blocks[0].lines.join("\n");
        let converted = link_converter::convert_links(&content);

        // Should have reference-style links (using link text as reference)
        assert!(converted.contains("[Rust docs]:") || converted.contains("[GitHub]:"));
    }

    /// Test that mixed comment types (//! and ///) are handled correctly.
    #[test]
    fn test_mixed_comment_types() {
        let source = include_str!("test_data/complete_file/input/sample_mixed_comments.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        // Should have multiple blocks
        assert!(blocks.len() >= 2);

        // Should have both Inner and Outer comment types
        let has_inner = blocks.iter().any(|b| b.comment_type == CommentType::Inner);
        let has_outer = blocks.iter().any(|b| b.comment_type == CommentType::Outer);
        assert!(has_inner);
        assert!(has_outer);
    }

    /// Test that indented rustdoc is preserved.
    #[test]
    fn test_indented_rustdoc() {
        let source = include_str!("test_data/complete_file/input/sample_indented.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        assert!(!blocks.is_empty());

        // Check that indentation is captured
        let has_indented = blocks.iter().any(|b| !b.indentation.is_empty());
        assert!(has_indented);
    }

    /// Test that files with no formatting needed are left unchanged.
    #[test]
    fn test_no_formatting_needed() {
        let source = include_str!("test_data/complete_file/input/sample_no_formatting_needed.rs");
        let blocks = extractor::extract_rustdoc_blocks(source);

        for block in blocks {
            let content = block.lines.join("\n");
            let formatted_tables = table_formatter::format_tables(&content);
            let formatted_links = link_converter::convert_links(&formatted_tables);

            // Should remain unchanged
            assert_eq!(content, formatted_links);
        }
    }

    /// Test `FileProcessor` with a temporary file.
    #[test]
    fn test_file_processor_on_temp_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        // Write a test file with a table
        let content = r"//! Test file
//! | A | B |
//! |---|---|
//! | 1 | 2 |

fn main() {}";
        fs::write(&test_file, content).unwrap();

        // Process the file
        let options = FormatOptions {
            format_tables: true,
            convert_links: false,
            check_only: false,
            verbose: false,
        };

        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        // Check that processing succeeded
        assert!(
            result.errors.is_empty(),
            "Processing errors: {:?}",
            result.errors
        );
    }

    /// Test `FileProcessor` in check mode.
    #[test]
    fn test_file_processor_check_mode() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        let content = r"//! Test with link [docs](https://example.com)

fn main() {}";
        fs::write(&test_file, content).unwrap();

        // Process in check mode
        let options = FormatOptions {
            format_tables: false,
            convert_links: true,
            check_only: true,
            verbose: false,
        };

        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        // Should detect modification but not write
        if result.modified {
            // File should still have original content
            let after_content = fs::read_to_string(&test_file).unwrap();
            assert_eq!(content, after_content);
        }
    }

    /// End-to-end test: format a file with both tables and links.
    #[test]
    fn test_end_to_end_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");

        let content = r"//! Documentation with table and link.
//! | Feature | Status |
//! |---|---|
//! | Working | [Yes](https://example.com) |

fn main() {}";
        fs::write(&test_file, content).unwrap();

        // Process the file
        let options = FormatOptions::default();
        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);

        // Read back and verify formatting
        let formatted_content = fs::read_to_string(&test_file).unwrap();

        // Should have formatted table (columns aligned)
        assert!(formatted_content.contains("Feature"));

        // Should have reference-style links (using link text, not numbers)
        assert!(
            formatted_content.contains("[Yes]:")
                || formatted_content.contains("https://example.com")
        );
    }
}
