// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// rustdoc-fmt: skip

//! Complete file validation tests for cargo-rustdoc-fmt.
//!
//! These tests verify the full workflow: parsing complete Rust files into a lossless
//! CST, formatting rustdoc blocks, and reconstructing the files.

// Allow doc_markdown warnings in this test file - the doc comments intentionally
// reference file names and identifiers without backticks as they describe test scenarios.
#![allow(clippy::doc_markdown)]

#[cfg(test)]
mod tests {
    use crate::cargo_rustdoc_fmt::{constants::{CRLF_LINE_ENDING, LF_LINE_ENDING,
                                               RUST_FILE_EXTENSION},
                                   formatters::TechnicalTermDictionary,
                                   processor,
                                   types::{FormatOptions, FormatStatus}};
    use std::{fs, path::Path};
    use tempfile::TempDir;

    /// Builds a deterministic registry from the embedded seed file for golden-file
    /// tests.
    fn test_registry() -> TechnicalTermDictionary {
        TechnicalTermDictionary::from_seed(None).unwrap()
    }

    /// Normalize CRLF to LF for cross-platform test compatibility.
    ///
    /// On Windows, `include_str!` loads files with `\r\n` (due to git [`autocrlf`]), but
    /// the formatter produces `\n` output. On macOS/Linux, files already use `\n` so this
    /// is a no-op early return.
    ///
    /// [`autocrlf`](https://docs.github.com/en/get-started/getting-started-with-git/configuring-git-to-handle-line-endings)
    fn normalize_line_endings(text: &str) -> String {
        if !text.contains(CRLF_LINE_ENDING) {
            return text.to_owned();
        }
        text.replace(CRLF_LINE_ENDING, LF_LINE_ENDING)
    }

    /// Runs `FileProcessor` with default options and the seed registry on `input`,
    /// writing to a temporary file so the git-tracked fixture on disk is not mutated.
    fn run_default_processor_on_fixture(
        file_name: &str,
        input: &str,
    ) -> (
        TempDir,
        std::path::PathBuf,
        processor::ProcessingResult,
        String,
    ) {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join(file_name);
        fs::write(&test_file, input).unwrap();

        let registry = test_registry();
        let processor =
            processor::FileProcessor::new(FormatOptions::default(), Some(&registry));
        let result = processor
            .process_files(std::slice::from_ref(&test_file))
            .pop()
            .unwrap();

        assert!(
            result.errors.is_empty(),
            "Processing errors for {file_name}: {:?}",
            result.errors
        );

        let formatted = fs::read_to_string(&test_file).unwrap();
        (temp_dir, test_file, result, formatted)
    }

    /// Runs `FileProcessor` 3 consecutive times on `input` and asserts that runs 2 and 3
    /// produce output identical to run 1.
    fn assert_fixture_idempotent(file_name: &str, input: &str) {
        let (_temp_dir, test_file, _result, after_run_1) =
            run_default_processor_on_fixture(file_name, input);

        let registry = test_registry();
        let processor =
            processor::FileProcessor::new(FormatOptions::default(), Some(&registry));

        let r2 = processor
            .process_files(std::slice::from_ref(&test_file))
            .pop()
            .unwrap();
        assert_eq!(r2.status, FormatStatus::Unchanged);
        let after_run_2 = fs::read_to_string(&test_file).unwrap();
        assert_eq!(
            after_run_1, after_run_2,
            "Run 2 should produce identical output to run 1 (idempotency)"
        );

        let r3 = processor
            .process_files(std::slice::from_ref(&test_file))
            .pop()
            .unwrap();
        assert_eq!(r3.status, FormatStatus::Unchanged);
        let after_run_3 = fs::read_to_string(&test_file).unwrap();
        assert_eq!(
            after_run_2, after_run_3,
            "Run 3 should produce identical output to run 2 (idempotency)"
        );
    }

    /// Compares `actual_file` against `expected` using `git diff --no-index` for
    /// readable diffs on large golden files across all platforms.
    fn assert_git_diff_eq(temp_dir: &TempDir, actual_file: &Path, expected: &str) {
        let expected_path = temp_dir.path().join("expected.rs");
        fs::write(&expected_path, expected).unwrap();

        let diff_output = std::process::Command::new("git")
            .args(["diff", "--no-index", "--color=never"])
            .arg(&expected_path)
            .arg(actual_file)
            .output()
            .expect("Failed to run git diff");

        if !diff_output.status.success() {
            eprintln!(
                "=== DIFF (expected vs formatted) ===\n{}",
                String::from_utf8_lossy(&diff_output.stdout)
            );
            panic!("Formatted output does not match expected output");
        }
    }

