// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Flat1DSimd, Flat1DSimdMut, address_translation};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, Pos, RangeBoundsExt, RangeExt,
            RangeValidityStatus, RowIndex};
use std::ops::Range;

impl<T> Flat1DSimd<'_, T> {
    /// Returns the raw contiguous slice for aggressive loop vectorization.
    #[must_use]
    pub fn as_raw_slice(&self) -> &[T] { self.data }

    /// Helper to calculate the 2D coordinates from a 1D index.
    ///
    /// This is the exact inverse of [`address_translation::pos_to_index`]. It is
    /// primarily used during [SIMD] fast-path diffing, where the algorithm iterates
    /// linearly over the 1D slice, finds a difference at a specific 1D `index`, and needs
    /// to know the corresponding `(row, col)` coordinate to issue a terminal cursor
    /// movement command.
    ///
    /// Delegates to [`address_translation::index_to_pos`].
    ///
    /// [`address_translation::index_to_pos`]:
    ///     crate::core::common::flat_2d_array::address_translation::index_to_pos
    /// [`address_translation::pos_to_index`]:
    ///     crate::core::common::flat_2d_array::address_translation::pos_to_index
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[inline]
    #[must_use]
    pub fn get_pos_from_index(&self, index: usize) -> Option<Pos> {
        address_translation::index_to_pos(index, self.width, self.height)
    }
}

impl<T> Flat1DSimdMut<'_, T> {
    /// Returns the raw contiguous slice for aggressive loop vectorization.
    pub fn as_raw_mut_slice(&mut self) -> &mut [T] { self.data }
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
        src_row_range: Range<RowIndex>,
        dest_row_start_idx: RowIndex,
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
}

