// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Implementation of bulk operations for [`OfsBuf`].
//!
//! This module provides methods for applying multiple changes to the buffer
//! in a single operation, which can be more efficient than individual changes.

use super::{CanvasStorage, OfsBuf, PixelChar, PixelCharDiffChunks};
use crate::{Flat2DArray, List, NarrowingCastToU16, VPPos, vp_col, vp_row};

impl<S: CanvasStorage> OfsBuf<S> {
    /// Apply multiple character changes at once.
    /// Returns the number of successful changes applied.
    pub fn apply_changes(&mut self, changes: Vec<(VPPos, PixelChar)>) -> usize {
        let mut applied_count = 0;

        for (pos, char) in changes {
            if self.set_char(pos, char).is_ok() {
                applied_count += 1;
            }
        }

        applied_count
    }
}

impl OfsBuf<Flat2DArray<PixelChar>> {
    /// Compares this offscreen buffer with another one and returns a list of differences.
    ///
    /// 1. Only processes the differences if the dimensions of both buffers are identical.
    /// 2. Iterates over the `storage` using `as_simd()` for high-performance diffing.
    /// 3. Returns [`None`] if dimensions mismatch, or a [`PixelCharDiffChunks`]
    ///    containing only the modified characters (from `other`).
    ///
    /// See the [Rule of Thumb for 1D vs 2D Memory Iteration] and the [Deep Dive: The
    /// Magic of SIMD Diffing] for a detailed breakdown of how this linear traversal
    /// eliminates CPU pipeline stalls and leverages multi-stream hardware prefetching.
    ///
    /// [`.chunks_exact()`]: slice::chunks_exact
    /// [`Flat1DSimd`]: crate::core::Flat1DSimd
    /// [`Flat2DArray`]: crate::core::Flat2DArray
    /// [Deep Dive: The Magic of SIMD Diffing]:
    ///     crate::core::Flat1DSimd#deep-dive-the-magic-of-simd-diffing
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::core::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[must_use]
    pub fn diff(&self, other: &Self) -> Option<PixelCharDiffChunks> {
        if self.width != other.width || self.height != other.height {
            return None;
        }

        let mut acc = List::default();
        let self_simd = self.as_simd();
        let other_simd = other.as_simd();
        let width = self.width.as_usize();

        let self_rows_iter = self_simd.as_raw_slice().chunks_exact(width);
        debug_assert!(
            self_rows_iter.remainder().is_empty(),
            "The data length should be a multiple of the number of columns."
        );

        let other_rows_iter = other_simd.as_raw_slice().chunks_exact(width);
        debug_assert!(
            other_rows_iter.remainder().is_empty(),
            "The data length should be a multiple of the number of columns."
        );

        let zipped_rows_iter = self_rows_iter.zip(other_rows_iter).enumerate();
        for (row_idx, (self_row_chunk, other_row_chunk)) in zipped_rows_iter {
            if self_row_chunk != other_row_chunk {
                let cols_iter = self_row_chunk
                    .iter()
                    .zip(other_row_chunk.iter())
                    .enumerate();
                for (col_idx, (self_pixel_char, other_pixel_char)) in cols_iter {
                    if self_pixel_char != other_pixel_char {
                        let pos = VPPos {
                            row_index: vp_row((row_idx).as_u16_narrowing()),
                            col_index: vp_col((col_idx).as_u16_narrowing()),
                        };
                        acc.push((pos, *other_pixel_char));
                    }
                }
            }
        }

        Some(PixelCharDiffChunks::from(acc))
    }
}

#[cfg(test)]
mod tests_bulk_ops {
    use super::*;
    use crate::{NarrowingCastToU16, OfsBufVT100, TuiStyle, vp_col, vp_height, vp_row,
                vp_width};

    fn create_test_buffer() -> OfsBufVT100 {
        let size = vp_width(4) + vp_height(4);
        OfsBufVT100::new_empty(size)
    }

    fn create_test_char(ch: char) -> PixelChar {
        PixelChar::PlainText {
            display_char: ch,
            style: TuiStyle::default(),
        }
    }

