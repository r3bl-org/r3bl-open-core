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
    use crate::{parse_markdown,
                parse_markdown_ng,
                AsStrSlice,
                GCString,
                ParserByteCache};

    /// Helper function to test compatibility between parse_markdown and parse_markdown_ng
    /// This simulates the real-world usage in try_parse_and_highlight where both parsers
    /// start from the same &[GCString] source but take different paths.
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
        let legacy_result = parse_markdown(materialized_input);

        // Step 3 - NG parser path:
        // &[GCString] -> AsStrSlice -> parse_markdown_ng(AsStrSlice)
        // Uses the original slice, not the materialized string.
        let ng_result = parse_markdown_ng(source_of_truth);

        // Step 4 - Compare results:
        // Both succeed → Compare their results
        // Both fail → Test passes (consistent failure)
        // One succeeds, one fails → Test should fail with a clear message

        // Both parsers should either succeed or fail consistently.
        assert_eq!(
            legacy_result.is_ok(),
            ng_result.is_ok(),
            "{}: One parser succeeded while the other failed. Legacy: {}, NG: {}",
            test_name,
            legacy_result.is_ok(),
            ng_result.is_ok()
        );

        // Both parsers should either succeed or fail consistently.
        match (legacy_result.is_ok(), ng_result.is_ok()) {
            (true, true) => {
                // Both succeeded - compare their results.
                let (legacy_remainder, legacy_doc) = legacy_result.unwrap();
                let (ng_remainder, ng_doc) = ng_result.unwrap();

                // Check documents are equivalent. This MUST be an EXACT match. This is
                // the actual compatibility test.
                assert_eq!(
                    legacy_doc, ng_doc,
                    "{}: Documents don't match.\nLegacy: {:#?}\nNG: {:#?}",
                    test_name, legacy_doc, ng_doc
                );

                // Materialize the NG remainder. Then compare them to ensure they match.
                // The remainder gets thrown away in the editor, so this is just for
                // consistency checking.
                let ng_remainder_str = ng_remainder.to_inline_string();
                if legacy_remainder != ng_remainder_str.as_str() {
                    panic!(
                        "The legacy and NG parser remainders don't match.\n\
                            Legacy: {:?}\nNG: {:?}\nTest: {}",
                        legacy_remainder, ng_remainder_str, test_name
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
                    legacy_result.is_ok(),
                    ng_result.is_ok()
                );
            }
        }
    }

    #[test]
    fn test_simple_inline_code() {
        test_compatibility_helper("simple_inline_code", "first\n`second`");
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
        let comprehensive_input = get_real_world_content();
        test_compatibility_helper("comprehensive_document", &comprehensive_input);
    }

    #[test]
    fn test_debug_comprehensive_document() {
        // Debug version of the comprehensive document test to help identify parsing
        // issues
        let comprehensive_input = get_real_world_content();
        debug_parser_processing("comprehensive_document", &comprehensive_input);
    }

    /// Returns the real-world markdown content from the ex_editor example.
    /// This content includes emojis in headings, nested lists, code blocks, metadata,
    /// and other complex markdown features that help identify parser compatibility
    /// issues.
    fn get_real_world_content() -> String {
        let lines = &[
            "0         1         2         3         4         5         6",
            "0123456789012345678901234567890123456789012345678901234567890",
            "@title: untitled",
            "@tags: foo, bar, baz",
            "@authors: xyz, abc",
            "@date: 12-12-1234",
            "",
            "# This approach will not be easy. You are required to fly straight😀",
            "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀",
            "",
            "1. line 1 of 2",
            "2. line 2 of 2",
            "",
            "This is _not italic [link](https://r3bl.com) not bold* etc.",
            "",
            "```ts",
            "let a=1;",
            "```",
            "",
            "`foo`",
            "",
            "*bar*",
            "**baz**",
            "",
            "```rs",
            "let a=1;",
            "```",
            "",
            "- [x] done",
            "- [ ] todo",
            "",
            "# Random writing from star wars text lorem ipsum generator",
            "",
            "1. A hyperlink [link](https://forcemipsum.com/)",
            "   inline code `code`",
            "    2. Did you hear that?",
            "       They've shut down the main reactor.",
            "       We'll be destroyed for sure.",
            "       This is madness!",
            "       We're doomed!",
            "",
            "## Random writing from star trek text lorem ipsum generator",
            "",
            "- Logic is the beginning of wisdom, not the end. ",
            "  A hyperlink [link](https://fungenerators.com/lorem-ipsum/startrek/)",
            "  I haven't faced death. I've cheated death. ",
            "  - I've tricked my way out of death and patted myself on the back for my ingenuity; ",
            "    I know nothing. It's not safe out here. ",
            "    - Madness has no purpose. Or reason. But it may have a goal.",
            "      Without them to strengthen us, we will weaken and die. ",
            "      You remove those obstacles.",
            "      - But one man can change the present!  Without freedom of choice there is no creativity. ",
            "        I object to intellect without discipline; I object to power without constructive purpose. ",
            "        - Live Long and Prosper. To Boldly Go Where No Man Has Gone Before",
            "          It's a — far, far better thing I do than I have ever done before",
            "          - A far better resting place I go to than I have ever know",
            "            Something Spock was trying to tell me on my birthday",
            "",
        ];

        lines.join("\n")
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
    fn test_inline_code_variations() {
        test_compatibility_helper(
            "inline_code_variations",
            "`simple code`\n`code with spaces`\n`code-with-dashes`\n`code_with_underscores`"
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
    fn test_debug_newline_handling() {
        let input_str = "Line 1\n\n\nLine 2\n\n";

        // Test what str.lines() produces
        let str_lines: Vec<&str> = input_str.lines().collect();
        println!("str.lines() produces: {:?}", str_lines);

        // Test what AsStrSlice produces when converted back
        let gc_lines: Vec<GCString> =
            str_lines.iter().map(|&line| GCString::from(line)).collect();
        let slice = AsStrSlice::from(gc_lines.as_slice());
        let as_str_slice_string = slice.to_inline_string();
        println!(
            "AsStrSlice converts back to: {:?}",
            as_str_slice_string.as_str()
        );

        // Compare with original
        println!("Original input: {:?}", input_str);
        println!(
            "Are they equal? {}",
            input_str == as_str_slice_string.as_str()
        );

        // Test legacy parser
        let legacy_result = parse_markdown(input_str);
        if let Ok((remainder, _)) = legacy_result {
            println!("Legacy remainder: {:?}", remainder);
        }

        // Test new parser
        let ng_result = parse_markdown_ng(slice);
        if let Ok((remainder, _)) = ng_result {
            println!("NG remainder: {:?}", remainder.to_inline_string().as_str());
        }
    }
    #[test]
    fn test_inline_code_with_unicode_now_fixed() {
        // This test verifies that the Unicode inline code issue has been fixed
        test_compatibility_helper("inline_code_with_unicode", "`code 🎯`");

        println!("✅ Fixed: NG parser now correctly parses Unicode in inline code");
        println!("Both parsers: `code 🎯` -> InlineCode(\"code 🎯\")");
    }

    /// Debug function to understand exactly how inputs are being processed
    fn debug_parser_processing(test_name: &str, input_str: &str) {
        println!("\n=== Debug: {} ===", test_name);
        println!("Original input: {:?}", input_str);

        // Show what str.lines() produces
        let str_lines: Vec<&str> = input_str.lines().collect();
        println!("str.lines() produces: {:?}", str_lines);

        // Show AsStrSlice conversion
        let gc_lines: Vec<GCString> =
            str_lines.iter().map(|&line| GCString::from(line)).collect();
        let slice = AsStrSlice::from(gc_lines.as_slice());
        println!("AsStrSlice has {} lines", slice.lines.len());
        for (i, line) in slice.lines.iter().enumerate() {
            println!("  Line {}: {:?}", i, line.string.as_str());
        }
        let slice_as_string = slice.to_inline_string();
        println!(
            "AsStrSlice converts back to: {:?}",
            slice_as_string.as_str()
        );

        // Test legacy parser with ORIGINAL input
        let legacy_result = parse_markdown(input_str);
        if let Ok((remainder, doc)) = legacy_result {
            println!("Legacy (original) remainder: {:?}", remainder);
            println!("Legacy (original) doc has {} elements:", doc.inner.len());
            for (i, element) in doc.inner.iter().enumerate() {
                println!("  Element {}: {:?}", i, element);
            }
        }

        // Test legacy parser with CONVERTED input (same as NG)
        let legacy_converted_result = parse_markdown(slice_as_string.as_str());
        if let Ok((remainder, doc)) = legacy_converted_result {
            println!("Legacy (converted) remainder: {:?}", remainder);
            println!("Legacy (converted) doc has {} elements:", doc.inner.len());
            for (i, element) in doc.inner.iter().enumerate() {
                println!("  Element {}: {:?}", i, element);
            }
        }

        // Test new parser
        let ng_result = parse_markdown_ng(slice);
        if let Ok((remainder, doc)) = ng_result {
            println!("NG remainder: {:?}", remainder.to_inline_string().as_str());
            println!("NG doc has {} elements:", doc.inner.len());
            for (i, element) in doc.inner.iter().enumerate() {
                println!("  Element {}: {:?}", i, element);
            }
        }
        println!("=========================\n");
    }

    #[test]
    fn test_debug_failing_cases() {
        // Debug the three failing cases
        debug_parser_processing("edge_case_empty_lines", "Line 1\n\n\nLine 2\n\n");
        debug_parser_processing("simple_inline_code", "first\n`second`");
        debug_parser_processing("inline_code_variations", "`simple code`\n`code with spaces`\n`code-with-dashes`\n`code_with_underscores`");
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

    #[test]
    fn test_debug_emoji_multiline() {
        // Debug the multiline emoji heading issue
        debug_parser_processing("emoji_h2_multiline", "## Heading 😀\n\n1. List item");
        debug_parser_processing("emoji_h2_long_multiline", "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀\n\n1. line 1 of 2");
    }

    #[test]
    fn test_debug_heading_emoji_isolation() {
        use crate::{as_str_slice_test_case, parse_line_heading_no_advance_ng};

        // Test simple emoji heading alone
        {
            as_str_slice_test_case!(input1, "## Heading 😀");
            let result1 = parse_line_heading_no_advance_ng(input1);
            match result1 {
                Ok((remainder, heading_data)) => {
                    println!("✅ Simple emoji heading parsed successfully:");
                    println!("   Level: {}", heading_data.level.level);
                    println!("   Text: '{}'", heading_data.text);
                    println!("   Remainder: '{}'", remainder.to_string());
                    println!("   Remainder is empty: {}", remainder.is_empty());
                }
                Err(e) => {
                    println!("❌ Simple emoji heading failed: {:?}", e);
                }
            }
        }

        // Test emoji heading with following content (multiline)
        {
            as_str_slice_test_case!(input2, "## Heading 😀", "", "Next line content");
            let result2 = parse_line_heading_no_advance_ng(input2);
            match result2 {
                Ok((remainder, heading_data)) => {
                    println!("✅ Multiline emoji heading parsed successfully:");
                    println!("   Level: {}", heading_data.level.level);
                    println!("   Text: '{}'", heading_data.text);
                    println!("   Remainder: '{}'", remainder.to_string());
                    println!("   Remainder is empty: {}", remainder.is_empty());
                    println!(
                        "   Remainder line index: {}",
                        remainder.line_index.as_usize()
                    );
                    println!("   Total lines: {}", remainder.lines.len());
                }
                Err(e) => {
                    println!("❌ Multiline emoji heading failed: {:?}", e);
                }
            }
        }
    }

    #[test]
    fn test_debug_main_parser_emoji_issue() {
        use crate::{as_str_slice_test_case, parse_markdown_ng};

        // Test the exact problematic case from our comprehensive test
        {
            as_str_slice_test_case!(input, "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀", "", "1. line 1 of 2");

            println!("=== INPUT ===");
            println!("Total lines: {}", input.lines.len());
            for (i, line) in input.lines.iter().enumerate() {
                println!("Line {}: '{}'", i, line.as_ref());
            }
            println!("=============");

            let result = parse_markdown_ng(input);
            match result {
                Ok((remainder, document)) => {
                    println!("✅ NG parser succeeded:");
                    println!("   Document elements: {}", document.len());
                    for (i, element) in document.iter().enumerate() {
                        println!("   Element {}: {:?}", i, element);
                    }
                    println!("   Remainder: '{}'", remainder.to_string());
                    println!("   Remainder is empty: {}", remainder.is_empty());
                    println!(
                        "   Remainder line index: {}",
                        remainder.line_index.as_usize()
                    );
                    println!("   Total remainder lines: {}", remainder.lines.len());
                }
                Err(e) => {
                    println!("❌ NG parser failed: {:?}", e);
                }
            }
        }
    }

    #[test]
    fn test_debug_line_by_line_parsing() {
        use crate::{as_str_slice_test_case,
                    parse_block_smart_list_advance_ng,
                    parse_line_empty_advance_ng,
                    parse_line_heading_no_advance_ng,
                    parse_markdown_ng};

        // Test step by step what happens after heading parsing
        {
            as_str_slice_test_case!(input, "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀", "", "1. line 1 of 2");

            println!("=== STEP BY STEP PARSING ===");
            println!("Input has {} lines", input.lines.len());

            // Step 1: Parse heading
            println!("\n1. Parsing heading...");
            let (remainder_after_heading, heading) =
                parse_line_heading_no_advance_ng(input).unwrap();
            println!("   Heading: {:?}", heading);
            println!(
                "   After heading - line_index: {}, total_lines: {}",
                remainder_after_heading.line_index.as_usize(),
                remainder_after_heading.lines.len()
            );
            println!(
                "   Current line: '{}'",
                remainder_after_heading
                    .get_current_line()
                    .unwrap_or("<none>")
            );

            // Step 2: Parse empty line
            println!("\n2. Parsing empty line...");
            let empty_result = parse_line_empty_advance_ng(remainder_after_heading);
            match empty_result {
                Ok((remainder_after_empty, empty_fragments)) => {
                    println!("   Empty line parsed: {:?}", empty_fragments);
                    println!(
                        "   After empty - line_index: {}, total_lines: {}",
                        remainder_after_empty.line_index.as_usize(),
                        remainder_after_empty.lines.len()
                    );
                    println!(
                        "   Current line: '{}'",
                        remainder_after_empty.get_current_line().unwrap_or("<none>")
                    );

                    // Step 3: Parse list
                    println!("\n3. Parsing list...");
                    let list_result =
                        parse_block_smart_list_advance_ng(remainder_after_empty);
                    match list_result {
                        Ok((remainder_after_list, list)) => {
                            println!("   List parsed: {:?}", list);
                            println!(
                                "   After list - line_index: {}, total_lines: {}",
                                remainder_after_list.line_index.as_usize(),
                                remainder_after_list.lines.len()
                            );
                            println!(
                                "   Remainder is_empty: {}",
                                remainder_after_list.is_empty()
                            );
                        }
                        Err(e) => {
                            println!("   ❌ List parsing failed: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Empty line parsing failed: {:?}", e);
                }
            }
        }
    }

    #[test]
    fn test_debug_heading_parser_advance() {
        use crate::{as_str_slice_test_case, parse_line_heading_no_advance_ng};

        println!("=== Testing main heading parser advancement ===");

        // Test heading followed by empty line and content
        as_str_slice_test_case!(input, "# Heading 😀", "", "Next line");
        println!("Input lines: {}", input.lines.len());
        for (i, line) in input.lines.iter().enumerate() {
            println!("  Line {}: '{}'", i, line.as_ref());
        }

        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Ok((remainder, heading_data)) => {
                println!(
                    "✅ Heading parsed: level={:?}, text='{}'",
                    heading_data.level, heading_data.text
                );
                println!(
                    "   Remainder line index: {}",
                    remainder.line_index.as_usize()
                );
                println!(
                    "   Remainder char index: {}",
                    remainder.char_index.as_usize()
                );
                println!("   Remainder is empty: {}", remainder.is_empty());

                if let Some(current_line) = remainder.get_current_line() {
                    println!("   Current line after parsing: '{}'", current_line);
                } else {
                    println!("   No current line - parser went past all lines");
                }

                // Show what's left to parse
                println!("   Full remainder: '{}'", remainder.to_string());
            }
            Err(e) => {
                println!("❌ Heading parsing failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_debug_heading_parser_detailed() {
        use crate::{as_str_slice_test_case,
                    parse_line_empty_advance_ng,
                    parse_line_heading_no_advance_ng};

        println!("=== Detailed heading parser investigation ===");

        // Test heading followed by empty line and content
        as_str_slice_test_case!(input, "# Heading 😀", "", "Next line");
        println!("Input representation: '{}'", input.to_string());
        println!("Input lines: {}", input.lines.len());
        for (i, line) in input.lines.iter().enumerate() {
            println!("  Line {}: '{}'", i, line.as_ref());
        }
        println!(
            "Starting position: line={}, char={}",
            input.line_index.as_usize(),
            input.char_index.as_usize()
        );

        // Parse the heading
        let result = parse_line_heading_no_advance_ng(input);
        match result {
            Ok((after_heading, heading_data)) => {
                println!(
                    "✅ Heading parsed: level={:?}, text='{}'",
                    heading_data.level, heading_data.text
                );
                println!(
                    "   After heading - line index: {}",
                    after_heading.line_index.as_usize()
                );
                println!(
                    "   After heading - char index: {}",
                    after_heading.char_index.as_usize()
                );
                println!("   After heading - is empty: {}", after_heading.is_empty());
                println!(
                    "   After heading - full content: '{}'",
                    after_heading.to_string()
                );

                if let Some(current_line) = after_heading.get_current_line() {
                    println!("   After heading - current line: '{}'", current_line);
                } else {
                    println!("   After heading - no current line");
                }

                // Now try to parse the empty line
                if !after_heading.is_empty() {
                    println!("\n--- Trying to parse empty line ---");
                    let empty_result = parse_line_empty_advance_ng(after_heading);
                    match empty_result {
                        Ok((after_empty, _)) => {
                            println!("✅ Empty line parsed");
                            println!(
                                "   After empty - line index: {}",
                                after_empty.line_index.as_usize()
                            );
                            println!(
                                "   After empty - char index: {}",
                                after_empty.char_index.as_usize()
                            );
                            println!(
                                "   After empty - is empty: {}",
                                after_empty.is_empty()
                            );
                            println!(
                                "   After empty - full content: '{}'",
                                after_empty.to_string()
                            );

                            if let Some(current_line) = after_empty.get_current_line() {
                                println!(
                                    "   After empty - current line: '{}'",
                                    current_line
                                );
                            } else {
                                println!("   After empty - no current line");
                            }
                        }
                        Err(e) => {
                            println!("❌ Empty line parsing failed: {:?}", e);
                        }
                    }
                } else {
                    println!("❌ No content left after heading to parse empty line");
                }
            }
            Err(e) => {
                println!("❌ Heading parsing failed: {:?}", e);
            }
        }
    }

    // TODO: Fix these debug tests - they have method signature issues
    // #[test]
    // fn test_debug_as_str_slice_advancement() {
    //     // This test needs to be fixed - AsStrSlice doesn't have peek_char/advance_char
    // methods }

    // #[test]
    // fn test_debug_tag_newline() {
    //     // This test needs to be fixed - advance() doesn't return a value
    // }
}
