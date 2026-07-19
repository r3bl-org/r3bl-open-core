// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Flat1DSimd, Flat1DSimdMut};
use crate::{
    c_row, ArrayBoundsCheck, ArrayOverflowResult, CRow, RangeBoundsExt, RangeExclusive,
    RangeExt, RangeValidityStatus, VPLength,
};
use std::cmp::{max, min};

impl<T> Flat1DSimd<'_, T> {
    /// Returns the raw contiguous slice for aggressive loop vectorization.
    #[must_use]
    pub fn as_raw_slice(&self) -> &[T] { self.data }
}

impl<T: Copy> Flat1DSimdMut<'_, T> {
    /// Optimized zero-allocation scrolling.
    ///
    /// Copies elements from one region of the slice to another, using
    /// [`slice::copy_within`].
    ///
    /// See [`Flat1DSimd`] for more details.
    ///
    /// [`slice::copy_within`]: slice::copy_within
    pub fn copy_within_rows(
        &mut self,
        src_row_range: RangeExclusive<CRow>,
        dest_row_start_idx: CRow,
    ) {
        let dest_row_range = {
            let num_rows = src_row_range.end - src_row_range.start;
            let dest_row_end_idx = dest_row_start_idx + num_rows;
            dest_row_start_idx..dest_row_end_idx
        };

        let src_row_range_is_invalid = src_row_range
            .check_range_is_valid_for_length(self.height)
            != RangeValidityStatus::Valid;
        let dest_row_range_is_invalid = dest_row_range
            .check_range_is_valid_for_length(self.height)
            != RangeValidityStatus::Valid;
        if src_row_range_is_invalid || dest_row_range_is_invalid {
            return;
        }

        let width = self.width.as_usize();

        let src_range = {
            let src_row_range = src_row_range.as_usize_range();
            let src_offset_start_idx = src_row_range.start * width;
            let src_offset_end_idx = src_row_range.end * width;
            src_offset_start_idx..src_offset_end_idx
        };

        let dest_row_range = dest_row_range.as_usize_range();
        let dest_offset_start_idx = dest_row_range.start * width;

        self.data.copy_within(src_range, dest_offset_start_idx);
    }

    /// Shifts rows up within the specified range by a given amount, filling vacated rows
    /// with `empty_char`.
    ///
    /// # Example
    ///
    /// Calling `shift_rows_up(c_row(1)..c_row(4), vp_len(1), empty_char)`:
    ///
    /// ```text
    /// Before:                           After shift_rows_up
    ///        ┌──────────┐                     ┌──────────┐
    ///  row 0 │  Row 0   │               row 0 │  Row 0   │
    ///        ├──────────┤                     ├──────────┤
    ///  row 1 │  Row 1   │ ◄─ start = 1  row 1 │  Row 2   │ shifted up
    ///        ├──────────┤                     ├──────────┤
    ///  row 2 │  Row 2   │               row 2 │  Row 3   │ shifted up
    ///        ├──────────┤                     ├──────────┤
    ///  row 3 │  Row 3   │               row 3 │  empty   │ vacated & filled w/ empty_char
    ///        ├──────────┤                     ├──────────┤
    ///  row 4 │  Row 4   │ ◄─ end = 4    row 4 │  Row 4   │
    ///        └──────────┘                     └──────────┘
    /// ```
    pub fn shift_rows_up(
        &mut self,
        row_range: RangeExclusive<CRow>,
        amount: VPLength,
        empty_char: T,
    ) {
        let clamped_range = row_range.clamp_range_to(self.height);

        if amount.is_empty() || clamped_range.is_empty() {
            return;
        }

        let start = clamped_range.start.as_usize();
        let end = clamped_range.end.as_usize();

        let row_range_to_copy = c_row(start + amount.as_usize())..c_row(end);
        let start_idx = c_row(start);
        self.copy_within_rows(row_range_to_copy, start_idx);

        let fill_start = max(start, end.saturating_sub(amount.as_usize()));
        self.fill_rows(c_row(fill_start)..c_row(end), empty_char);
    }

    /// Shifts rows down within the specified range by a given amount, filling vacated
    /// rows with `empty_char`.
    ///
    /// # Example
    ///
    /// Calling `shift_rows_down(c_row(1)..c_row(4), vp_len(1), empty_char)`:
    ///
    /// ```text
    /// Before:                           After shift_rows_down
    ///        ┌──────────┐                     ┌──────────┐
    ///  row 0 │  Row 0   │               row 0 │  Row 0   │
    ///        ├──────────┤                     ├──────────┤
    ///  row 1 │  Row 1   │ ◄─ start = 1  row 1 │  empty   │ vacated & filled w/ empty_char
    ///        ├──────────┤                     ├──────────┤
    ///  row 2 │  Row 2   │               row 2 │  Row 1   │ shifted down
    ///        ├──────────┤                     ├──────────┤
    ///  row 3 │  Row 3   │               row 3 │  Row 2   │ shifted down
    ///        ├──────────┤                     ├──────────┤
    ///  row 4 │  Row 4   │ ◄─ end = 4    row 4 │  Row 4   │
    ///        └──────────┘                     └──────────┘
    /// ```
    pub fn shift_rows_down(
        &mut self,
        row_range: RangeExclusive<CRow>,
        amount: VPLength,
        empty_char: T,
    ) {
        let clamped_range = row_range.clamp_range_to(self.height);

        if amount.is_empty() || clamped_range.is_empty() {
            return;
        }

        let start = clamped_range.start.as_usize();
        let end = clamped_range.end.as_usize();

        let row_range_to_copy =
            c_row(start)..c_row(end.saturating_sub(amount.as_usize()));
        let start_idx = c_row(start + amount.as_usize());
        self.copy_within_rows(row_range_to_copy, start_idx);

        let fill_end = min(start + amount.as_usize(), end);
        self.fill_rows(c_row(start)..c_row(fill_end), empty_char);
    }
}

