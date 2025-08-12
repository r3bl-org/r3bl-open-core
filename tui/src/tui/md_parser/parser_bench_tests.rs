// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ## Legacy Parser Benchmark Suite
//!
//! This module contains performance benchmarks for the legacy markdown parser
//! using ALL the comprehensive test data from `conformance_test_data`.
//!
//! ## Running the Benchmarks
//!
//! Since the project uses nightly Rust (configured in rust-toolchain.toml),
//! you can run benchmarks directly:
//!
//! ```bash
//! # Run all benchmarks in the project
//! cargo bench
//!
//! # Run benchmarks for the r3bl_tui package specifically
//! cargo bench --package r3bl_tui
//!
//! # Run benchmarks matching a specific pattern
//! cargo bench bench_small
//! cargo bench bench_medium
//! cargo bench bench_large
//! ```
//!
//! ## Benchmark Categories
//!
//! The benchmarks test the same conformance data used in snapshot tests:
//! - **Small inputs**: Basic markdown elements (empty strings, single lines, simple
//!   formatting)
//! - **Medium inputs**: Multi-paragraph documents, lists, code blocks
//! - **Large inputs**: Complex nested documents from real-world usage
//! - **Invalid inputs**: Malformed syntax to test error handling
//! - **Jumbo inputs**: Real-world files for performance testing at scale
//!
//! Each benchmark measures the time to parse the input completely, including
//! all allocations and processing. The results help identify performance
//! regressions and optimization opportunities.

#[cfg(test)]
mod benchmarks {
    extern crate test;
    use test::Bencher;

    use crate::{ZeroCopyGapBuffer, md_parser::conformance_test_data::*, parse_markdown};

    // Helper macro to reduce boilerplate in benchmarks
    macro_rules! bench_parser {
        ($input:expr) => {{
            let gap_buffer = ZeroCopyGapBuffer::from($input);
            let _unused = parse_markdown(&gap_buffer);
        }};
    }

    // =============================================================================
    // Small valid input benchmarks
    // =============================================================================