    #[test]
    fn test_apply_changes_batch() {
        let mut buffer = create_test_buffer();

        let changes = vec![
            (vp_row(0) + vp_col(0), create_test_char('A')),
            (vp_row(0) + vp_col(1), create_test_char('B')),
            (vp_row(1) + vp_col(0), create_test_char('C')),
            (vp_row(1) + vp_col(1), create_test_char('D')),
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 4); // All changes should be applied successfully

        // Verify all changes were applied.
        assert_eq!(
            buffer
                .get_char(vp_row(0) + vp_col(0))
                .expect("conversion error"),
            create_test_char('A')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(0) + vp_col(1))
                .expect("conversion error"),
            create_test_char('B')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(1) + vp_col(0))
                .expect("conversion error"),
            create_test_char('C')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(1) + vp_col(1))
                .expect("conversion error"),
            create_test_char('D')
        );
    }

    #[test]
    fn test_apply_changes_with_invalid_positions() {
        let mut buffer = create_test_buffer();

        let changes = vec![
            (vp_row(0) + vp_col(0), create_test_char('V')), // Valid
            (vp_row(10) + vp_col(0), create_test_char('I')), // Invalid row
            (vp_row(0) + vp_col(10), create_test_char('I')), // Invalid column
            (vp_row(2) + vp_col(2), create_test_char('V')), // Valid
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 2); // Only 2 valid changes should be applied

        // Verify valid changes were applied.
        assert_eq!(
            buffer
                .get_char(vp_row(0) + vp_col(0))
                .expect("conversion error"),
            create_test_char('V')
        );
        assert_eq!(
            buffer
                .get_char(vp_row(2) + vp_col(2))
                .expect("conversion error"),
            create_test_char('V')
        );
    }

    #[test]
    fn test_apply_changes_empty_batch() {
        let mut buffer = create_test_buffer();

        let changes = vec![];
        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 0);
    }

    #[test]
    fn test_apply_changes_large_batch() {
        let mut buffer = create_test_buffer();

        // Create a large batch of changes.
        let mut changes = vec![];
        for r in 0..4 {
            for c in 0..4 {
                changes.push((
                    vp_row(r.as_u16_narrowing()) + vp_col(c.as_u16_narrowing()),
                    create_test_char('*'),
                ));
            }
        }

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 16); // All 16 positions in 4x4 buffer

        // Verify all positions were changed.
        for r in 0..4 {
            for c in 0..4 {
                assert_eq!(
                    buffer
                        .get_char(
                            vp_row(r.as_u16_narrowing()) + vp_col(c.as_u16_narrowing())
                        )
                        .expect("conversion error"),
                    create_test_char('*')
                );
            }
        }
    }

    #[test]
    fn test_apply_changes_overlapping() {
        let mut buffer = create_test_buffer();

        // Apply changes to same position multiple times.
        let changes = vec![
            (vp_row(1) + vp_col(1), create_test_char('1')),
            (vp_row(1) + vp_col(1), create_test_char('2')),
            (vp_row(1) + vp_col(1), create_test_char('3')),
        ];

        let applied_count = buffer.apply_changes(changes);
        assert_eq!(applied_count, 3); // All changes should be applied

        // The last change should win.
        assert_eq!(
            buffer
                .get_char(vp_row(1) + vp_col(1))
                .expect("conversion error"),
            create_test_char('3')
        );
    }

    #[test]
    fn test_ofs_buf_diff_exercises_viewport_narrowing() {
        let buf1 = create_test_buffer();
        let mut buf2 = create_test_buffer();

        let diff_pos = vp_row(2) + vp_col(3);
        let diff_char = create_test_char('X');
        buf2.apply_changes(vec![(diff_pos, diff_char)]);

        let diff_chunks = buf1.diff(&buf2).expect("diff failed");

        assert_eq!(diff_chunks.len(), 1);
        assert_eq!(diff_chunks[0].0, diff_pos);
        assert_eq!(diff_chunks[0].1, diff_char);
    }
}
