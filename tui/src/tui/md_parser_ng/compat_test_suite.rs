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

//! ## Compatibility test suite
//!
//! This module contains tests that ensure the NG parser (`parse_markdown_ng`)
//! produces identical output to the legacy parser (`parse_markdown`) for all
//! markdown inputs, including challenging edge cases.
//!
//! ## Parser comparison with real world use cases
//!
//! There are two main paths for parsing markdown in the R3BL TUI editor:
//! 1. NG parser path: Convert &[`crate::GCString`] to [`AsStrSlice`] (no copy) ->
//!    [`crate::parse_markdown_ng()`]
//! 2. Legacy parser path: &[`crate::GCString`] -> materialized string (full copy) ->
//!    [`crate::parse_markdown()`]
//!
//! ## Module structure (Phase 1: Completed)
//!
//! The test suite is now organized in a modular structure:
//! - `compat_test_suite.rs`: Main test functions using external data
//! - `compat_test_data/`: External test input constants organized by complexity
//!   - `invalid_inputs.rs`: Malformed markdown and edge cases
//!   - `valid_small_inputs.rs`: Basic markdown constructs
//!   - `valid_medium_inputs.rs`: Multi-line content with moderate complexity
//!   - `valid_large_inputs.rs`: Complex documents with nested structures
//!   - `valid_jumbo_inputs.rs`: Real-world large files for performance testing
//!
//! ## Ready for Phase 2 & 3
//!
//! - **Phase 2**: Add real-world markdown files using `include_str!` macro
//! - **Phase 3**: Add `#[bench]` functions for `cargo bench` performance comparison
//!
//! All test functionality has been preserved during the modularization.

// Import the external test data constants
use super::compat_test_data::*;
use crate::{get_real_world_editor_content, parse_markdown, parse_markdown_ng,
            AsStrSlice, GCString, ParserByteCache};