    /// Test that complex file with tables, inline links, and technical terms is
    /// formatted to match its golden file.
    #[test]
    fn test_complex_file_processing() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_complex.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_complex.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_complex.rs", input);

        assert_eq!(result.status, FormatStatus::Modified);
        assert_eq!(formatted, expected);
    }

    /// Test table formatting on `sample_table.rs` against its golden file.
    #[test]
    fn test_format_table_from_fixture() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_table.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_table.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_table.rs", input);

        assert_eq!(result.status, FormatStatus::Modified);
        assert_eq!(formatted, expected);
    }

    /// Test link conversion on `sample_links.rs` against its golden file.
    #[test]
    fn test_convert_links_from_fixture() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_links.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_links.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_links.rs", input);

        assert_eq!(result.status, FormatStatus::Modified);
        assert_eq!(formatted, expected);
    }

    /// Test that mixed comment types (`//!` and `///`) are formatted and match the
    /// golden file.
    #[test]
    fn test_mixed_comment_types() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_mixed_comments.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_mixed_comments.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_mixed_comments.rs", input);

        assert_eq!(result.status, FormatStatus::Modified);
        assert_eq!(formatted, expected);
    }

    /// Test that indented rustdoc blocks preserve their indentation and match the
    /// golden file.
    #[test]
    fn test_indented_rustdoc() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_indented.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_indented.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_indented.rs", input);

        assert_eq!(result.status, FormatStatus::Modified);
        assert_eq!(formatted, expected);
    }

    /// Test that files with no formatting needed are left unchanged and match the
    /// golden file.
    #[test]
    fn test_no_formatting_needed() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_no_formatting_needed.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_no_formatting_needed.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_no_formatting_needed.rs", input);

        assert_eq!(result.status, FormatStatus::Unchanged);
        assert_eq!(formatted, expected);
    }

    /// Comprehensive real-world test using keyboard.rs file.
    ///
    /// This test uses a real-world file (keyboard.rs) with:
    /// - Multiple heading levels (##, ###) that must be preserved
    /// - Markdown tables that need alignment
    /// - Inline links that should be converted to reference style
    #[test]
    fn test_real_world_file_complete_formatting() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_real_world.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_real_world.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_real_world.rs", input);

        // Verify heading levels are preserved.
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

        // Verify tables are formatted (aligned).
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

        // Verify reference-style links are created.
        assert!(
            formatted.contains("]: mod@"),
            "Should have reference-style links"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Test that files with `rustfmt_skip` attribute ARE formatted by
    /// `cargo-rustdoc-fmt`.
    ///
    /// This test uses a real-world file (mouse.rs) that has:
    /// - `#![cfg_attr(rustfmt, rustfmt_skip)]` at the top
    /// - Content that should be formatted (inline links, scattered references, terms)
    ///
    /// Because `#![rustfmt::skip]` is decoupled from `cargo-rustdoc-fmt` (only
    /// `// rustdoc-fmt: skip` skips rustdoc formatting), `FileProcessor` formats
    /// the doc blocks normally while preserving `#![cfg_attr(rustfmt, rustfmt_skip)]`.
    #[test]
    fn test_real_world_file_2_complete_formatting() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_real_world_2.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_real_world_2.rs"
        )));

        let (temp_dir, test_file, result, output) =
            run_default_processor_on_fixture("sample_real_world_2.rs", input);

        // File SHOULD be modified because rustfmt_skip is decoupled from
        // cargo-rustdoc-fmt.
        assert_eq!(
            result.status,
            FormatStatus::Modified,
            "File with rustfmt_skip should still have its rustdoc comments formatted"
        );

        assert!(
            output.contains("#![cfg_attr(rustfmt, rustfmt_skip)]"),
            "rustfmt_skip attribute must be preserved"
        );
        assert!(
            output.contains("//! [`VT100MouseAction`]: super::VT100MouseAction"),
            "Scattered references should be preserved and aggregated"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Test aggregation of scattered reference-style links.
    ///
    /// This test verifies that reference definitions scattered throughout doc blocks
    /// are correctly extracted, sorted alphabetically, and moved to the bottom with
    /// a blank line separator.
    #[test]
    fn test_scattered_references_aggregation() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_scattered_references.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_scattered_references.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_scattered_references.rs", input);

        assert_eq!(
            result.status,
            FormatStatus::Modified,
            "File should be modified (references need aggregation)"
        );

        // Verify references are at bottom of each block.
        assert!(
            formatted.contains("Navigate**:\n//! - ⬆️"),
            "Content should be preserved"
        );

        // Verify case-insensitive alphabetical sorting in module doc block.
        let module_refs = formatted
            .lines()
            .skip_while(|line| !line.starts_with("//! [`keyboard`]:"))
            .take(5)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            module_refs.contains("VT100MouseAction")
                && module_refs.contains("VT100MouseButton"),
            "Module references should be sorted alphabetically"
        );

        // Verify blank line before references for visual clarity.
        assert!(
            formatted.contains("\n//!\n//! [`keyboard`]:"),
            "Should have blank line before references"
        );

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
    /// Verifies that the CST parser preserves HTML comment blocks while allowing link
    /// conversion and reference sorting in surrounding prose.
    #[test]
    fn test_html_comments_preserved_links_converted() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_html_comments.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_html_comments.rs"
        )));

        let (_temp_dir, _test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_html_comments.rs", input);

        // Verify HTML comments are preserved.
        assert!(
            formatted.contains("<!-- It is ok to use ignore here -->"),
            "HTML comments should be preserved exactly"
        );

        // Verify code blocks with ignore attribute are preserved.
        assert!(
            formatted.contains("```ignore"),
            "Code blocks should be preserved"
        );

        // Verify references are sorted alphabetically at the bottom.
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
    /// - Code fences with `rust,ignore` are normalized to `ignore` and their body is
    ///   preserved verbatim as a `DocNode::CodeFence`
    /// - Reference definitions scattered throughout the rustdoc block are aggregated
    /// - References are sorted alphabetically at the bottom
    #[test]
    fn test_code_fence_comma_language_tag() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_code_fence_comma.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_code_fence_comma.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_code_fence_comma.rs", input);

        assert_eq!(
            result.status,
            FormatStatus::Modified,
            "File should be modified (references need aggregation)"
        );

        // Verify code fence with rust,ignore is normalized to ignore and body is
        // preserved.
        assert!(
            formatted.contains("```ignore"),
            "Code fence with rust,ignore should be normalized to ignore"
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
            .position(|line| line.contains("[`DirectToAnsi`]: mod@super"))
            .expect("Should find DirectToAnsi reference");
        let ref_einval = lines
            .iter()
            .position(|line| line.contains("[`EINVAL`]:"))
            .expect("Should find EINVAL reference");
        assert!(
            ref_start < ref_einval,
            "References should be sorted: [`DirectToAnsi`] before [`EINVAL`]"
        );

        // Verify all references are at the end of the rustdoc block.
        let last_rustdoc_line = lines
            .iter()
            .rposition(|line| line.starts_with("//!"))
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
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_indented_table.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_indented_table.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_indented_table.rs", input);

        assert_eq!(
            result.status,
            FormatStatus::Modified,
            "File should be modified (tables need formatting)"
        );

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

    /// Regression test: reference definitions with generic type params in backtick
    /// spans (such as `` [`Option<F::Waker>`] `` and `` [`Vec<u8>`] ``) must be
    /// recognized as `DocNode::ReferenceDefinitions` and sorted properly at the bottom
    /// of the doc block without creating spurious blank lines.
    #[test]
    fn test_rrt_generic_type_refs_sorted_correctly() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_rrt.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_rrt.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_rrt.rs", input);

        // References with generic type params in backticks should be sorted.
        assert!(
            formatted.contains("/// [`Arc<Mutex<Option<W::Waker>>>`]: std::sync::Arc"),
            "Generic-param reference should be present"
        );
        assert!(
            formatted.contains("/// [`Vec<u8>`]: std::vec::Vec"),
            "Generic-param reference should be present"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Idempotency test: running `rustdoc-fmt` on `rrt.rs` multiple times
    /// must produce identical output after the first run.
    ///
    /// This catches regressions where:
    /// - Multi-line ref defs get corrupted or duplicated on each run
    /// - Terms inside ref def URLs get linkified
    /// - Ref defs accumulate at the bottom
    #[test]
    fn test_rrt_idempotent_across_multiple_runs() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_rrt.rs"
        ));
        assert_fixture_idempotent("sample_rrt.rs", input);
    }

    /// Regression test: bracketed escape sequences inside backtick spans (such as
    /// `` `ESC[200~` ``) must not chain across lines to distant `]` characters or
    /// duplicate content during inline link conversion.
    #[test]
    fn test_input_event_no_cross_line_duplication() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_input_event.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_input_event.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_input_event.rs", input);

        // The escape sequence line must remain intact (not duplicated or mangled).
        assert!(
            formatted.contains("sequences (`ESC[200~` text `ESC[201~`)"),
            "Escape sequence line should remain intact"
        );

        // Content must appear exactly once (the key regression check).
        assert_eq!(
            formatted.matches("Clipboard Paste (Ctrl+V)").count(),
            1,
            "Content must not be duplicated by cross-line link matching"
        );

        // The inline link should be converted to reference-style.
        assert!(
            formatted.contains("[`EnableBracketedPaste`]"),
            "Inline link should be converted to reference-style"
        );
        assert!(
            formatted.contains("[`EnableBracketedPaste`]: crate::PaintRenderOpImplCrossterm::raw_mode_enter"),
            "Reference definition should be appended"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Regression test: `resilient_reactor_thread/mod.rs` reference sorting.
    ///
    /// Exercises reference aggregation and sorting across a large doc block with many
    /// scattered references, verifying no spurious blank lines or wrong ordering.
    #[test]
    fn test_rrt_mod_reference_sorting() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_rrt_mod.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_rrt_mod.rs"
        )));

        let (temp_dir, test_file, _result, _formatted) =
            run_default_processor_on_fixture("sample_rrt_mod.rs", input);

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Regression test: inline links inside backtick spans must not be converted.
    ///
    /// This test uses `parse_fragments_in_a_line.rs` which contains:
    /// - `` `[link](http://r3bl.com)` `` in a module doc comment
    /// - An intra-doc link ``
    ///   [`crate::parse_block_markdown_text_with_or_without_new_line()`] ``
    ///
    /// The backtick-wrapped `[link](http://r3bl.com)` is an inline code example, NOT
    /// a real link. The formatter must preserve it exactly.
    #[test]
    fn test_backtick_inline_link_not_converted() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_backtick_inline_link.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_backtick_inline_link.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_backtick_inline_link.rs", input);

        // Key regression check: backtick-wrapped inline link must be preserved.
        assert!(
            formatted.contains("`[link](http://r3bl.com)`"),
            "Inline link inside backticks must not be converted to reference-style"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Regression test: URLs with balanced parentheses must be captured in full.
    ///
    /// This test uses `engine_public_api.rs` which contains:
    /// - `[`editor_buffer.get_lines()`](EditorBuffer::get_lines())` - an inline link
    ///   whose URL (`EditorBuffer::get_lines()`) contains balanced parentheses
    ///
    /// The formatter must capture the full URL including the `()` suffix, not
    /// truncate at the first `)`.
    #[test]
    fn test_balanced_parens_url_not_truncated() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_balanced_parens_url.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_balanced_parens_url.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_balanced_parens_url.rs", input);

        // Key regression check: URL with balanced parens must be captured in full.
        assert!(
            formatted
                .contains("[`editor_buffer.get_lines()`]: EditorBuffer::get_lines()"),
            "Reference URL must include the full `get_lines()` with parens"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Regression test: reference definitions with nested brackets must not
    /// produce spurious blank lines.
    ///
    /// Link names like `` [`SmallString<[u8; 64]>`] `` contain nested `[]`
    /// brackets. The parser must match the outermost `]` so these are
    /// recognized as references and sorted together with other references
    /// without inserting a blank line between groups.
    #[test]
    fn test_nested_bracket_refs_no_spurious_blank_line() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_nested_bracket_refs.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_nested_bracket_refs.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_nested_bracket_refs.rs", input);

        // Key regression check: no blank line between reference definitions.
        assert!(
            !formatted.contains(
                "/// [`String`]: std::string::String\n///\n/// [`Display::fmt`]"
            ),
            "No spurious blank line should appear between reference definitions"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Test that text diagrams (ASCII art) inside code fences are preserved verbatim.
    ///
    /// This test uses a snapshot of `resilient_reactor_thread/mod.rs` containing:
    /// - Complex ASCII art diagrams inside code fences
    /// - Multiple markdown tables
    /// - HTML comments
    /// - Reference-style links scattered throughout
    #[test]
    fn test_resilient_reactor_text_diagrams() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_resilient_reactor.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_resilient_reactor.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_resilient_reactor.rs", input);

        // Verify text diagrams are preserved.
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

        // Verify tables are preserved.
        assert!(
            formatted.contains("| Component"),
            "Tables should be preserved"
        );
        assert!(
            formatted.contains("| :--"),
            "Table alignment markers should be preserved"
        );

        // Verify reference-style links are present.
        assert!(
            formatted.contains("[`epoll`]:"),
            "Reference-style links should be present"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Tests readline_async/mod.rs golden file comparison.
    ///
    /// Verifies:
    /// - Image links `![alt](url)` are NOT converted
    /// - Shortcut reference links `[text]` are NOT clobbered
    /// - Known terms (e.g., [`ANSI`]) are linked outside of protected regions
    /// - Already-formatted input is not modified (idempotency via golden file)
    #[test]
    fn test_readline_async_file_complete_formatting() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_readline_async.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_readline_async.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_readline_async.rs", input);

        // Verify image link is preserved.
        assert!(
            formatted.contains("![`readline_async_video`](https://github.com/r3bl-org/r3bl-open-core/tree/main/docs/video/r3bl_terminal_async_clip_ffmpeg.gif?raw=true)"),
            "Image link should be preserved"
        );

        // Verify shortcut reference links are not clobbered.
        assert!(
            formatted
                .contains("[Linux TTY and async Rust - Article on developerlife.com]"),
            "Shortcut reference link should not be modified: {formatted}"
        );

        // Verify known term ANSI is linked.
        assert!(
            formatted.contains("[`ANSI`]"),
            "Known term ANSI should be linked"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Idempotency test for readline_async/mod.rs: run the formatter 3 times and
    /// verify output is identical after each run.
    #[test]
    fn test_readline_async_idempotent_across_multiple_runs() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_readline_async.rs"
        ));
        assert_fixture_idempotent("sample_readline_async.rs", input);
    }

    /// Verifies every seed term is correctly handled across three states: bare
    /// (plain text), backticked only, and already linked. Also covers qualified
    /// paths, single-colon edge cases, compound spec terms, and code fences.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_seed_term_coverage() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_seed_term_coverage.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_seed_term_coverage.rs"
        )));

        let (temp_dir, test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_seed_term_coverage.rs", input);

        // Bare terms should be linkified.
        for term in [
            "CSI",
            "SGR",
            "ESC",
            "DSR",
            "OSC",
            "stdout",
            "stderr",
            "stdin",
            "stdio",
            "crossterm",
            "mio",
            "portable_pty",
            "ANSI",
            "ASCII",
            "VT-100",
            "tokio",
            "vte",
            "xterm",
            "Alacritty",
            "Kitty",
            "RXVT",
            "Sixel",
            "DEC",
            "EOF",
            "SIGWINCH",
            "SIGINT",
            "SIGTERM",
            "grapheme",
            "Unicode",
            "RGB",
            "epoll",
            "kqueue",
            "Bracketed paste",
            "truecolor",
            "termios",
        ] {
            assert!(
                formatted.contains(&format!("[`{term}`]")),
                "Bare term '{term}' should be linked in output: not found"
            );
        }

        // Compound spec terms should be linked.
        for spec in [
            "[`CSI` spec]",
            "[`SGR` spec]",
            "[`ESC` spec]",
            "[`VT-100` spec]",
        ] {
            assert!(
                formatted.contains(spec),
                "Compound spec term '{spec}' should be linked"
            );
        }

        // Qualified paths must NOT be split.
        for path in [
            "tokio::io::stdin()",
            "vte::Parser",
            "std::io::stdout()",
            "crossterm::event::read()",
            "mio::Poll::new()",
        ] {
            assert!(
                formatted.contains(path),
                "Qualified path {path} should not be split"
            );
        }

        // Single colon after term must still linkify.
        assert!(
            formatted.contains("[`ESC`]:"),
            "ESC followed by single colon should still be linked"
        );

        // Code fence content must NOT be linkified.
        assert!(
            formatted.contains("CSI 38 ; 5 ; n m"),
            "Terms inside code fence should not be linkified"
        );
        assert!(
            formatted.contains("tokio::spawn(async"),
            "Qualified path inside code fence should not be touched"
        );

        // Autolink URLs containing a known term must NOT be corrupted.
        assert!(
            formatted.contains(
                "<https://unix.stackexchange.com/questions/597083/how-does-piping-affect-stdin>"
            ),
            "Term inside autolink URL should not be linkified"
        );

        // Standalone plain shortcut link [termios] must be upgraded to [`termios`],
        // while multi-line bracketed link text must NOT have interior terms linkified.
        assert!(
            formatted.contains("The POSIX [`termios`] flag controls signal generation."),
            "Standalone [termios] shortcut link should be upgraded to [`termios`]"
        );
        assert!(
            formatted.contains(
                "See our [Unix\n//! implementation's termios section] for details."
            ),
            "Multi-line bracketed link text should not have interior term linkified"
        );

        // Compound shortcut link starting with backtick must NOT have interior terms
        // linkified.
        assert!(
            formatted
                .contains("Controls debug logging for the [`VT-100` PTY output parser]."),
            "Compound shortcut link starting with backtick must not have interior PTY linkified"
        );

        // Multi-line inline links in blockquotes and lists must produce single-line
        // normalized ref defs.
        assert!(
            formatted.contains("/// [coordinates module documentation]: crate::coordinates#common-workflows"),
            "Multi-line blockquote inline link should produce normalized single-line ref def"
        );
        assert!(
            formatted
                .contains("/// [Python-based alternatives]: https://github.com/edc/bass"),
            "Multi-line indented inline link should produce normalized single-line ref def"
        );

        // Intra-doc backticked ref def must not be overwritten by duplicate plain URL ref
        // def.
        assert!(
            formatted.contains("/// [`termios`]: rustix::termios"),
            "Intra-doc backticked ref def should be preserved over plain external URL ref def"
        );

        // TOC block content must not be modified.
        assert!(
            formatted.contains("/// - [stdin and termios overview](#stdin-handling)"),
            "Terms inside <!-- TOC --> block should not be linkified"
        );

        // Bold, hyphenated, parenthesized, and slash-separated terms must linkify
        // cleanly.
        assert!(
            formatted.contains(
                "/// Uses [`termios`] API and **[`termios`]** flags for [`Unicode`]-safe, [`grapheme`]-cluster-aware\n/// ([`stdin`]) reading and [`RGB`]/[`truecolor`] rendering."
            ),
            "Bold, hyphenated, parenthesized, and slash-separated terms should linkify cleanly"
        );

        // Heading with known term must be linked, and wrong external URL replaced by
        // canonical.
        assert!(
            formatted.contains("/// # [`CSI`] Edge Cases"),
            "Heading with known term should be linked"
        );
        assert!(
            !formatted.contains("wrong-url.com"),
            "Wrong external URL on known term should be replaced by canonical target"
        );

        // Image alt text, inline anchor links, full ref links, and converted inline links
        // containing known terms must not have interior terms linkified.
        assert!(
            formatted.contains(
                "/// Diagram: ![termios and stdin diagram](https://example.com/stdin.png) and\n/// [see the stdin section](#stdin-handling), [CSI overview][csi-ref], and\n/// [Linux TTY and CSI codes]."
            ),
            "Image alt text and links containing known terms should not be corrupted"
        );

        // Multi-backtick inline code span before backticked inline link and
        // double-backticked term must not be converted.
        assert!(
            formatted.contains(
                "/// Parses `` `code` `` and `[inline link inside code](https://example.com)` and\n/// `` `CSI` `` without converting code spans."
            ),
            "Backticked inline link and double-backticked term after multi-backtick span should not be converted"
        );
        assert!(
            formatted.contains(
                "/// - Bold (`**text**`), italic (`_text_`), inline code (`` `code` ``)\n/// - Images (`![alt](url)`), links (`[text](url)`)"
            ),
            "Backticked inline link after multi-backtick span (tui/src/lib.rs pattern) should not be converted"
        );

        // Single-line and multi-line code spans containing terms must not be modified.
        assert!(
            formatted.contains(
                "/// - Code spans with terms: `cargo test -p CSI --lib foo -- --nocapture`, `cargo test -p r3bl_tui --lib CSI --\n///   --nocapture`, and ``cargo test CSI\n///   --nocapture``"
            ),
            "Single-line and multi-line code spans containing terms should not be modified"
        );

        // Table cells containing known terms must be linkified and columns aligned.
        assert!(
            formatted.contains("/// | [`stdin`] | [`SIGWINCH`] | [`epoll`] |"),
            "Table cells containing known terms should be linkified and aligned"
        );

        // Ref defs should be present for all seed terms.
        assert!(
            formatted.contains("[`CSI`]: crate::CsiSequence"),
            "CSI ref def should be present"
        );
        assert!(
            formatted.contains("[`tokio`]: tokio"),
            "tokio ref def should be present"
        );
        assert!(
            formatted
                .contains("[`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code"),
            "ANSI ref def should be present"
        );

        assert_git_diff_eq(&temp_dir, &test_file, &expected);
    }

    /// Idempotency test: running the formatter multiple times on the seed term
    /// coverage fixture must produce identical output after the first run.
    #[test]
    fn test_seed_term_coverage_idempotent() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_seed_term_coverage.rs"
        ));
        assert_fixture_idempotent("sample_seed_term_coverage.rs", input);
    }

    /// Regression test: multi-line plain (non-backticked) reference definitions
    /// must not be deleted by the term linker.
    ///
    /// When rustfmt splits a long reference definition across two lines:
    /// ```text
    /// [Inclusive Naming Initiative - Tier 1 Terms]:
    ///     https://inclusivenaming.org/word-lists/tier-1/
    /// ```
    /// the CST parser groups the indented URL continuation line into the
    /// `DocNode::ReferenceDefinitions` so that both the link converter and term linker
    /// preserve it intact.
    #[test]
    fn test_multiline_plain_ref_def_preserved() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_pty_types.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_pty_types.rs"
        )));

        let (_temp_dir, _test_file, _result, formatted) =
            run_default_processor_on_fixture("sample_pty_types.rs", input);

        // Key regression check: multi-line plain ref def must be preserved.
        assert!(
            formatted.contains(
                "/// [Inclusive Naming Initiative - Tier 1 Terms]:\n///     https://inclusivenaming.org/word-lists/tier-1/"
            ),
            "Multi-line plain reference definition should be preserved"
        );

        assert_eq!(
            formatted, expected,
            "Formatted output should match expected output"
        );
    }

    /// Utility test to regenerate golden files. Run with:
    /// ```bash
    /// cargo test -p r3bl-build-infra --lib -- regen_golden_output_snapshot_files_for_input_files --ignored --nocapture
    /// ```
    ///
    /// Auto-discovers all `.rs` files in `test_data/complete_file/input/` and
    /// regenerates the matching `expected_output/` files using the full pipeline
    /// (registry + default options).
    #[test]
    #[ignore = "utility for regenerating golden files on demand"]
    fn regen_golden_output_snapshot_files_for_input_files() {
        let manifest = env!("CARGO_MANIFEST_DIR");
        let registry = test_registry();
        let proc =
            processor::FileProcessor::new(FormatOptions::default(), Some(&registry));

        let input_dir = std::path::Path::new(manifest)
            .join("src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input");
        let output_dir = std::path::Path::new(manifest).join(
            "src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output",
        );

        let mut entries: Vec<_> = fs::read_dir(&input_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == RUST_FILE_EXTENSION)
            })
            .collect();
        entries.sort_by_key(std::fs::DirEntry::file_name);

        for entry in &entries {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let input = fs::read_to_string(entry.path()).unwrap();
            let temp_dir = TempDir::new().unwrap();
            let temp_file = temp_dir.path().join(name.as_ref());
            fs::write(&temp_file, &input).unwrap();
            let _results = proc.process_files(std::slice::from_ref(&temp_file));
            let output = fs::read_to_string(&temp_file).unwrap();
            fs::write(output_dir.join(name.as_ref()), &output).unwrap();
            eprintln!("Regenerated: {name}");
        }
    }

    #[test]
    fn test_fence_normalizer_strips_redundant_rust() {
        let input = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/input/sample_fence_normalizer.rs"
        ));
        let expected = normalize_line_endings(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/expected_output/sample_fence_normalizer.rs"
        )));

        let (_temp_dir, _test_file, result, formatted) =
            run_default_processor_on_fixture("sample_fence_normalizer.rs", input);

        assert_eq!(
            result.status,
            FormatStatus::Modified,
            "File should be modified when redundant rust fence tags are stripped"
        );

        assert!(!formatted.contains("```rust"));
        assert!(formatted.contains("//! ```ignore"));
        assert!(formatted.contains("//! ```no_run"));
        assert!(formatted.contains("//! ```text"));
        assert_eq!(formatted, expected);
    }
}
