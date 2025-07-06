/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

/// ## Compatibility test suite
///
/// This module contains tests that ensure the NG parser (`parse_markdown_ng`)
/// produces identical output to the legacy parser (`parse_markdown`) for all
/// markdown inputs, including challenging edge cases.
///
/// ## Parser comparison with real world use cases
///
/// There are two main paths for parsing markdown in the R3BL TUI editor:
/// 1. NG parser path: Convert &[`crate::GCString`] to [`AsStrSlice`] (no copy) ->
///    [`crate::parse_markdown_ng()`]
/// 2. Legacy parser path: &[`crate::GCString`] -> materialized string (full copy) ->
///    [`crate::parse_markdown()`]
///
/// ## Test categories
///
/// The test suite covers the following categories:
///
/// ### Basic text handling
/// - Empty strings and single/multiple line inputs
/// - Edge cases (empty lines, whitespace variations, trailing spaces)
///
/// ### Core markdown features
/// - Headings (all levels H1-H6)
/// - Text formatting (bold, italic, inline code)
/// - Mixed and nested formatting
/// - Lists (ordered, unordered, nested, checkboxes)
/// - Links and images (simple and complex)
/// - Code blocks (with/without language, empty blocks)
///
/// ### Advanced features
/// - Metadata (@title, @tags, @authors, @date)
/// - Complex documents with mixed content
/// - Code blocks within lists
///
/// ### Edge cases and error handling
/// - Unicode and special characters
/// - Malformed syntax and unclosed formatting
/// - Whitespace and newline handling variations
///
/// ### Debug and development tests
/// - Debug utilities for parser comparison
/// - Known issue verification (Unicode inline code fix)
/// - Failing case analysis
///
/// ## Test methodology
///
/// Each test follows a comparison pattern:
/// 1. Convert input string to `&[GCString]` (simulating editor content)
/// 2. Legacy parser path: materialize to string -> `parse_markdown(&str)`
/// 3. NG parser path: use `AsStrSlice` directly -> `parse_markdown_ng(AsStrSlice)`
/// 4. Compare results: both must succeed/fail consistently with identical output
/// 5. Validate remainder consistency between parsers
///
/// ## Known limitations
///
/// - One comprehensive test is skipped due to known differences in code block spacing
/// - Some tests include debug output for development purposes
#[cfg(test)]
mod tests_parse_markdown_compatibility {
    use crate::{get_real_world_editor_content, parse_markdown, parse_markdown_ng,
                AsStrSlice, GCString, ParserByteCache};

    /// Helper function to test compatibility between `parse_markdown` and
    /// `parse_markdown_ng` This simulates the real-world usage in
    /// `try_parse_and_highlight` where both parsers start from the same &[`GCString`]
    /// source but take different paths.
    fn test_compatibility_helper(test_name: &str, input_str: &str) {
        // Step 1 - Convert input_str to &[GCString]:
        // The common source of truth, simulating editor content.
        let gcs_lines: Vec<GCString> = input_str.lines().map(GCString::from).collect();
        let source_of_truth = AsStrSlice::from(gcs_lines.as_slice());

        // Step 2 - Legacy parser path:
        // &[GCString] -> materialize string -> parse_markdown(&str)
        // Transform gc_lines into a materialized string.
        let size_hint = gcs_lines.iter().map(|line| line.len().as_usize() + 1).sum();
        let mut materialized_cache = ParserByteCache::with_capacity(size_hint);
        source_of_truth.write_to_byte_cache_compat(size_hint, &mut materialized_cache);
        let materialized_input = materialized_cache.as_str();
        let og_res = parse_markdown(materialized_input);

        // Step 3 - NG parser path:
        // &[GCString] -> AsStrSlice -> parse_markdown_ng(AsStrSlice)
        // Uses the original slice, not the materialized string.
        let ng_res = parse_markdown_ng(source_of_truth);

        // Step 4 - Compare results:
        // Both succeed → Compare their results
        // Both fail → Test passes (consistent failure)
        // One succeeds, one fails → Test should fail with a clear message

        // Both parsers should either succeed or fail consistently.
        assert_eq!(
            og_res.is_ok(),
            ng_res.is_ok(),
            // Panic message if assertion fails.
            "{a}: One parser succeeded while the other failed. Legacy: {b}, NG: {c}",
            a = test_name,
            b = og_res.is_ok(),
            c = ng_res.is_ok()
        );

        // Both parsers should either succeed or fail consistently.
        match (og_res.is_ok(), ng_res.is_ok()) {
            (true, true) => {
                // Both succeeded - compare their results.
                let (og_rem, og_doc) = og_res.unwrap();
                let (ng_rem, ng_doc) = ng_res.unwrap();

                // Check documents are equivalent. This MUST be an EXACT match. This is
                // the actual compatibility test.
                assert_eq!(
                    og_doc, ng_doc,
                    "{test_name}: Documents don't match.\nLegacy: {og_doc:#?}\nNG: {ng_doc:#?}",
                );

                // Materialize the NG remainder. Then compare them to ensure they match.
                // The remainder gets thrown away in the editor, so this is just for
                // consistency checking.
                let ng_remainder_str = ng_rem.to_inline_string();
                if og_rem != ng_remainder_str.as_str() {
                    panic!(
                        "The legacy and NG parser remainders don't match.\n\
                            Legacy: {og_rem:?}\nNG: {ng_remainder_str:?}\nTest: {test_name}",
                    );
                }
            }
            (false, false) => {
                // Both failed - test passes (consistent failure is valid).
            }
            _ => {
                // One parser succeeded while the other failed.
                panic!(
                    "{}: One parser succeeded while the other failed. Legacy: {}, NG: {}",
                    test_name,
                    og_res.is_ok(),
                    ng_res.is_ok()
                );
            }
        }
    }