impl<T: Clone> Flat1DSimdMut<'_, T> {
    /// Optimized clearing using [`slice::fill`].
    ///
    /// Fills the specified row range with the provided value.
    ///
    /// See [`Flat1DSimd`] for more details.
    ///
    /// [`slice::fill`]: slice::fill
    pub fn fill_rows(&mut self, row_range: RangeExclusive<CRow>, val: T) {
        let is_invalid = row_range.check_range_is_valid_for_length(self.height)
            != RangeValidityStatus::Valid;
        if is_invalid {
            return;
        }

        let width = self.width.as_usize();

        let target_range = {
            let row_range_usize = row_range.as_usize_range();
            let row_offset_start_idx = row_range_usize.start * width;
            let row_offset_end_idx = row_range_usize.end * width;
            row_offset_start_idx..row_offset_end_idx
        };

        self.data[target_range].fill(val);
    }

    /// Fills the entire grid with the provided value.
    ///
    /// See [`Flat1DSimd`] for more details.
    pub fn fill_all(&mut self, val: T) { self.data.fill(val); }

    /// Optimized swapping of two rows using slice splitting and
    /// [`slice::swap_with_slice`].
    ///
    /// # Algorithm
    ///
    /// This safely circumvents Rust's borrowing rules (which prevent holding two mutable
    /// references to the same array) to swap two memory chunks simultaneously:
    ///
    /// 1. Finds the true physical 1D start index for both rows.
    /// 2. Determines which row comes first in memory (`min`) and which comes second
    ///    (`max`).
    /// 3. Uses [`slice::split_at_mut`] to cleanly divide the single array into two
    ///    non-overlapping mutable slices exactly at the `max` boundary.
    ///    - The `left` slice gets everything before the second row (including the entire
    ///      first row).
    ///    - The `right` slice starts exactly at the second row.
    /// 4. Leverages [`slice::swap_with_slice`] to perform a highly optimized bulk swap of
    ///    the first row's bytes (from the `left` slice) with the second row's bytes (from
    ///    the `right` slice).
    ///
    /// # Example
    ///
    /// What happens if we call `swap_rows(2, 0)` on a grid with a `width` of 10?
    ///
    /// ```text
    /// ┌─────────┬─────────┬─────────┬─────────┐
    /// │  Row 0  │  Row 1  │  Row 2  │  Row 3  │
    /// │ [0..9]  │ [10..19]│ [20..29]│ [30..39]│
    /// └─────────┴─────────┴─────────┴─────────┘
    ///                     ▲
    ///                     │ split_at_mut(20)
    ///
    /// ┌───────────────────┐ ┌───────────────────┐
    /// │       Left        │ │       Right       │
    /// ├─────────┬─────────┤ ├─────────┬─────────┤
    /// │  Row 0  │  Row 1  │ │  Row 2  │  Row 3  │
    /// │ [0..9]  │ [10..19]│ │ [20..29]│ [30..39]│
    /// └─────────┴─────────┘ └─────────┴─────────┘
    /// ```
    ///
    /// 1. `row_1_start_idx` = `2 * 10 = 20`
    /// 2. `row_2_start_idx` = `0 * 10 = 0`
    /// 3. We sort the indices:
    ///    - `first_row_start_idx` = `min(20, 0) = 0`
    ///    - `second_row_start_idx` = `max(20, 0) = 20`
    /// 4. We call `split_at_mut(20)` as shown in the diagram:
    ///    - `left` becomes the slice from index `0` to `19`. (This safely contains all of
    ///      Row 0).
    ///    - `right` becomes the slice from index `20` to the end. (`right[0]` is the
    ///      start of Row 2).
    /// 5. Finally, we swap `left[0..10]` with `right[0..10]`.
    ///
    /// Because we dynamically sorted the inputs and cut at the larger index, it works
    /// perfectly regardless of the order the row parameters were provided in.
    ///
    /// [`slice::split_at_mut`]: slice::split_at_mut
    /// [`slice::swap_with_slice`]: slice::swap_with_slice
    pub fn swap_rows(&mut self, row_1: CRow, row_2: CRow) {
        // If the two row indices are the same, there's nothing to swap, so we can return
        // early.
        if row_1 == row_2 {
            return;
        }

        // Check if either row index is out of bounds. If so, return early to avoid
        // panicking.
        let is_invalid_1 = row_1.overflows(self.height) != ArrayOverflowResult::Within;
        let is_invalid_2 = row_2.overflows(self.height) != ArrayOverflowResult::Within;
        if is_invalid_1 || is_invalid_2 {
            return;
        }

        // Determine the starting indices of the two rows in the underlying 1D slice.
        let width = self.width.as_usize();
        let row_1_start_idx = row_1.as_usize() * width;
        let row_2_start_idx = row_2.as_usize() * width;

        // We must sort the indices to safely use `split_at_mut`. `split_at_mut` cuts the
        // array into two pieces. By splitting at the higher index
        // (`second_row_start_idx`), we guarantee that the first row is completely
        // contained in the `left` slice, and the second row is at the very beginning of
        // the `right` slice. This ensures the two rows do not overlap, satisfying Rust's
        // strict mutability rules.
        let first_row_start_idx = row_1_start_idx.min(row_2_start_idx);
        let second_row_start_idx = row_1_start_idx.max(row_2_start_idx);

        // Cut the array exactly at the start of the second row.
        // `right[0]` is now the beginning of the second row.
        let (left, right) = self.data.split_at_mut(second_row_start_idx);

        // Swap the corresponding chunk in the left partition with the chunk in the right
        // partition.
        left[first_row_start_idx..first_row_start_idx + width]
            .swap_with_slice(&mut right[0..width]);
    }
}

