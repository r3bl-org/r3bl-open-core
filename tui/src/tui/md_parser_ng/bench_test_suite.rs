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
//! - **`f_materialization_cost_isolation_*`**: Isolating materialization costs
//! - **`g_character_access_patterns_*`**: Character access pattern benchmarks
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
//! cargo bench bench_f  # Materialization cost isolation benchmarks
//! cargo bench bench_g  # Character access pattern benchmarks
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

// =============================================================================
// MATERIALIZATION COST ISOLATION BENCHMARKS - Measure only the cost of converting
// &[GCString] to String
// =============================================================================

/// Benchmark the cost of materializing `&[GCString]` to `String` for small content
#[bench]
fn bench_f_materialization_small_real_world(b: &mut Bencher) {
    let content = SMALL_REAL_WORLD_CONTENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let _materialized = gcs_lines
            .iter()
            .map(GCString::as_ref)
            .collect::<Vec<_>>()
            .join("\n");
    });
}

/// Benchmark the cost of materializing `&[GCString]` to `String` for medium content
#[bench]
fn bench_f_materialization_medium_blog_post(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let _materialized = gcs_lines
            .iter()
            .map(GCString::as_ref)
            .collect::<Vec<_>>()
            .join("\n");
    });
}

/// Benchmark the cost of materializing `&[GCString]` to `String` for large content
#[bench]
fn bench_f_materialization_large_complex_document(b: &mut Bencher) {
    let content = COMPLEX_NESTED_DOCUMENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let _materialized = gcs_lines
            .iter()
            .map(GCString::as_ref)
            .collect::<Vec<_>>()
            .join("\n");
    });
}

/// Benchmark the cost of materializing `&[GCString]` to `String` for jumbo content
#[bench]
fn bench_f_materialization_jumbo_api_documentation(b: &mut Bencher) {
    let content = REAL_WORLD_EDITOR_CONTENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let _materialized = gcs_lines
            .iter()
            .map(GCString::as_ref)
            .collect::<Vec<_>>()
            .join("\n");
    });
}

/// Benchmark the cost of materializing highly fragmented `GCString` array
#[bench]
fn bench_f_materialization_highly_fragmented(b: &mut Bencher) {
    // Create many small GCString fragments to simulate worst-case fragmentation
    let fragments: Vec<GCString> = (0..1000)
        .map(|i| GCString::new(format!("line {i}")))
        .collect();

    b.iter(|| {
        let _materialized = fragments
            .iter()
            .map(GCString::as_ref)
            .collect::<Vec<_>>()
            .join("\n");
    });
}

// =============================================================================
// CHARACTER ACCESS PATTERN BENCHMARKS - Compare contiguous vs non-contiguous character
// access =============================================================================

/// Benchmark sequential character iteration over a materialized String
#[bench]
fn bench_g_char_access_string_small(b: &mut Bencher) {
    let content = SMALL_REAL_WORLD_CONTENT;

    b.iter(|| {
        let mut char_count = 0;
        for ch in content.chars() {
            if ch.is_alphabetic() {
                char_count += 1;
            }
        }
        char_count
    });
}

/// Benchmark sequential character iteration over `&[GCString]`
#[bench]
fn bench_g_char_access_gcstring_small(b: &mut Bencher) {
    let content = SMALL_REAL_WORLD_CONTENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let mut char_count = 0;
        for line in &gcs_lines {
            for ch in line {
                for ch_char in ch.chars() {
                    if ch_char.is_alphabetic() {
                        char_count += 1;
                    }
                }
            }
        }
        char_count
    });
}

/// Benchmark sequential character iteration over a materialized String (medium content)
#[bench]
fn bench_g_char_access_string_medium(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;

    b.iter(|| {
        let mut char_count = 0;
        for ch in content.chars() {
            if ch.is_alphabetic() {
                char_count += 1;
            }
        }
        char_count
    });
}

/// Benchmark sequential character iteration over `&[GCString]` (medium content)
#[bench]
fn bench_g_char_access_gcstring_medium(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let mut char_count = 0;
        for line in &gcs_lines {
            for ch in line {
                for ch_char in ch.chars() {
                    if ch_char.is_alphabetic() {
                        char_count += 1;
                    }
                }
            }
        }
        char_count
    });
}

/// Benchmark random character access patterns over a materialized String
#[bench]
fn bench_g_char_access_random_string(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    let chars: Vec<char> = content.chars().collect();

    b.iter(|| {
        let mut found_count = 0;
        // Simulate parser looking for specific characters at various positions
        for i in (0..chars.len()).step_by(10) {
            if let Some(&c) = chars.get(i) {
                if c == ' ' || c == '\n' || c == '*' {
                    found_count += 1;
                }
            }
        }
        found_count
    });
}

/// Benchmark boundary-heavy access patterns (line transitions) with `&[GCString]`
#[bench]
fn bench_g_char_access_boundary_heavy_gcstring(b: &mut Bencher) {
    let content = BLOG_POST_DOCUMENT;
    let gcs_lines: Vec<GCString> = content.lines().map(GCString::from).collect();

    b.iter(|| {
        let mut boundary_count = 0;
        for (line_idx, line) in gcs_lines.iter().enumerate() {
            let line_str = line.as_ref();
            // Simulate parser checking line boundaries and looking ahead
            if line_str.starts_with('#')
                || line_str.starts_with('-')
                || line_str.starts_with('*')
            {
                boundary_count += 1;
            }
            // Simulate looking ahead to next line
            if line_idx + 1 < gcs_lines.len() {
                let next_line = &gcs_lines[line_idx + 1];
                if next_line.as_ref().is_empty() {
                    boundary_count += 1;
                }
            }
        }
        boundary_count
    });
}

/// Benchmark pattern matching across line boundaries with `&[GCString]` (worst case for
/// NG parser)
#[bench]
fn bench_g_char_access_cross_boundary_patterns(b: &mut Bencher) {
    // Create content with patterns that span multiple lines
    let multiline_content = vec![
        GCString::new("```rust"),
        GCString::new("fn main() {"),
        GCString::new("    println!(\"Hello\");"),
        GCString::new("}"),
        GCString::new("```"),
        GCString::new(""),
        GCString::new("Some text here"),
        GCString::new(""),
        GCString::new("```python"),
        GCString::new("def hello():"),
        GCString::new("    print('Hello')"),
        GCString::new("```"),
    ];

    b.iter(|| {
        let mut code_blocks = 0;
        let mut i = 0;
        while i < multiline_content.len() {
            let line = &multiline_content[i];
            let line_str = line.as_ref();
            if line_str.starts_with("```") {
                // Simulate finding the matching closing ```
                i += 1;
                while i < multiline_content.len()
                    && !multiline_content[i].as_ref().starts_with("```")
                {
                    i += 1;
                }
                if i < multiline_content.len() {
                    code_blocks += 1;
                }
            }
            i += 1;
        }
        code_blocks
    });
}