impl<T: Clone> Flat1DSimdMut<'_, T> {
    /// Optimized clearing using [`slice::fill`].
    ///
    /// Fills the specified row range with the provided value.
    ///
    /// See [`Flat1DSimd`] for more details.
    ///
    /// [`slice::fill`]: slice::fill
    pub fn fill_rows(&mut self, row_range: Range<RowIndex>, val: T) {
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
    pub fn swap_rows(&mut self, row_1: RowIndex, row_2: RowIndex) {
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

    use crate::{Flat2DArray, Flat2DArrayError, col, height, row, width};

    #[test]
    fn test_get_pos_from_index() {
        let grid = Flat2DArray::<i32>::new_empty((width(3), height(3)), 0);
        let simd = grid.as_simd();

        // First row
        assert_eq!(simd.get_pos_from_index(0), Some(row(0) + col(0)));
        assert_eq!(simd.get_pos_from_index(1), Some(row(0) + col(1)));
        assert_eq!(simd.get_pos_from_index(2), Some(row(0) + col(2)));

        // Second row
        assert_eq!(simd.get_pos_from_index(3), Some(row(1) + col(0)));
        assert_eq!(simd.get_pos_from_index(4), Some(row(1) + col(1)));
        assert_eq!(simd.get_pos_from_index(5), Some(row(1) + col(2)));

        // Third row
        assert_eq!(simd.get_pos_from_index(6), Some(row(2) + col(0)));
        assert_eq!(simd.get_pos_from_index(7), Some(row(2) + col(1)));
        assert_eq!(simd.get_pos_from_index(8), Some(row(2) + col(2)));

        // Out of bounds
        assert_eq!(simd.get_pos_from_index(9), None);
        assert_eq!(simd.get_pos_from_index(100), None);
    }

    #[test]
    fn test_copy_within_rows() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        // Row 0: [1, 2]
        let _ = grid.try_set(row(0) + col(0), 1);
        let _ = grid.try_set(row(0) + col(1), 2);
        // Row 1: [3, 4]
        let _ = grid.try_set(row(1) + col(0), 3);
        let _ = grid.try_set(row(1) + col(1), 4);

        // Copy Row 0 to Row 2
        grid.as_simd_mut().copy_within_rows(row(0)..row(1), row(2));

        // Row 2 should now be [1, 2]
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(2) + col(1)), Ok(&2));

        // Row 1 should still be [3, 4]
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&3));
    }

    #[test]
    fn test_fill_rows() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);

        // Fill Row 1
        grid.as_simd_mut().fill_rows(row(1)..row(2), 99);

        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&0));
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&99));
        assert_eq!(grid.try_get(row(1) + col(1)), Ok(&99));
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&0));
    }

    #[test]
    fn test_zero_dimensions() {
        let mut grid = Flat2DArray::new_empty((width(0), height(0)), 0);
        assert_eq!(grid.as_simd().as_raw_slice().len(), 0);
        assert_eq!(
            grid.try_set(row(0) + col(0), 1),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_get(row(0) + col(0)),
            Err(Flat2DArrayError::OutOfBounds)
        );
    }

    #[test]
    fn test_copy_within_rows_overlapping() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        // Row 0: [1, 2], Row 1: [3, 4], Row 2: [5, 6]
        let _ = grid.try_set(row(0) + col(0), 1);
        let _ = grid.try_set(row(0) + col(1), 2);
        let _ = grid.try_set(row(1) + col(0), 3);
        let _ = grid.try_set(row(1) + col(1), 4);
        let _ = grid.try_set(row(2) + col(0), 5);
        let _ = grid.try_set(row(2) + col(1), 6);

        // Copy Row 0..2 (Rows 0 and 1) to Row 1..3 (Rows 1 and 2)
        grid.as_simd_mut().copy_within_rows(row(0)..row(2), row(1));

        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&3));
    }

    #[test]
    fn test_copy_within_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        let _ = grid.try_set(row(0) + col(0), 1);

        // Source out of bounds
        grid.as_simd_mut().copy_within_rows(row(0)..row(4), row(1)); // Should not panic

        // Destination out of bounds
        grid.as_simd_mut().copy_within_rows(row(0)..row(1), row(3)); // Should not panic

        // Inverse range (start > end)
        grid.as_simd_mut().copy_within_rows(row(2)..row(1), row(0)); // Should not panic

        // Grid should remain unmodified
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&0));
    }

    #[test]
    fn test_fill_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);

        // Out of bounds end
        grid.as_simd_mut().fill_rows(row(1)..row(5), 99); // Should not panic

        // Inverse range
        grid.as_simd_mut().fill_rows(row(2)..row(1), 99); // Should not panic

        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&0)); // Remained 0
    }

    #[test]
    fn test_fill_all() {
        let mut grid = Flat2DArray::new_empty((width(2), height(2)), 0);
        grid.as_simd_mut().fill_all(42);
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&42));
        assert_eq!(grid.try_get(row(1) + col(1)), Ok(&42));
    }

    #[test]
    fn test_swap_rows() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        // Row 0: [1, 2]
        let _ = grid.try_set(row(0) + col(0), 1);
        let _ = grid.try_set(row(0) + col(1), 2);
        // Row 1: [3, 4]
        let _ = grid.try_set(row(1) + col(0), 3);
        let _ = grid.try_set(row(1) + col(1), 4);
        // Row 2: [5, 6]
        let _ = grid.try_set(row(2) + col(0), 5);
        let _ = grid.try_set(row(2) + col(1), 6);

        // Swap row 0 and 2
        grid.as_simd_mut().swap_rows(row(0), row(2));
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&5));
        assert_eq!(grid.try_get(row(0) + col(1)), Ok(&6));
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(2) + col(1)), Ok(&2));
        // Row 1 unchanged
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&3));

        // Swap row 2 and 1 (reverse order params)
        grid.as_simd_mut().swap_rows(row(2), row(1));
        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&1)); // formerly row 2
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&3)); // formerly row 1
    }

    #[test]
    fn test_swap_rows_out_of_bounds() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        let _ = grid.try_set(row(0) + col(0), 1);

        // Should not panic, just return early
        grid.as_simd_mut().swap_rows(row(0), row(5));
        grid.as_simd_mut().swap_rows(row(5), row(0));

        // Array unchanged
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&1));
    }

    #[test]
    fn test_swap_rows_same_row() {
        let mut grid = Flat2DArray::new_empty((width(2), height(3)), 0);
        let _ = grid.try_set(row(0) + col(0), 1);
        let _ = grid.try_set(row(0) + col(1), 2);

        // Should not panic, just return early
        grid.as_simd_mut().swap_rows(row(0), row(0));

        // Array unchanged
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(0) + col(1)), Ok(&2));
    }
}