    #[test]
    fn test_simple_inline_code() {
        test_compatibility_helper("simple_inline_code", "first\n`second`");
    }

    #[test]
    fn test_inline_code_variations() {
        test_compatibility_helper(
            "inline_code_variations",
            "`simple code`\n`code with spaces`\n`code-with-dashes`\n`code_with_underscores`"
        );
    }

    #[test]
    fn test_inline_code_with_unicode() {
        // This test verifies that the Unicode inline code issue has been fixed
        test_compatibility_helper("inline_code_with_unicode", "`code 🎯`");
    }

    #[test]
    fn test_single_line_no_newline() {
        test_compatibility_helper("single_line_no_newline", "Hello World");
    }

    #[test]
    fn test_single_line_with_newline() {
        test_compatibility_helper("single_line_with_newline", "Hello World\n");
    }

    #[test]
    fn test_multiple_lines() {
        test_compatibility_helper(
            "multiple_lines",
            "First line\nSecond line\nThird line",
        );
    }

    #[test]
    fn test_empty_string() { test_compatibility_helper("empty_string", ""); }

    #[test]
    fn test_only_newlines() { test_compatibility_helper("only_newlines", "\n\n\n"); }

    #[test]
    fn test_heading_basic() {
        test_compatibility_helper("heading_basic", "# Main Heading\nSome content");
    }

    #[test]
    fn test_multiple_headings() {
        test_compatibility_helper("multiple_headings", "# H1\n## H2\n### H3\nContent");
    }

    #[test]
    fn test_bold_text() { test_compatibility_helper("bold_text", "This is *bold* text"); }

    #[test]
    fn test_italic_text() {
        test_compatibility_helper("italic_text", "This is _italic_ text");
    }

    #[test]
    fn test_mixed_formatting() {
        test_compatibility_helper(
            "mixed_formatting",
            "Mix of *bold* and _italic_ and `code`",
        );
    }

    #[test]
    fn test_links() {
        test_compatibility_helper(
            "links",
            "Check out [Rust](https://rust-lang.org) website",
        );
    }

    #[test]
    fn test_images() {
        test_compatibility_helper("images", "![Alt text](https://example.com/image.png)");
    }

    #[test]
    fn test_unordered_list_simple() {
        test_compatibility_helper(
            "unordered_list_simple",
            "- Item 1\n- Item 2\n- Item 3",
        );
    }

    #[test]
    fn test_ordered_list_simple() {
        test_compatibility_helper("ordered_list_simple", "1. First\n2. Second\n3. Third");
    }

    #[test]
    fn test_nested_unordered_list() {
        test_compatibility_helper(
            "nested_unordered_list",
            "- Top level\n  - Nested item\n    - Deep nested\n- Back to top",
        );
    }

    #[test]
    fn test_nested_ordered_list() {
        test_compatibility_helper(
            "nested_ordered_list",
            "1. First\n  2. Nested second\n     Content\n    3. Nested third\n2. Second top"
        );
    }

    #[test]
    fn test_checkboxes() {
        test_compatibility_helper(
            "checkboxes",
            "- [ ] Unchecked\n- [x] Checked\n- [X] Also checked",
        );
    }

    #[test]
    fn test_code_block_bash() {
        test_compatibility_helper(
            "code_block_bash",
            "```bash\necho \"Hello World\"\nls -la\n```",
        );
    }

    #[test]
    fn test_code_block_rust() {
        test_compatibility_helper(
            "code_block_rust",
            "```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```",
        );
    }

    #[test]
    fn test_code_block_no_language() {
        test_compatibility_helper(
            "code_block_no_language",
            "```\nSome code\nwithout language\n```",
        );
    }

    #[test]
    fn test_empty_code_block() {
        test_compatibility_helper("empty_code_block", "```rust\n```");
    }

    #[test]
    fn test_metadata_title() {
        test_compatibility_helper("metadata_title", "@title: My Document Title");
    }

    #[test]
    fn test_metadata_tags() {
        test_compatibility_helper("metadata_tags", "@tags: rust, programming, tutorial");
    }

    #[test]
    fn test_metadata_authors() {
        test_compatibility_helper("metadata_authors", "@authors: John Doe, Jane Smith");
    }

    #[test]
    fn test_metadata_date() {
        test_compatibility_helper("metadata_date", "@date: 2025-01-01");
    }

