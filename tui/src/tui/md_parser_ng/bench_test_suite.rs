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

//! ## Benchmark test suite
//!
//! This module contains performance benchmarks that compare the legacy parser
//! (`parse_markdown`) with the NG parser (`parse_markdown_ng`) across a wide
//! variety of markdown content types and sizes.
//!
//! ## Benchmark Organization
//!
//! Benchmarks are alphabetically grouped by content complexity for natural ordering:
//! - **`a_small_*`**: Basic content like empty strings and simple formatting
//! - **`b_medium_*`**: Multi-line content with moderate complexity
//! - **`c_large_*`**: Complex documents with nested structures
//! - **`d_jumbo_*`**: Real-world large content for stress testing
//! - **`e_unicode_*`**: Unicode and emoji content for encoding performance
//!
//! ## Usage
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench
//! ```
//!
//! Run specific benchmark groups:
//! ```bash
//! cargo bench bench_a  # Small content benchmarks
//! cargo bench bench_b  # Medium content benchmarks
//! cargo bench bench_c  # Large content benchmarks
//! cargo bench bench_d  # Jumbo content benchmarks
//! cargo bench bench_e  # Unicode content benchmarks
//! ```
//!
//! ## Related Modules
//!
//! - [`super::compat_test_suite`]: Compatibility tests ensuring identical parser output
//! - [`super::compat_test_data`]: Shared test data constants used by both test suites

#[cfg(test)]
use test::Bencher;

use super::compat_test_data::*;
#[cfg(test)]
use crate::{get_real_world_editor_content, parse_markdown, parse_markdown_ng,
            AsStrSlice, GCString};

// =============================================================================
// BENCHMARKS - Performance comparison between legacy and NG parsers
// =============================================================================

// Small content benchmarks (alphabetically ordered for natural grouping)

#[bench]
fn bench_a_small_empty_string_legacy(b: &mut Bencher) {
    let content = EMPTY_STRING;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_a_small_empty_string_ng(b: &mut Bencher) {
    let content = EMPTY_STRING;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_a_small_real_world_legacy(b: &mut Bencher) {
    let content = SMALL_REAL_WORLD_CONTENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_a_small_real_world_ng(b: &mut Bencher) {
    let content = SMALL_REAL_WORLD_CONTENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_a_small_simple_formatting_legacy(b: &mut Bencher) {
    let content = MIXED_FORMATTING;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_a_small_simple_formatting_ng(b: &mut Bencher) {
    let content = MIXED_FORMATTING;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

// Medium content benchmarks

#[bench]
fn bench_b_medium_blog_post_legacy(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_b_medium_blog_post_ng(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_b_medium_code_blocks_legacy(b: &mut Bencher) {
    let content = CODE_BLOCK_RUST;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_b_medium_code_blocks_ng(b: &mut Bencher) {
    let content = CODE_BLOCK_RUST;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_b_medium_nested_lists_legacy(b: &mut Bencher) {
    let content = NESTED_UNORDERED_LIST;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_b_medium_nested_lists_ng(b: &mut Bencher) {
    let content = NESTED_UNORDERED_LIST;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

// Large content benchmarks

#[bench]
fn bench_c_large_complex_document_legacy(b: &mut Bencher) {
    let content = COMPLEX_NESTED_DOCUMENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_c_large_complex_document_ng(b: &mut Bencher) {
    let content = COMPLEX_NESTED_DOCUMENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_c_large_tutorial_legacy(b: &mut Bencher) {
    let content = TUTORIAL_DOCUMENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_c_large_tutorial_ng(b: &mut Bencher) {
    let content = TUTORIAL_DOCUMENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

// Jumbo content benchmarks

#[bench]
fn bench_d_jumbo_api_documentation_legacy(b: &mut Bencher) {
    let content = REAL_WORLD_EDITOR_CONTENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_d_jumbo_api_documentation_ng(b: &mut Bencher) {
    let content = REAL_WORLD_EDITOR_CONTENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_d_jumbo_comprehensive_document_legacy(b: &mut Bencher) {
    let comprehensive_input = get_real_world_editor_content().join("\n");
    b.iter(|| {
        let _unused = parse_markdown(&comprehensive_input);
    });
}

#[bench]
fn bench_d_jumbo_comprehensive_document_ng(b: &mut Bencher) {
    let comprehensive_input = get_real_world_editor_content().join("\n");
    b.iter(|| {
        let gcs_lines: Vec<GCString> =
            comprehensive_input.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

// Unicode and emoji stress tests

#[bench]
fn bench_e_unicode_emoji_legacy(b: &mut Bencher) {
    let content = UNICODE_CONTENT;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_e_unicode_emoji_ng(b: &mut Bencher) {
    let content = UNICODE_CONTENT;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}

#[bench]
fn bench_e_unicode_emoji_headings_legacy(b: &mut Bencher) {
    let content = EMOJI_START_MIDDLE_END;
    b.iter(|| {
        let _unused = parse_markdown(content);
    });
}

#[bench]
fn bench_e_unicode_emoji_headings_ng(b: &mut Bencher) {
    let content = EMOJI_START_MIDDLE_END;
    b.iter(|| {
        let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();
        let ng_input = AsStrSlice::from(gcs_lines.as_slice());
        let _unused = parse_markdown_ng(ng_input);
    });
}
