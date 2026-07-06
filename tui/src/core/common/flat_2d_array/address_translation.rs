// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, ColWidth, Pos, RowHeight};
use std::ops::Range;

/// Helper to calculate the 2D coordinates from a 1D index.
///
/// This is the exact inverse of [`pos_to_index`]. It is primarily used
/// during [SIMD] fast-path diffing, where the algorithm iterates linearly over the
/// 1D slice, finds a difference at a specific 1D `index`, and needs to know the
/// corresponding `(row, col)` coordinate to issue a terminal cursor movement
/// command.
///
/// # 1D to 2D Mapping
///
/// This is the exact inverse of the above. It is primarily used
/// during SIMD fast-path diffing, where the algorithm iterates linearly over the
/// 1D slice, finds a difference at a specific 1D `index`, and needs to know the
/// corresponding `(row, col)` coordinate to issue a terminal cursor movement
/// command.
///
/// ```text
/// 1D Grid:
///
///   row 0                   row 1                   row 2
///   col 0   col 1   col 2   col 0   col 1   col 2   col 0   col 1   col 2
/// ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
/// │ idx 0 │ idx 1 │ idx 2 │ idx 3 │ idx 4 │ idx 5 │ idx 6 │ idx 7 │ idx 8 │
/// └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘
///                                             ↑
///                                       index_to_pos(5)
/// 2D Grid (equivalent):
///
///          col 0   col 1   col 2
///        ┌───────┬───────┬───────┐
///  row 0 │ idx 0 │ idx 1 │ idx 2 │
///        ├───────┼───────┼───────┤
///  row 1 │ idx 3 │ idx 4 │ idx 5 │  ← index_to_pos(5)
///        ├───────┼───────┼───────┤    = Pos { row: 1, col: 2 }
///  row 2 │ idx 6 │ idx 7 │ idx 8 │
///        └───────┴───────┴───────┘
/// ```
///
/// Example: `index 5` with `width 3`
/// - `row = index / width = 5 / 3 = 1`
/// - `col = index % width = 5 % 3 = 2`
///
/// [SIMD]: https://en.wikipedia.org/wiki/SIMD
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn index_to_pos(index: usize, width: ColWidth, height: RowHeight) -> Option<Pos> {
    let max_len = width.as_usize() * height.as_usize();
    if index >= max_len {
        return None;
    }

    // We must use `usize` for this math because the 1D `index` mathematically
    // represents `width * height`, which can easily exceed `u16::MAX` (e.g. 300 * 300
    // = 90,000). Casting `index` to `u16` before division would overflow.
    let row = index / width.as_usize();
    let col = index % width.as_usize();

    // The division guarantees `row < height` and `col < width`. Since `width`
    // and `height` are `u16`, it's completely safe to cast them back to `u16`.
    Some(Pos {
        row_index: (row as u16).into(),
        col_index: (col as u16).into(),
    })
}

/// Helper to calculate the 1D index from 2D coordinates.
///
/// # 2D to 1D Mapping
///
/// The grid is stored row-by-row in a flat 1D slice.
///
/// To find the element at `(row, col)`, we skip `row` full rows of size `width`,
/// and then step forward by `col`.
///
/// ```text
/// 2D Grid:
///           col 0   col 1   col 2
///        ┌───────┬───────┬───────┐
///  row 0 │ idx 0 │ idx 1 │ idx 2 │
///        ├───────┼───────┼───────┤
///  row 1 │ idx 3 │ idx 4 │ idx 5 │
///        ├───────┼───────┼───────┤
///  row 2 │ idx 6 │ idx 7 │ idx 8 │
///        └───────┴───────┴───────┘
///
/// 1D Grid (equivalent):
///
///   row 0                   row 1                   row 2
///   col 0   col 1   col 2   col 0   col 1   col 2   col 0   col 1   col 2
/// ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
/// │ idx 0 │ idx 1 │ idx 2 │ idx 3 │ idx 4 │ idx 5 │ idx 6 │ idx 7 │ idx 8 │
/// └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘
/// ```
///
/// Example: `(row 1, col 2)`
/// - `row_offset = row_index * width      = 1 * 3 = 3`
/// - `1d_index   = row_offset + col_index = 3 + 2 = 5`
#[inline]
#[must_use]
pub fn pos_to_index(
    arg_pos: impl Into<Pos>,
    width: ColWidth,
    height: RowHeight,
) -> Option<usize> {
    let Pos {
        row_index,
        col_index,
    } = arg_pos.into();

    let row_idx_is_within_bounds =
        row_index.overflows(height) == ArrayOverflowResult::Within;
    let col_idx_is_within_bounds =
        col_index.overflows(width) == ArrayOverflowResult::Within;

    if row_idx_is_within_bounds && col_idx_is_within_bounds {
        let width = width.as_usize();
        let row = row_index.as_usize();
        let col = col_index.as_usize();
        let row = row * width;
        let final_index = row + col;
        Some(final_index)
    } else {
        None
    }
}