/// Helper function to test compatibility between `parse_markdown` and
/// `parse_markdown_ng` This simulates the real-world usage in
/// `try_parse_and_highlight` where both parsers start from the same &[`GCString`]
/// source but take different paths.
fn test_compat_helper(test_name: &str, input_str: &str) {
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

// =============================================================================
// INVALID INPUTS - Edge cases and malformed syntax
// =============================================================================

#[test]
fn test_malformed_syntax() { test_compat_helper("malformed_syntax", MALFORMED_SYNTAX); }

#[test]
fn test_unclosed_formatting() {
    test_compat_helper("unclosed_formatting", UNCLOSED_FORMATTING);
}

// =============================================================================
// VALID SMALL INPUTS - Basic markdown constructs
// =============================================================================

#[test]
fn test_empty_string() { test_compat_helper("empty_string", EMPTY_STRING); }

#[test]
fn test_only_newlines() { test_compat_helper("only_newlines", ONLY_NEWLINES); }

#[test]
fn test_single_line_no_newline() {
    test_compat_helper("single_line_no_newline", SINGLE_LINE_NO_NEWLINE);
}

#[test]
fn test_single_line_with_newline() {
    test_compat_helper("single_line_with_newline", SINGLE_LINE_WITH_NEWLINE);
}

#[test]
fn test_simple_inline_code() {
    test_compat_helper("simple_inline_code", SIMPLE_INLINE_CODE);
}

#[test]
fn test_inline_code_variations() {
    test_compat_helper("inline_code_variations", INLINE_CODE_VARIATIONS);
}

#[test]
fn test_inline_code_with_unicode() {
    test_compat_helper("inline_code_with_unicode", INLINE_CODE_WITH_UNICODE);
}

#[test]
fn test_bold_text() { test_compat_helper("bold_text", BOLD_TEXT); }

#[test]
fn test_italic_text() { test_compat_helper("italic_text", ITALIC_TEXT); }

#[test]
fn test_mixed_formatting() { test_compat_helper("mixed_formatting", MIXED_FORMATTING); }

#[test]
fn test_links() { test_compat_helper("links", LINKS); }

#[test]
fn test_images() { test_compat_helper("images", IMAGES); }

#[test]
fn test_metadata_title() { test_compat_helper("metadata_title", METADATA_TITLE); }

#[test]
fn test_metadata_tags() { test_compat_helper("metadata_tags", METADATA_TAGS); }

#[test]
fn test_metadata_authors() { test_compat_helper("metadata_authors", METADATA_AUTHORS); }

#[test]
fn test_metadata_date() { test_compat_helper("metadata_date", METADATA_DATE); }

#[test]
fn test_special_characters() {
    test_compat_helper("special_characters", SPECIAL_CHARACTERS);
}

#[test]
fn test_unicode_content() { test_compat_helper("unicode_content", UNICODE_CONTENT); }

// =============================================================================
// VALID MEDIUM INPUTS - Multi-line and structured content
// =============================================================================

#[test]
fn test_multiple_lines() { test_compat_helper("multiple_lines", MULTIPLE_LINES); }

#[test]
fn test_heading_basic() { test_compat_helper("heading_basic", HEADING_BASIC); }

#[test]
fn test_multiple_headings() {
    test_compat_helper("multiple_headings", MULTIPLE_HEADINGS);
}

#[test]
fn test_all_heading_levels() {
    test_compat_helper("all_heading_levels", ALL_HEADING_LEVELS);
}

#[test]
fn test_unordered_list_simple() {
    test_compat_helper("unordered_list_simple", UNORDERED_LIST_SIMPLE);
}

#[test]
fn test_ordered_list_simple() {
    test_compat_helper("ordered_list_simple", ORDERED_LIST_SIMPLE);
}

#[test]
fn test_nested_unordered_list() {
    test_compat_helper("nested_unordered_list", NESTED_UNORDERED_LIST);
}

#[test]
fn test_nested_ordered_list() {
    test_compat_helper("nested_ordered_list", NESTED_ORDERED_LIST);
}

#[test]
fn test_checkboxes() { test_compat_helper("checkboxes", CHECKBOXES); }

#[test]
fn test_mixed_list_types() { test_compat_helper("mixed_list_types", MIXED_LIST_TYPES); }

#[test]
fn test_code_block_bash() { test_compat_helper("code_block_bash", CODE_BLOCK_BASH); }

#[test]
fn test_code_block_rust() { test_compat_helper("code_block_rust", CODE_BLOCK_RUST); }

#[test]
fn test_code_block_no_language() {
    test_compat_helper("code_block_no_language", CODE_BLOCK_NO_LANGUAGE);
}

#[test]
fn test_empty_code_block() { test_compat_helper("empty_code_block", EMPTY_CODE_BLOCK); }

#[test]
fn test_formatting_edge_cases() {
    test_compat_helper("formatting_edge_cases", FORMATTING_EDGE_CASES);
}

#[test]
fn test_nested_formatting() {
    test_compat_helper("nested_formatting", NESTED_FORMATTING);
}

#[test]
fn test_edge_case_empty_lines() {
    test_compat_helper("edge_case_empty_lines", EDGE_CASE_EMPTY_LINES);
}

#[test]
fn test_edge_case_whitespace_lines() {
    test_compat_helper("edge_case_whitespace_lines", EDGE_CASE_WHITESPACE_LINES);
}

#[test]
fn test_edge_case_trailing_spaces() {
    test_compat_helper("edge_case_trailing_spaces", EDGE_CASE_TRAILING_SPACES);
}

#[test]
fn test_emoji_start_middle_end() {
    test_compat_helper("emoji_start_middle_end", EMOJI_START_MIDDLE_END);
}

// =============================================================================
// VALID LARGE INPUTS - Complex documents
// =============================================================================

#[test]
fn test_blog_post_document() {
    test_compat_helper("blog_post_document", BLOG_POST_DOCUMENT);
}

#[test]
fn test_complex_nested_document() {
    test_compat_helper("complex_nested_document", COMPLEX_NESTED_DOCUMENT);
}

#[test]
fn test_tutorial_document() {
    test_compat_helper("tutorial_document", TUTORIAL_DOCUMENT);
}

// =============================================================================
// VALID JUMBO INPUTS - Real-world content (Phase 2 will expand these)
// =============================================================================

#[test]
fn test_comprehensive_document() {
    // Use real-world content from tui/examples/tui_apps/ex_editor/state.rs
    // This includes emojis in headings and other complex markdown features
    let comprehensive_input = get_real_world_editor_content().join("\n");
    test_compat_helper("comprehensive_document", &comprehensive_input);
}

#[test]
fn test_real_world_editor_content() {
    test_compat_helper("real_world_editor_content", REAL_WORLD_EDITOR_CONTENT);
}

#[test]
fn test_small_real_world_content() {
    test_compat_helper("small_real_world_content", SMALL_REAL_WORLD_CONTENT);
}

// =============================================================================
// COMPLEX TEST PATTERNS - Mixed content types
// =============================================================================

#[test]
fn test_complex_list_with_content() {
    let complex_list_input = r#"1. First item
   This is additional content for item 1

   More content with empty line

2. Second item
   - Nested unordered
   - Another nested
     With additional content
3. Back to ordered"#;
    test_compat_helper("complex_list_with_content", complex_list_input);
}

#[test]
fn test_code_blocks_in_lists() {
    let code_blocks_in_lists_input = r#"1. Install dependencies:
   ```bash
   cargo install my-tool
   ```
2. Run the tool:
   ```bash
   my-tool --help
   ```"#;
    test_compat_helper("code_blocks_in_lists", code_blocks_in_lists_input);
}

#[test]
fn test_complex_links() {
    let complex_links_input = r#"Various links:
- [Simple](https://example.com)
- [With title](https://example.com "Title")
- [Complex URL](https://example.com/path?param=value&other=test#section)
- ![Image link](https://example.com/image.png "Alt text")"#;
    test_compat_helper("complex_links", complex_links_input);
}

// =============================================================================
// EMOJI TESTS - Special handling for emoji in headings
// =============================================================================

#[test]
fn test_emoji_in_headings() {
    // Test simple emoji in H1 heading
    test_compat_helper("emoji_h1_simple", EMOJI_H1_SIMPLE);

    // Test emoji in H2 heading
    test_compat_helper("emoji_h2_simple", EMOJI_H2_SIMPLE);

    // Test multiple emojis in heading
    test_compat_helper("emoji_multiple", EMOJI_MULTIPLE);
}

#[test]
fn test_emoji_headings_with_content() {
    // Test the specific case from our real-world content that's failing
    test_compat_helper(
            "emoji_h2_long",
            "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀"
        );

    // Test emoji heading followed by content
    test_compat_helper(
        "emoji_heading_with_content",
        "# Heading 😀\nSome content below",
    );
}

#[test]
fn test_emoji_heading_in_multiline_context() {
    // Test the exact pattern from our comprehensive test that's failing
    test_compat_helper(
            "emoji_h2_with_following_content",
            "## Did he take those two new droids with him? They hit accelerator.😀 We will deal with your Rebel friends. Commence primary ignition.😀\n\n1. line 1 of 2"
        );

    // Simpler version to isolate the issue
    test_compat_helper("emoji_h2_with_list", "## Heading 😀\n\n1. List item");

    // Test with H1 to see if it's specific to H2
    test_compat_helper("emoji_h1_with_list", "# Heading 😀\n\n1. List item");
}