    #[bench]
    fn bench_small_empty_string(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMPTY_STRING);
        });
    }

    #[bench]
    fn bench_small_only_newlines(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(ONLY_NEWLINES);
        });
    }

    #[bench]
    fn bench_small_single_line_no_newline(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(SINGLE_LINE_NO_NEWLINE);
        });
    }

    #[bench]
    fn bench_small_single_line_with_newline(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(SINGLE_LINE_WITH_NEWLINE);
        });
    }

    #[bench]
    fn bench_small_bold_text(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(BOLD_TEXT);
        });
    }

    #[bench]
    fn bench_small_italic_text(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(ITALIC_TEXT);
        });
    }

    #[bench]
    fn bench_small_mixed_formatting(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(MIXED_FORMATTING);
        });
    }

    #[bench]
    fn bench_small_simple_inline_code(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(SIMPLE_INLINE_CODE);
        });
    }

    #[bench]
    fn bench_small_inline_code_variations(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(INLINE_CODE_VARIATIONS);
        });
    }

    #[bench]
    fn bench_small_inline_code_with_unicode(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(INLINE_CODE_WITH_UNICODE);
        });
    }

    #[bench]
    fn bench_small_links(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(LINKS);
        });
    }

    #[bench]
    fn bench_small_images(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(IMAGES);
        });
    }

    #[bench]
    fn bench_small_metadata_title(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(METADATA_TITLE);
        });
    }

    #[bench]
    fn bench_small_metadata_tags(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(METADATA_TAGS);
        });
    }

    #[bench]
    fn bench_small_metadata_authors(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(METADATA_AUTHORS);
        });
    }

    #[bench]
    fn bench_small_metadata_date(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(METADATA_DATE);
        });
    }

    #[bench]
    fn bench_small_special_characters(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(SPECIAL_CHARACTERS);
        });
    }

    #[bench]
    fn bench_small_unicode_content(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(UNICODE_CONTENT);
        });
    }

    #[bench]
    fn bench_small_emoji_h1_simple(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMOJI_H1_SIMPLE);
        });
    }

    #[bench]
    fn bench_small_emoji_h2_simple(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMOJI_H2_SIMPLE);
        });
    }

    #[bench]
    fn bench_small_emoji_multiple(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMOJI_MULTIPLE);
        });
    }

    // =============================================================================
    // Medium valid input benchmarks
    // =============================================================================

    #[bench]
    fn bench_medium_multiple_lines(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(MULTIPLE_LINES);
        });
    }

    #[bench]
    fn bench_medium_heading_basic(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(HEADING_BASIC);
        });
    }

    #[bench]
    fn bench_medium_multiple_headings(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(MULTIPLE_HEADINGS);
        });
    }

    #[bench]
    fn bench_medium_all_heading_levels(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(ALL_HEADING_LEVELS);
        });
    }

    #[bench]
    fn bench_medium_unordered_list_simple(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(UNORDERED_LIST_SIMPLE);
        });
    }

    #[bench]
    fn bench_medium_ordered_list_simple(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(ORDERED_LIST_SIMPLE);
        });
    }

    #[bench]
    fn bench_medium_nested_unordered_list(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(NESTED_UNORDERED_LIST);
        });
    }

    #[bench]
    fn bench_medium_nested_ordered_list(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(NESTED_ORDERED_LIST);
        });
    }

    #[bench]
    fn bench_medium_code_block_rust(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(CODE_BLOCK_RUST);
        });
    }

    #[bench]
    fn bench_medium_code_block_bash(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(CODE_BLOCK_BASH);
        });
    }

    #[bench]
    fn bench_medium_code_block_no_language(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(CODE_BLOCK_NO_LANGUAGE);
        });
    }

    #[bench]
    fn bench_medium_empty_code_block(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMPTY_CODE_BLOCK);
        });
    }

    #[bench]
    fn bench_medium_mixed_list_types(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(MIXED_LIST_TYPES);
        });
    }

    #[bench]
    fn bench_medium_checkboxes(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(CHECKBOXES);
        });
    }

    #[bench]
    fn bench_medium_formatting_edge_cases(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(FORMATTING_EDGE_CASES);
        });
    }

    #[bench]
    fn bench_medium_nested_formatting(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(NESTED_FORMATTING);
        });
    }

    #[bench]
    fn bench_medium_edge_case_empty_lines(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EDGE_CASE_EMPTY_LINES);
        });
    }

    #[bench]
    fn bench_medium_edge_case_whitespace_lines(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EDGE_CASE_WHITESPACE_LINES);
        });
    }

    #[bench]
    fn bench_medium_edge_case_trailing_spaces(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EDGE_CASE_TRAILING_SPACES);
        });
    }

    #[bench]
    fn bench_medium_emoji_start_middle_end(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EMOJI_START_MIDDLE_END);
        });
    }

    #[bench]
    fn bench_medium_blog_post_document(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(BLOG_POST_DOCUMENT);
        });
    }

    // =============================================================================
    // Large valid input benchmarks
    // =============================================================================

    #[bench]
    fn bench_large_complex_nested_document(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(COMPLEX_NESTED_DOCUMENT);
        });
    }

    #[bench]
    fn bench_large_tutorial_document(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(TUTORIAL_DOCUMENT);
        });
    }

    // =============================================================================
    // Invalid input benchmarks
    // =============================================================================

    #[bench]
    fn bench_invalid_malformed_syntax(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(MALFORMED_SYNTAX);
        });
    }

    #[bench]
    fn bench_invalid_unclosed_formatting(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(UNCLOSED_FORMATTING);
        });
    }

    // =============================================================================
    // Jumbo/Real world input benchmarks
    // =============================================================================

    #[bench]
    fn bench_small_real_world_content(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(SMALL_REAL_WORLD_CONTENT);
        });
    }

    #[bench]
    fn bench_small_ex_editor_content(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(EX_EDITOR_CONTENT);
        });
    }

    #[bench]
    fn bench_jumbo_real_world_editor(b: &mut Bencher) {
        b.iter(|| {
            bench_parser!(REAL_WORLD_EDITOR_CONTENT);
        });
    }
}