/// Helper to get the 1D slice range for a specific row index.
///
/// # 1D Slice Range Mapping
///
/// The returned range `start..end` corresponds exactly to the flat 1D slice
/// indices for that specific row. `row_idx` represents the zero-indexed
/// row number (e.g., `0` for the first row).
///
/// ```text
///          col 0   col 1   col 2
///        ┌───────┬───────┬───────┐
///  row 0 │ idx 0 │ idx 1 │ idx 2 │  ← row_idx_to_bounds(0) = 0..3
///        ├───────┼───────┼───────┤
///  row 1 │ idx 3 │ idx 4 │ idx 5 │  ← row_idx_to_bounds(1) = 3..6
///        ├───────┼───────┼───────┤
///  row 2 │ idx 6 │ idx 7 │ idx 8 │  ← row_idx_to_bounds(2) = 6..9
///        └───────┴───────┴───────┘
/// ```
///
/// # Design: Why is `row_idx: usize`?
///
/// This function accepts a raw [`usize`] instead of a strongly-typed [`RowIndex`]
/// because it serves as the foundational engine for the [`Index<usize>`] and
/// [`IndexMut<usize>`] implementations. Those traits must accept [`usize`] to
/// enable standard native `buffer[row][col]` ergonomics.
///
/// # Panics
///
/// Panics if the row index is out of bounds.
///
/// [`Index<usize>`]: std::ops::Index
/// [`IndexMut<usize>`]: std::ops::IndexMut
/// [`RowIndex`]: crate::RowIndex
#[inline]
#[must_use]
pub fn row_idx_to_bounds(
    row_idx: usize,
    width: ColWidth,
    height: RowHeight,
) -> Range<usize> {
    let is_overflowed = row_idx /* 0-index */ >= height.as_usize() /* 1-index */;
    #[allow(clippy::manual_assert)]
    if is_overflowed {
        panic!(
            "row index out of bounds: the height is {} but the index is {}",
            height.as_usize(),
            row_idx
        );
    }
    let width_usize = width.as_usize();
    let row_offset_start_idx = row_idx * width_usize;
    let row_offset_end_idx = row_offset_start_idx + width_usize;

    row_offset_start_idx..row_offset_end_idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_address_translation_pure_math() {
        // We can test massive grids without allocating any memory!
        let w = width(u16::MAX);
        let h = height(u16::MAX);

        // Max possible valid index before overflow (though size is w * h)
        let max_pos = row(u16::MAX - 1) + col(u16::MAX - 1);
        let max_index = pos_to_index(max_pos, w, h).unwrap();

        // 1. pos_to_index -> index_to_pos
        assert_eq!(index_to_pos(max_index, w, h), Some(max_pos));

        // 2. Out of bounds checking (without allocating!)
        assert_eq!(index_to_pos((w.as_usize() * h.as_usize()) + 1, w, h), None);
        assert_eq!(pos_to_index(row(u16::MAX) + col(0), w, h), None);
        assert_eq!(pos_to_index(row(0) + col(u16::MAX), w, h), None);

        // 3. row bounds checking
        let bounds = row_idx_to_bounds(0, w, h);
        assert_eq!(bounds, 0..w.as_usize());

        let bounds_max = row_idx_to_bounds((u16::MAX - 1) as usize, w, h);
        let start = (u16::MAX - 1) as usize * u16::MAX as usize;
        let end = start + u16::MAX as usize;
        assert_eq!(bounds_max, start..end);
    }
}
