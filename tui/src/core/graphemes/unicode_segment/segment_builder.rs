// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Segment building utilities for grapheme clusters.
//!
//! This module provides functions to build segments from string slices, extracting the
//! core logic from [`GCStringOwned`] for reuse in other components like the gap buffer
//! implementation.
//!
//! See the [module docs] for comprehensive information about Unicode handling, grapheme
//! clusters, and the three types of indices used in this system.
//!
//! [`GCStringOwned`]: crate::GCStringOwned
//! [module docs]: crate::graphemes

use crate::{CWidth, DocSeg, NarrowingCastToU16, Seg, SegmentArray, VPCol, VPWidth,
            byte_index, byte_len, c_col, c_index, c_width, seg_index, vp_col, vp_len,
            vp_width};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Builds grapheme cluster segments for any string slice.
///
/// This function analyzes a [`UTF-8`] string and creates a segment for each grapheme
/// cluster (user-perceived character). It includes an [`ASCII`] fast path for better
/// performance when dealing with [`ASCII`]-only text.
///
/// # Arguments
///
/// * `input` - A string slice to segment
///
/// # Returns
///
/// A [`SegmentArray`] containing one [`Seg`] for each grapheme cluster in the input
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
#[must_use]
pub fn build_segments_for_str(input: &str) -> SegmentArray {
    // ASCII fast path
    if input.is_ascii() {
        return build_ascii_segments(input);
    }

    let mut segments = SegmentArray::new();
    let mut byte_offset = 0;
    let mut display_col = 0u16;

    for (seg_idx, grapheme) in input.graphemes(true).enumerate() {
        let bytes_size = vp_len((grapheme.len()).as_u16_narrowing());
        let display_width = UnicodeWidthStr::width(grapheme);

        segments.push(Seg {
            start_byte_index: byte_index(byte_offset),
            end_byte_index: byte_index(byte_offset + bytes_size.as_usize()),
            display_width: vp_width((display_width).as_u16_narrowing()),
            seg_index: seg_index((seg_idx).as_u16_narrowing()),
            bytes_size,
            start_display_col_index: vp_col(display_col),
        });

        byte_offset += bytes_size.as_usize();
        display_col += (display_width).as_u16_narrowing();
    }

    segments
}

/// Builds segments for [`ASCII`]-only strings (optimized path).
///
/// Since [`ASCII`] characters are always 1 byte and 1 display column wide, we can build
/// segments more efficiently without Unicode analysis.
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
fn build_ascii_segments(input: &str) -> SegmentArray {
    let mut segments = SegmentArray::with_capacity(input.len());

    for (i, _) in input.char_indices() {
        segments.push(Seg {
            start_byte_index: byte_index(i),
            end_byte_index: byte_index(i + 1),
            display_width: vp_width(1),
            seg_index: seg_index((i).as_u16_narrowing()),
            bytes_size: vp_len(1),
            start_display_col_index: vp_col((i).as_u16_narrowing()),
        });
    }

    segments
}

/// Calculates total display width from segments.
///
/// This sums up the display width of all segments to get the total width of the string
/// when rendered in a terminal.
#[must_use]
pub fn calculate_display_width(segments: &SegmentArray) -> VPWidth {
    match segments.last() {
        Some(seg) => {
            let start_col: VPCol = seg.start_display_col_index;
            let seg_width: VPWidth = seg.display_width;
            let end_col = *start_col + *seg_width;
            vp_width(end_col)
        }
        None => vp_width(0),
    }
}

/// Builds [`DocSeg`] grapheme cluster segments for any string slice using 64-bit Canvas
/// coordinates.
#[must_use]
pub fn build_doc_segments_for_str(input: &str) -> Vec<DocSeg> {
    if input.is_ascii() {
        return build_ascii_doc_segments(input);
    }

    let mut segments = Vec::new();
    let mut byte_offset = 0usize;
    let mut display_col = 0usize;

    for (seg_idx, grapheme) in input.graphemes(true).enumerate() {
        let bytes_size = byte_len(grapheme.len());
        let display_width = UnicodeWidthStr::width(grapheme);

        segments.push(DocSeg {
            start_byte_index: byte_index(byte_offset),
            end_byte_index: byte_index(byte_offset + bytes_size.as_usize()),
            display_width: c_width(display_width),
            seg_index: c_index(seg_idx),
            bytes_size,
            start_display_col_index: c_col(display_col),
        });

        byte_offset += bytes_size.as_usize();
        display_col += display_width;
    }

    segments
}

fn build_ascii_doc_segments(input: &str) -> Vec<DocSeg> {
    let mut segments = Vec::with_capacity(input.len());

    for (i, _) in input.char_indices() {
        segments.push(DocSeg {
            start_byte_index: byte_index(i),
            end_byte_index: byte_index(i + 1),
            display_width: c_width(1usize),
            seg_index: c_index(i),
            bytes_size: byte_len(1usize),
            start_display_col_index: c_col(i),
        });
    }

    segments
}

/// Calculates total [`Canvas`] display width from [`DocSeg`] segments.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
#[must_use]
pub fn calculate_doc_display_width(segments: &[DocSeg]) -> CWidth {
    match segments.last() {
        Some(seg) => seg.start_display_col_index - c_col(0usize) + seg.display_width,
        None => c_width(0usize),
    }
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
        assert_eq2!(calculate_display_width(&segments), vp_width(5));

        // Check first segment 'H'.
        let seg = &segments[0];
        assert_eq2!(seg.start_byte_index, byte_index(0));
        assert_eq2!(seg.end_byte_index, byte_index(1));
        assert_eq2!(seg.display_width, vp_width(1));
        assert_eq2!(seg.start_display_col_index, vp_col(0));
    }

    #[test]
    fn test_build_segments_emoji() {
        let input = "H😀!";
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 3);
        assert_eq2!(calculate_display_width(&segments), vp_width(4)); // H(1) + 😀(2) + !(1)

        // Check emoji segment.
        let emoji_seg = &segments[1];
        assert_eq2!(emoji_seg.start_byte_index, byte_index(1));
        assert_eq2!(emoji_seg.end_byte_index, byte_index(5)); // 4 bytes
        assert_eq2!(emoji_seg.display_width, vp_width(2));
        assert_eq2!(emoji_seg.start_display_col_index, vp_col(1));
    }

    #[test]
    fn test_build_segments_combining_chars() {
        // Using composed form to avoid clippy warning.
        let input = "café"; // é is composed
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 4);
        assert_eq2!(calculate_display_width(&segments), vp_width(4));
    }

    #[test]
    fn test_build_segments_jumbo_emoji() {
        let input = "🙏🏽"; // Folded hands with skin tone
        let segments = build_segments_for_str(input);

        assert_eq2!(segments.len(), 1); // Single grapheme cluster
        assert_eq2!(calculate_display_width(&segments), vp_width(2));

        let seg = &segments[0];
        assert_eq2!(seg.bytes_size.as_usize(), 8); // 4 bytes for 🙏 + 4 bytes for 🏽
        assert_eq2!(seg.display_width, vp_width(2));
    }

    #[test]
    fn test_calculate_display_width_empty() {
        let segments = SegmentArray::new();
        assert_eq2!(calculate_display_width(&segments), vp_width(0));
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use std::hint::black_box;
    use test::Bencher;

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
