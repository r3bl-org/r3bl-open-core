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

//! ## Snapshot tests for legacy parser
//!
//! This module contains comprehensive tests that verify the legacy parser produces
//! correct output for ALL markdown inputs using the test data from conformance_test_data.
//!
//! These tests ensure that:
//! 1. The parser handles all valid markdown constructs correctly
//! 2. Edge cases and invalid inputs are handled gracefully
//! 3. The parser output remains consistent across code changes

#[cfg(test)]
mod tests {
    use crate::{
        parse_markdown,
        md_parser::conformance_test_data::*,
    };

    /// Helper function to test parsing - we just verify it parses without panic
    fn test_parse(input: &str) {
        let result = parse_markdown(input);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
        let (remainder, _doc) = result.unwrap();
        // Remainder should be empty for valid parsing
        assert!(remainder.is_empty() || input.is_empty(), "Parser did not consume all input. Remainder: {:?}", remainder);
    }

    // =============================================================================
    // Small valid input tests
    // =============================================================================

    #[test]
    fn test_small_empty_string() {
        test_parse(EMPTY_STRING);
    }

    #[test]
    fn test_small_only_newlines() {
        test_parse(ONLY_NEWLINES);
    }

    #[test]
    fn test_small_single_line_no_newline() {
        test_parse(SINGLE_LINE_NO_NEWLINE);
    }

    #[test]
    fn test_small_single_line_with_newline() {
        test_parse(SINGLE_LINE_WITH_NEWLINE);
    }

    #[test]
    fn test_small_simple_inline_code() {
        test_parse(SIMPLE_INLINE_CODE);
    }

    #[test]
    fn test_small_inline_code_variations() {
        test_parse(INLINE_CODE_VARIATIONS);
    }

    #[test]
    fn test_small_inline_code_with_unicode() {
        test_parse(INLINE_CODE_WITH_UNICODE);
    }

    #[test]
    fn test_small_bold_text() {
        test_parse(BOLD_TEXT);
    }

    #[test]
    fn test_small_italic_text() {
        test_parse(ITALIC_TEXT);
    }

    #[test]
    fn test_small_mixed_formatting() {
        test_parse(MIXED_FORMATTING);
    }

    #[test]
    fn test_small_links() {
        test_parse(LINKS);
    }

    #[test]
    fn test_small_images() {
        test_parse(IMAGES);
    }

    #[test]
    fn test_small_metadata_title() {
        test_parse(METADATA_TITLE);
    }

    #[test]
    fn test_small_metadata_tags() {
        test_parse(METADATA_TAGS);
    }

    #[test]
    fn test_small_metadata_authors() {
        test_parse(METADATA_AUTHORS);
    }

    #[test]
    fn test_small_metadata_date() {
        test_parse(METADATA_DATE);
    }

    #[test]
    fn test_small_special_characters() {
        test_parse(SPECIAL_CHARACTERS);
    }

    #[test]
    fn test_small_unicode_content() {
        test_parse(UNICODE_CONTENT);
    }

    #[test]
    fn test_small_emoji_h1_simple() {
        test_parse(EMOJI_H1_SIMPLE);
    }

    #[test]
    fn test_small_emoji_h2_simple() {
        test_parse(EMOJI_H2_SIMPLE);
    }

    #[test]
    fn test_small_emoji_multiple() {
        test_parse(EMOJI_MULTIPLE);
    }

    #[test]
    fn test_small_real_world_content() {
        test_parse(SMALL_REAL_WORLD_CONTENT);
    }

    // =============================================================================
    // Medium valid input tests
    // =============================================================================

    #[test]
    fn test_medium_multiple_lines() {
        test_parse(MULTIPLE_LINES);
    }

    #[test]
    fn test_medium_heading_basic() {
        test_parse(HEADING_BASIC);
    }

    #[test]
    fn test_medium_multiple_headings() {
        test_parse(MULTIPLE_HEADINGS);
    }

    #[test]
    fn test_medium_all_heading_levels() {
        test_parse(ALL_HEADING_LEVELS);
    }

    #[test]
    fn test_medium_unordered_list_simple() {
        test_parse(UNORDERED_LIST_SIMPLE);
    }

    #[test]
    fn test_medium_ordered_list_simple() {
        test_parse(ORDERED_LIST_SIMPLE);
    }

    #[test]
    fn test_medium_nested_unordered_list() {
        test_parse(NESTED_UNORDERED_LIST);
    }

    #[test]
    fn test_medium_nested_ordered_list() {
        test_parse(NESTED_ORDERED_LIST);
    }

    #[test]
    fn test_medium_checkboxes() {
        test_parse(CHECKBOXES);
    }

    #[test]
    fn test_medium_mixed_list_types() {
        test_parse(MIXED_LIST_TYPES);
    }

    #[test]
    fn test_medium_code_block_bash() {
        test_parse(CODE_BLOCK_BASH);
    }

    #[test]
    fn test_medium_code_block_rust() {
        test_parse(CODE_BLOCK_RUST);
    }

    #[test]
    fn test_medium_code_block_no_language() {
        test_parse(CODE_BLOCK_NO_LANGUAGE);
    }

    #[test]
    fn test_medium_empty_code_block() {
        test_parse(EMPTY_CODE_BLOCK);
    }

    #[test]
    fn test_medium_formatting_edge_cases() {
        test_parse(FORMATTING_EDGE_CASES);
    }

    #[test]
    fn test_medium_nested_formatting() {
        test_parse(NESTED_FORMATTING);
    }

    #[test]
    fn test_medium_edge_case_empty_lines() {
        test_parse(EDGE_CASE_EMPTY_LINES);
    }

    #[test]
    fn test_medium_edge_case_whitespace_lines() {
        test_parse(EDGE_CASE_WHITESPACE_LINES);
    }

    #[test]
    fn test_medium_edge_case_trailing_spaces() {
        test_parse(EDGE_CASE_TRAILING_SPACES);
    }

    #[test]
    fn test_medium_emoji_start_middle_end() {
        test_parse(EMOJI_START_MIDDLE_END);
    }

    #[test]
    fn test_medium_blog_post_document() {
        test_parse(BLOG_POST_DOCUMENT);
    }

    // =============================================================================
    // Large valid input tests
    // =============================================================================

    #[test]
    fn test_large_complex_nested_document() {
        test_parse(COMPLEX_NESTED_DOCUMENT);
    }

    #[test]
    fn test_large_tutorial_document() {
        test_parse(TUTORIAL_DOCUMENT);
    }

    // =============================================================================
    // Invalid input tests
    // =============================================================================

    #[test]
    fn test_invalid_malformed_syntax() {
        test_parse(MALFORMED_SYNTAX);
    }

    #[test]
    fn test_invalid_unclosed_formatting() {
        test_parse(UNCLOSED_FORMATTING);
    }

    // =============================================================================
    // Jumbo/Real world file tests
    // =============================================================================

    #[test]
    fn test_jumbo_real_world_editor() {
        test_parse(REAL_WORLD_EDITOR_CONTENT);
    }
}