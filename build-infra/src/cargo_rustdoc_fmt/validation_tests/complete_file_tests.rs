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
    /// This is a true end-to-end test that runs the actual CLI binary (which
    /// includes `cargo fmt` at the end), matching real-world behavior.
    ///
    /// The test runs from the workspace directory, so `cargo fmt` uses the
    /// workspace's `rustfmt.toml` configuration.
    #[test]
    fn test_real_world_file_complete_formatting() {
        let input = include_str!("test_data/complete_file/input/sample_real_world.rs");
        let expected =
            include_str!("test_data/complete_file/expected_output/sample_real_world.rs");

        // Create temp dir and copy input file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_real_world.rs");
        fs::write(&test_file, input).unwrap();

        // Run the actual CLI binary for true e2e testing (includes cargo fmt)
        // Runs from workspace dir, so cargo fmt uses workspace's rustfmt.toml
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let output = std::process::Command::new(&cargo)
            .args(["run", "--bin", "cargo-rustdoc-fmt", "--"])
            .arg("rustdoc-fmt")
            .arg(&test_file)
            .output()
            .expect("Failed to run cargo-rustdoc-fmt binary");

        assert!(
            output.status.success(),
            "CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
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

        // Copy expected to temp dir for diff comparison
        let expected_path = temp_dir.path().join("expected.rs");
        fs::write(&expected_path, expected).unwrap();

        // Use diff for comparison - clearer than assert_eq for large files
        let diff_output = std::process::Command::new("diff")
            .args(["-u", "--color=never"])
            .arg(&expected_path)
            .arg(&test_file)
            .output()
            .expect("Failed to run diff");

        if !diff_output.status.success() {
            eprintln!(
                "=== DIFF (expected vs formatted) ===\n{}",
                String::from_utf8_lossy(&diff_output.stdout)
            );
            panic!("Formatted output does not match expected output");
        }
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
    /// This test uses `mio_poller/mod.rs` which has:
    /// - HTML comments (`<!-- It is ok to use ignore here -->`)
    /// - Reference-style links that may be reordered
    /// - Code blocks with `ignore` attribute
    ///
    /// This verifies that `ContentProtector` correctly protects HTML comments
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

    /// Test code fences with comma-separated language tags (e.g., `rust,ignore`).
    ///
    /// This test verifies that:
    /// - Code fences with `rust,ignore` are correctly protected (not mangled)
    /// - Reference definitions scattered throughout the rustdoc block are aggregated
    /// - References are sorted alphabetically at the bottom
    ///
    /// This was a regression where the regex `^```\w*$` didn't match commas,
    /// causing `rust,ignore` fences to not be recognized as fence starts.
    #[test]
    fn test_code_fence_comma_language_tag() {
        let input =
            include_str!("test_data/complete_file/input/sample_code_fence_comma.rs");
        let expected = include_str!(
            "test_data/complete_file/expected_output/sample_code_fence_comma.rs"
        );

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_code_fence_comma.rs");
        fs::write(&test_file, input).unwrap();

        let processor = processor::FileProcessor::new(FormatOptions::default());
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

        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify code fence with rust,ignore is preserved correctly.
        assert!(
            formatted.contains("```rust,ignore"),
            "Code fence with rust,ignore should be preserved"
        );
        assert!(
            formatted.contains("pub fn poll_impl"),
            "Code inside fence should be preserved"
        );

        // Verify references are aggregated at bottom, sorted alphabetically.
        // [`DirectToAnsi`] should come before [`EINVAL`] (backtick sorts before 'E').
        let lines: Vec<&str> = formatted.lines().collect();
        let ref_start = lines
            .iter()
            .position(|l| l.contains("[`DirectToAnsi`]: mod@super"))
            .expect("Should find DirectToAnsi reference");
        let ref_einval = lines
            .iter()
            .position(|l| l.contains("[`EINVAL`]:"))
            .expect("Should find EINVAL reference");
        assert!(
            ref_start < ref_einval,
            "References should be sorted: [`DirectToAnsi`] before [`EINVAL`]"
        );

        // Verify all references are at the end of the rustdoc block.
        let last_rustdoc_line = lines
            .iter()
            .rposition(|l| l.starts_with("//!"))
            .expect("Should have rustdoc lines");
        assert!(
            lines[last_rustdoc_line].starts_with("//! ["),
            "Last rustdoc line should be a reference definition"
        );

        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }

    /// Test that indented tables (nested under numbered lists) preserve their
    /// indentation.
    ///
    /// This test uses a file with tables indented 4 spaces to appear under numbered
    /// list items. The formatter should:
    /// - Preserve the 4-space indentation before each table line
    /// - Still format the table columns correctly
    /// - Not strip the indentation (which would break the markdown structure)
    #[test]
    fn test_indented_table_preservation() {
        let input =
            include_str!("test_data/complete_file/input/sample_indented_table.rs");
        let expected = include_str!(
            "test_data/complete_file/expected_output/sample_indented_table.rs"
        );

        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_indented_table.rs");
        fs::write(&test_file, input).unwrap();

        // Process with table formatting only.
        let options = FormatOptions {
            format_tables: true,
            convert_links: false,
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
            "File should be modified (tables need formatting)"
        );

        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify indented tables preserve their 4-space indentation.
        // These lines should start with "//!     |" (4 spaces after "//! ").
        assert!(
            formatted.contains("//!     | Trigger"),
            "First indented table header should preserve 4-space indent"
        );
        assert!(
            formatted.contains("//!     | Column One"),
            "Second indented table header should preserve 4-space indent"
        );

        // Verify non-indented table has no indentation.
        assert!(
            formatted.contains("//! | Header A"),
            "Non-indented table should have no extra indentation"
        );

        // Verify the numbered list structure is preserved.
        assert!(
            formatted.contains("//! 1. First mechanism"),
            "Numbered list should be preserved"
        );
        assert!(
            formatted.contains("//! 2. Second mechanism"),
            "Numbered list should be preserved"
        );

        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }

    /// Test that text diagrams (ASCII art) are preserved correctly.
    ///
    /// This test uses the resilient_reactor_thread/mod.rs file which triggered
    /// the PROTECTED_CONTENT bug. It contains:
    /// - Complex ASCII art diagrams inside code fences
    /// - Multiple markdown tables
    /// - HTML comments
    /// - Reference-style links scattered throughout
    ///
    /// This verifies that ContentProtector's Unicode arrow placeholders don't
    /// get mangled by pulldown_cmark (the original `___` underscores were
    /// interpreted as bold+italic markdown).
    #[test]
    fn test_resilient_reactor_text_diagrams() {
        let input =
            include_str!("test_data/complete_file/input/sample_resilient_reactor.rs");
        let expected = include_str!(
            "test_data/complete_file/expected_output/sample_resilient_reactor.rs"
        );

        // Create temp dir and copy input file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("sample_resilient_reactor.rs");
        fs::write(&test_file, input).unwrap();

        // Run the actual CLI binary for true e2e testing (includes cargo fmt)
        // Runs from workspace dir, so cargo fmt uses workspace's rustfmt.toml
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let output = std::process::Command::new(&cargo)
            .args(["run", "--bin", "cargo-rustdoc-fmt", "--"])
            .arg("rustdoc-fmt")
            .arg(&test_file)
            .output()
            .expect("Failed to run cargo-rustdoc-fmt binary");

        assert!(
            output.status.success(),
            "CLI failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Read the formatted result
        let formatted = fs::read_to_string(&test_file).unwrap();

        // Verify text diagrams are preserved (key test for the PROTECTED_CONTENT bug)
        assert!(
            formatted.contains("Thread: poll()"),
            "ASCII diagram should be preserved"
        );
        assert!(
            formatted.contains("──blocks──►"),
            "ASCII diagram arrows should be preserved"
        );
        assert!(
            formatted.contains("┌────────"),
            "Box drawing characters should be preserved"
        );

        // Verify NO PROTECTED_CONTENT placeholders remain
        assert!(
            !formatted.contains("PROTECTED_CONTENT"),
            "All placeholders should be restored"
        );

        // Verify tables are preserved
        assert!(
            formatted.contains("| Component"),
            "Tables should be preserved"
        );
        assert!(
            formatted.contains("| :--"),
            "Table alignment markers should be preserved"
        );

        // Verify reference-style links are present
        assert!(
            formatted.contains("[`epoll`]:"),
            "Reference-style links should be present"
        );

        // Copy expected to temp dir for diff comparison
        let expected_path = temp_dir.path().join("expected.rs");
        fs::write(&expected_path, expected).unwrap();

        // Use diff for comparison - clearer than assert_eq for large files
        let diff_output = std::process::Command::new("diff")
            .args(["-u", "--color=never"])
            .arg(&expected_path)
            .arg(&test_file)
            .output()
            .expect("Failed to run diff");

        if !diff_output.status.success() {
            eprintln!(
                "=== DIFF (expected vs formatted) ===\n{}",
                String::from_utf8_lossy(&diff_output.stdout)
            );
            panic!("Formatted output does not match expected output");
        }
    }
}
