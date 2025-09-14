// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Segment building utilities for grapheme clusters.
//!
//! This module provides functions to build segments from string slices, extracting
//! the core logic from [`GCStringOwned`](crate::GCStringOwned) for reuse in other
//! components like the gap buffer implementation.
//!
//! See the [module docs](crate::graphemes) for
//! comprehensive information about Unicode handling, grapheme clusters, and the three
//! types of indices used in this system.

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::{ColIndex, ColWidth, Seg, SegmentArray, ch, col, len, seg_index, width};

/// Build grapheme cluster segments for any string slice.
///
/// This function analyzes a UTF-8 string and creates a segment for each grapheme
/// cluster (user-perceived character). It includes an ASCII fast path for better
/// performance when dealing with ASCII-only text.
///
/// # Arguments
///
/// * `input` - A string slice to segment
///
/// # Returns
///
/// A [`SegmentArray`] containing one [`Seg`] for each grapheme cluster in the input
#[must_use]
pub fn build_segments_for_str(input: &str) -> SegmentArray {
    // ASCII fast path
    if input.is_ascii() {
        return build_ascii_segments(input);
    }

    let mut segments = SegmentArray::new();
    let mut byte_offset = 0;
    let mut display_col = 0;

    for (seg_idx, grapheme) in input.graphemes(true).enumerate() {
        let bytes_size = len(grapheme.len());
        let display_width = UnicodeWidthStr::width(grapheme);

        segments.push(Seg {
            start_byte_index: ch(byte_offset),
            end_byte_index: ch(byte_offset + bytes_size.as_usize()),
            display_width: width(display_width),
            seg_index: seg_index(seg_idx),
            bytes_size,
            start_display_col_index: col(display_col),
        });

        byte_offset += bytes_size.as_usize();
        display_col += display_width;
    }

    segments
}

/// Build segments for ASCII-only strings (optimized path).
///
/// Since ASCII characters are always 1 byte and 1 display column wide,
/// we can build segments more efficiently without Unicode analysis.
fn build_ascii_segments(input: &str) -> SegmentArray {
    let mut segments = SegmentArray::with_capacity(input.len());

    for (i, _) in input.char_indices() {
        segments.push(Seg {
            start_byte_index: ch(i),
            end_byte_index: ch(i + 1),
            display_width: width(1),
            seg_index: seg_index(i),
            bytes_size: len(1),
            start_display_col_index: col(i),
        });
    }

    segments
}

/// Calculate total display width from segments.
///
/// This sums up the display width of all segments to get the total
/// width of the string when rendered in a terminal.
#[must_use]
pub fn calculate_display_width(segments: &SegmentArray) -> ColWidth {
    segments
        .last()
        .map_or(/* None */ width(0), /* Some */ |seg| {
            let start_col: ColIndex = seg.start_display_col_index;
            let seg_width: ColWidth = seg.display_width;
            let end_col = *start_col + *seg_width;
            width(end_col)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_build_segments_ascii() {
        let input = "Hello";
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 5);
        assert_eq2!(calculate_display_width(&segments), width(5));

        // Check first segment 'H'.
        let seg = &segments[0];
        assert_eq2!(seg.start_byte_index, ch(0));
        assert_eq2!(seg.end_byte_index, ch(1));
        assert_eq2!(seg.display_width, width(1));
        assert_eq2!(seg.start_display_col_index, col(0));
    }

    #[test]
    fn test_build_segments_emoji() {
        let input = "H😀!";
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 3);
        assert_eq2!(calculate_display_width(&segments), width(4)); // H(1) + 😀(2) + !(1)

        // Check emoji segment.
        let emoji_seg = &segments[1];
        assert_eq2!(emoji_seg.start_byte_index, ch(1));
        assert_eq2!(emoji_seg.end_byte_index, ch(5)); // 4 bytes
        assert_eq2!(emoji_seg.display_width, width(2));
        assert_eq2!(emoji_seg.start_display_col_index, col(1));
    }

    #[test]
    fn test_build_segments_combining_chars() {
        // Using composed form to avoid clippy warning.
        let input = "café"; // é is composed
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 4);
        assert_eq2!(calculate_display_width(&segments), width(4));
    }

    #[test]
    fn test_build_segments_jumbo_emoji() {
        let input = "🙏🏽"; // Folded hands with skin tone
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 1); // Single grapheme cluster
        assert_eq2!(calculate_display_width(&segments), width(2));

        let seg = &segments[0];
        assert_eq2!(seg.bytes_size.as_usize(), 8); // 4 bytes for 🙏 + 4 bytes for 🏽
        assert_eq2!(seg.display_width, width(2));
    }

    #[test]
    fn test_calculate_display_width_empty() {
        let segments = SegmentArray::new();
        assert_eq2!(calculate_display_width(&segments), width(0));
    }
}

#[cfg(test)]
mod benches {
    use std::hint::black_box;

    use test::Bencher;

    use super::*;

    extern crate test;

    #[bench]
    fn bench_build_segments_ascii_short(b: &mut Bencher) {
        let input = "Hello, World!";
        b.iter(|| {
            black_box(build_segments_for_str(black_box(input)));
        });
    }

    #[bench]
    fn bench_build_segments_ascii_long(b: &mut Bencher) {
        let input = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.";
        b.iter(|| {
            black_box(build_segments_for_str(black_box(input)));
        });
    }

    #[bench]
    fn bench_build_segments_unicode_emoji(b: &mut Bencher) {
        let input = "Hello 😀 World 🌍 Test 🚀 Code 💻 Rust 🦀!";
        b.iter(|| {
            black_box(build_segments_for_str(black_box(input)));
        });
    }

    #[bench]
    fn bench_build_segments_unicode_mixed(b: &mut Bencher) {
        let input = "café münchen björk 北京 東京 🇺🇸🇬🇧 naïve résumé";
        b.iter(|| {
            black_box(build_segments_for_str(black_box(input)));
        });
    }

    #[bench]
    fn bench_build_segments_unicode_complex(b: &mut Bencher) {
        // Complex grapheme clusters with skin tone modifiers.
        let input = "👨🏾‍🤝‍👨🏿 Family: 👨‍👩‍👧‍👦 Emoji: 🙏🏽 Flag: 🏳️‍🌈";
        b.iter(|| {
            black_box(build_segments_for_str(black_box(input)));
        });
    }

    #[bench]
    fn bench_calculate_display_width_ascii(b: &mut Bencher) {
        let input = "Hello, World! This is a longer ASCII string for benchmarking.";
        let segments = build_segments_for_str(input);
        b.iter(|| {
            black_box(calculate_display_width(black_box(&segments)));
        });
    }

    #[bench]
    fn bench_calculate_display_width_unicode(b: &mut Bencher) {
        let input = "Hello 😀 World 🌍 Test 🚀 Code 💻 Rust 🦀!";
        let segments = build_segments_for_str(input);
        b.iter(|| {
            black_box(calculate_display_width(black_box(&segments)));
        });
    }
}
