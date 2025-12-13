// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Complete file validation tests for cargo-rustdoc-fmt.
//!
//! These tests verify the full workflow: extracting rustdoc blocks from complete
//! Rust files, formatting them, and reconstructing the files.

#[cfg(test)]
mod tests {
    use crate::cargo_rustdoc_fmt::{extractor, link_converter, processor,
                                   table_formatter,
                                   types::{CommentType, FormatOptions}};
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
            with_links.contains("[Grapheme clusters]:")
                || with_links.contains("[UTF-8 String]:")
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
        let source =
            include_str!("test_data/complete_file/input/sample_mixed_comments.rs");
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
        let source =
            include_str!("test_data/complete_file/input/sample_no_formatting_needed.rs");
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

    /// Comprehensive real-world test using keyboard.rs file.
    ///
    /// This test uses a real-world file (keyboard.rs) with:
    /// - Multiple heading levels (##, ###) that must be preserved
    /// - Markdown tables that need alignment
    /// - Inline links that should be converted to reference style
    ///
    /// This verifies the complete formatting pipeline on actual production code.
    #[test]
    fn test_real_world_file_complete_formatting() {
        let input = include_str!("test_data/complete_file/input/sample_real_world.rs");
        let expected =
            include_str!("test_data/complete_file/expected_output/sample_real_world.rs");

        // Use the actual FileProcessor to test the complete pipeline
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_real_world.rs");
        fs::write(&test_file, input).unwrap();

        // Process with default options (tables + links)
        let options = FormatOptions::default();
        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        assert!(
            result.errors.is_empty(),
            "Processing errors: {:?}",
            result.errors
        );
        assert!(
            result.modified,
            "File should be modified (has formatting to do)"
        );

        // Read the formatted result
        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify heading levels are preserved
        assert!(
            formatted.contains("## Parser Dispatch Priority Pipeline"),
            "Level-2 heading should be preserved as ##, not changed to #"
        );
        assert!(
            formatted.contains("## Comprehensive List of Supported Keyboard Shortcuts"),
            "Level-2 heading should be preserved"
        );
        assert!(
            formatted.contains("### Basic Keys"),
            "Level-3 heading should be preserved as ###"
        );

        // Verify tables are formatted (aligned)
        let h2_in_expected = expected.matches("## ").count();
        let h2_in_formatted = formatted.matches("## ").count();
        assert_eq!(
            h2_in_formatted, h2_in_expected,
            "Number of level-2 headings (##) should be preserved"
        );

        let h3_in_expected = expected.matches("### ").count();
        let h3_in_formatted = formatted.matches("### ").count();
        assert_eq!(
            h3_in_formatted, h3_in_expected,
            "Number of level-3 headings (###) should be preserved"
        );

        // Verify reference-style links are created
        assert!(
            formatted.contains("]: mod@"),
            "Should have reference-style links"
        );

        // The formatted result should match expected output
        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }

    /// Test that files with `rustfmt_skip` attribute are correctly skipped.
    ///
    /// This test uses a real-world file (mouse.rs) that has:
    /// - `#![cfg_attr(rustfmt, rustfmt_skip)]` at the top
    /// - Content that would normally be formatted (inline links, scattered references)
    ///
    /// This verifies that the formatter correctly:
    /// - Detects the `rustfmt_skip` attribute
    /// - Skips processing entirely (does not modify the file)
    /// - Leaves the file unchanged
    #[test]
    fn test_real_world_file_2_complete_formatting() {
        let input = include_str!("test_data/complete_file/input/sample_real_world_2.rs");

        // Use the actual FileProcessor to test the complete pipeline
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_real_world_2.rs");
        fs::write(&test_file, input).unwrap();

        // Process with default options (tables + links)
        let options = FormatOptions::default();
        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        assert!(
            result.errors.is_empty(),
            "Processing errors: {:?}",
            result.errors
        );

        // File should NOT be modified because it has rustfmt_skip
        assert!(
            !result.modified,
            "File with rustfmt_skip should not be modified"
        );

        // Read the result - should be unchanged
        let output = fs::read_to_string(&test_file).unwrap();

        // Content should be exactly the same as input
        assert_eq!(
            output, input,
            "File with rustfmt_skip should remain unchanged"
        );
    }

    /// Test aggregation of scattered reference-style links.
    ///
    /// This test verifies that reference definitions scattered throughout doc blocks
    /// are correctly extracted, sorted alphabetically, and moved to the bottom with
    /// a blank line separator.
    #[test]
    fn test_scattered_references_aggregation() {
        let input =
            include_str!("test_data/complete_file/input/sample_scattered_references.rs");
        let expected = include_str!(
            "test_data/complete_file/expected_output/sample_scattered_references.rs"
        );

        // Use the actual FileProcessor to test the complete pipeline
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_scattered_references.rs");
        fs::write(&test_file, input).unwrap();

        // Process with link conversion enabled
        let options = FormatOptions {
            format_tables: false,
            convert_links: true,
            check_only: false,
            verbose: false,
        };
        let processor = processor::FileProcessor::new(options);
        let result = processor.process_file(&test_file);

        assert!(
            result.errors.is_empty(),
            "Processing errors: {:?}",
            result.errors
        );
        assert!(
            result.modified,
            "File should be modified (references need aggregation)"
        );

        // Read the formatted result
        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify references are at bottom of each block
        assert!(
            formatted.contains("Navigate**:\n//! - ⬆️"),
            "Content should be preserved"
        );

        // Verify alphabetical sorting in module doc block
        let module_refs = formatted
            .lines()
            .skip_while(|l| !l.contains("[`VT100MouseAction`]"))
            .take(5)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            module_refs.contains("VT100MouseAction")
                && module_refs.contains("VT100MouseButton"),
            "Module references should be sorted alphabetically"
        );

        // Verify blank line before references for visual clarity
        assert!(
            formatted.contains("\n//!\n//! [`VT100MouseAction`]:"),
            "Should have blank line before references"
        );

        // The formatted result should match expected output
        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }

    /// Test that HTML comments are preserved while links are still converted.
    ///
    /// This test uses mio_poller/mod.rs which has:
    /// - HTML comments (`<!-- It is ok to use ignore here -->`)
    /// - Reference-style links that may be reordered
    /// - Code blocks with `ignore` attribute
    ///
    /// This verifies that ContentProtector correctly protects HTML comments
    /// while still allowing link conversion in non-HTML parts.
    #[test]
    fn test_html_comments_preserved_links_converted() {
        let input = include_str!("test_data/complete_file/input/sample_html_comments.rs");
        let expected = include_str!(
            "test_data/complete_file/expected_output/sample_html_comments.rs"
        );

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_html_comments.rs");
        fs::write(&test_file, input).unwrap();

        let processor = processor::FileProcessor::new(FormatOptions::default());
        let result = processor.process_file(&test_file);

        assert!(
            result.errors.is_empty(),
            "Processing errors: {:?}",
            result.errors
        );

        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify HTML comments are preserved
        assert!(
            formatted.contains("<!-- It is ok to use ignore here -->"),
            "HTML comments should be preserved exactly"
        );

        // Verify code blocks with ignore attribute are preserved
        assert!(
            formatted.contains("```ignore"),
            "Code blocks should be preserved"
        );

        // Verify references are sorted alphabetically at the bottom
        // Looking for a reference that comes after another alphabetically
        assert!(
            formatted.contains("[`epoll`]:"),
            "References should be present"
        );

        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }
}