#[cfg(test)]
mod tests {

    use crate::{CHeight, CSize, CWidth, Flat2DArray, c_row};

    #[test]
    fn test_copy_within_rows() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        // Row 0: [1, 2]
        grid[c_row(0)][0] = 1;
        grid[c_row(0)][1] = 2;
        // Row 1: [3, 4]
        grid[c_row(1)][0] = 3;
        grid[c_row(1)][1] = 4;

        // Copy Row 0 to Row 2
        grid.as_simd_mut()
            .copy_within_rows(c_row(0)..c_row(1), c_row(2));

        // Row 2 should now be [1, 2]
        assert_eq!(grid[c_row(2)][0], 1);
        assert_eq!(grid[c_row(2)][1], 2);

        // Row 1 should still be [3, 4]
        assert_eq!(grid[c_row(1)][0], 3);
    }

    #[test]
    fn test_fill_rows() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );

        // Fill Row 1
        grid.as_simd_mut().fill_rows(c_row(1)..c_row(2), 99);

        assert_eq!(grid[c_row(0)][0], 0);
        assert_eq!(grid[c_row(1)][0], 99);
        assert_eq!(grid[c_row(1)][1], 99);
        assert_eq!(grid[c_row(2)][0], 0);
    }

    #[test]
    fn test_zero_dimensions() {
        let grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(0usize), CHeight::from(0usize))),
            0,
        );
        assert_eq!(grid.as_simd().as_raw_slice().len(), 0);
    }

    #[test]
    fn test_copy_within_rows_overlapping() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        // Row 0: [1, 2], Row 1: [3, 4], Row 2: [5, 6]
        grid[c_row(0)][0] = 1;
        grid[c_row(0)][1] = 2;
        grid[c_row(1)][0] = 3;
        grid[c_row(1)][1] = 4;
        grid[c_row(2)][0] = 5;
        grid[c_row(2)][1] = 6;

        // Copy Row 0..2 (Rows 0 and 1) to Row 1..3 (Rows 1 and 2)
        grid.as_simd_mut()
            .copy_within_rows(c_row(0)..c_row(2), c_row(1));

        assert_eq!(grid[c_row(0)][0], 1);
        assert_eq!(grid[c_row(1)][0], 1);
        assert_eq!(grid[c_row(2)][0], 3);
    }

    #[test]
    fn test_copy_within_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        grid[c_row(0)][0] = 1;

        // Source out of bounds
        grid.as_simd_mut()
            .copy_within_rows(c_row(0)..c_row(4), c_row(1)); // Should not panic

        // Destination out of bounds
        grid.as_simd_mut()
            .copy_within_rows(c_row(0)..c_row(1), c_row(3)); // Should not panic

        // Inverse range (start > end)
        grid.as_simd_mut()
            .copy_within_rows(c_row(2)..c_row(1), c_row(0)); // Should not panic

        // Grid should remain unmodified
        assert_eq!(grid[c_row(0)][0], 1);
        assert_eq!(grid[c_row(1)][0], 0);
    }

    #[test]
    fn test_fill_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );

        // Out of bounds end
        grid.as_simd_mut().fill_rows(c_row(1)..c_row(5), 99); // Should not panic

        // Inverse range
        grid.as_simd_mut().fill_rows(c_row(2)..c_row(1), 99); // Should not panic

        assert_eq!(grid[c_row(1)][0], 0); // Remained 0
    }

    #[test]
    fn test_fill_all() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(2usize))),
            0,
        );
        grid.as_simd_mut().fill_all(42);
        assert_eq!(grid[c_row(0)][0], 42);
        assert_eq!(grid[c_row(1)][1], 42);
    }

    #[test]
    fn test_swap_rows() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        // Row 0: [1, 2]
        grid[c_row(0)][0] = 1;
        grid[c_row(0)][1] = 2;
        // Row 1: [3, 4]
        grid[c_row(1)][0] = 3;
        grid[c_row(1)][1] = 4;
        // Row 2: [5, 6]
        grid[c_row(2)][0] = 5;
        grid[c_row(2)][1] = 6;

        // Swap c_row 0 and 2
        grid.as_simd_mut().swap_rows(c_row(0), c_row(2));
        assert_eq!(grid[c_row(0)][0], 5);
        assert_eq!(grid[c_row(0)][1], 6);
        assert_eq!(grid[c_row(2)][0], 1);
        assert_eq!(grid[c_row(2)][1], 2);
        // Row 1 unchanged
        assert_eq!(grid[c_row(1)][0], 3);

        // Swap c_row 2 and 1 (reverse order params)
        grid.as_simd_mut().swap_rows(c_row(2), c_row(1));
        assert_eq!(grid[c_row(1)][0], 1); // formerly c_row 2
        assert_eq!(grid[c_row(2)][0], 3); // formerly c_row 1
    }

    #[test]
    fn test_swap_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        grid[c_row(0)][0] = 1;

        // Should not panic, just return early
        grid.as_simd_mut().swap_rows(c_row(0), c_row(5));
        grid.as_simd_mut().swap_rows(c_row(5), c_row(0));

        // Array unchanged
        assert_eq!(grid[c_row(0)][0], 1);
    }

    #[test]
    fn test_swap_rows_same_row() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(3usize))),
            0,
        );
        grid[c_row(0)][0] = 1;
        grid[c_row(0)][1] = 2;

        // Should not panic, just return early
        grid.as_simd_mut().swap_rows(c_row(0), c_row(0));

        // Array unchanged
        assert_eq!(grid[c_row(0)][0], 1);
        assert_eq!(grid[c_row(0)][1], 2);
    }
}