    #[test]
    fn test_comprehensive_document() {
        // Use real-world content from tui/examples/tui_apps/ex_editor/state.rs
        // This includes emojis in headings and other complex markdown features
        let comprehensive_input = get_real_world_editor_content().join("\n");
        test_compatibility_helper("comprehensive_document", &comprehensive_input);
    }

    #[test]
    fn test_edge_case_empty_lines() {
        test_compatibility_helper("edge_case_empty_lines", "Line 1\n\n\nLine 2\n\n");
    }

    #[test]
    fn test_edge_case_whitespace_lines() {
        test_compatibility_helper(
            "edge_case_whitespace_lines",
            "Line 1\n   \n\t\nLine 2",
        );
    }

    #[test]
    fn test_edge_case_trailing_spaces() {
        test_compatibility_helper(
            "edge_case_trailing_spaces",
            "Line with trailing spaces   \nAnother line  ",
        );
    }

    #[test]
    fn test_mixed_list_types() {
        test_compatibility_helper(
            "mixed_list_types",
            "- Unordered item\n1. Ordered item\n- [ ] Checkbox item\n2. Another ordered",
        );
    }

    #[test]
    fn test_formatting_edge_cases() {
        test_compatibility_helper(
            "formatting_edge_cases",
            "*start bold*\n_start italic_\n`start code`\nend *bold*\nend _italic_\nend `code`"
        );
    }

    #[test]
    fn test_unclosed_formatting() {
        test_compatibility_helper(
            "unclosed_formatting",
            "This has *unclosed bold\nThis has _unclosed italic\nThis has `unclosed code",
        );
    }

    #[test]
    fn test_nested_formatting() {
        test_compatibility_helper(
            "nested_formatting",
            "This is *bold with `code` inside*\nThis is _italic with `code` inside_",
        );
    }

    #[test]
    fn test_complex_list_with_content() {
        test_compatibility_helper(
            "complex_list_with_content",
            r#"1. First item
   This is additional content for item 1

   More content with empty line

2. Second item
   - Nested unordered
   - Another nested
     With additional content
3. Back to ordered"#,
        );
    }

    #[test]
    fn test_code_blocks_in_lists() {
        test_compatibility_helper(
            "code_blocks_in_lists",
            r#"1. Install dependencies:
   ```bash
   cargo install my-tool
   ```
2. Run the tool:
   ```bash
   my-tool --help
   ```"#,
        );
    }

    #[test]
    fn test_all_heading_levels() {
        test_compatibility_helper(
            "all_heading_levels",
            "# H1\n## H2\n### H3\n#### H4\n##### H5\n###### H6",
        );
    }

    #[test]
    fn test_special_characters() {
        test_compatibility_helper(
            "special_characters",
            "Special chars: !@#$%^&*()_+-=[]{}|;':\",./<>?",
        );
    }

    #[test]
    fn test_unicode_content() {
        test_compatibility_helper(
            "unicode_content",
            "Unicode: 🦀 Rust, 📝 Markdown, 🚀 Fast parsing\nEmoji in `code 🎯`",
        );
    }

    #[test]
    fn test_complex_links() {
        test_compatibility_helper(
            "complex_links",
            r#"Various links:
- [Simple](https://example.com)
- [With title](https://example.com "Title")
- [Complex URL](https://example.com/path?param=value&other=test#section)
- ![Image link](https://example.com/image.png "Alt text")"#,
        );
    }

    #[test]
    fn test_malformed_syntax() {
        test_compatibility_helper(
            "malformed_syntax",
            "###not a heading\n```notclosed\n- [  invalid checkbox\n*not bold text",
        );
    }

    #[test]
    fn test_emoji_in_headings() {
        // Test simple emoji in H1 heading
        test_compatibility_helper("emoji_h1_simple", "# Heading with emoji 😀");

        // Test emoji in H2 heading
        test_compatibility_helper("emoji_h2_simple", "## Subheading with emoji 😀");

        // Test multiple emojis in heading
        test_compatibility_helper("emoji_multiple", "# Multiple emojis 😀🚀📝");

        // Test emoji at different positions
        test_compatibility_helper(
            "emoji_start_middle_end",
            "# 😀 Emoji at start\n## Middle 😀 emoji\n### Emoji at end 😀",
        );
    }

    #[test]
    fn test_emoji_headings_with_content() {
        // Test the specific case from our real-world content that's failing
        test_compatibility_helper(
            "emoji_h2_long",
            "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀"
        );

        // Test emoji heading followed by content
        test_compatibility_helper(
            "emoji_heading_with_content",
            "# Heading 😀\nSome content below",
        );
    }

    #[test]
    fn test_emoji_heading_in_multiline_context() {
        // Test the exact pattern from our comprehensive test that's failing
        test_compatibility_helper(
            "emoji_h2_with_following_content",
            "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀\n\n1. line 1 of 2"
        );

        // Simpler version to isolate the issue
        test_compatibility_helper("emoji_h2_with_list", "## Heading 😀\n\n1. List item");

        // Test with H1 to see if it's specific to H2
        test_compatibility_helper("emoji_h1_with_list", "# Heading 😀\n\n1. List item");
    }
}
