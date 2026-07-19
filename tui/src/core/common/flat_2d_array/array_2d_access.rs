// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words indexmut

use super::{Flat1DSimd, Flat1DSimdMut, Flat2DArray};
use crate::{c_row, ArrayBoundsCheck, ArrayOverflowResult, CRow, CWidth, RangeExclusive};
use std::ops::{Index, IndexMut};

impl<T> Flat2DArray<T> {
    /// Returns the fixed width (columns) of the 2D grid.
    #[must_use]
    pub fn get_width(&self) -> CWidth { self.width }

    /// Grants access to the [SIMD]-optimized read-only fast paths.
    ///
    /// Unlike standard slice indexing (e.g., `&grid[row]`), which is intended for
    /// cell-by-cell iteration, these methods operate on the underlying memory in bulk.
    ///
    /// **Performance Note**: If you need to iterate over the entire buffer while
    /// maintaining 2D coordinates (row and column indices), see the [Rule of Thumb for 1D
    /// vs 2D Memory Iteration] on how to properly use [`.chunks_exact()`]. This
    /// effectively creates a cache-friendly double loop while explicitly eliminating the
    /// massive CPU pipeline stalls caused by division (`/`) and modulo (`%`) math.
    ///
    /// [`.chunks_exact()`]: slice::chunks_exact
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::core::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[inline]
    #[must_use]
    pub fn as_simd(&self) -> Flat1DSimd<'_, T> {
        Flat1DSimd {
            data: &self.data,
            width: self.width,
            height: self.height,
        }
    }

    /// Grants access to the [SIMD]-optimized mutable fast paths.
    ///
    /// Unlike standard slice indexing (e.g., `&mut grid[row]`), which is intended for
    /// cell-by-cell iteration, these methods bypass loops and use raw memory operations
    /// (like slice filling or swapping) to manipulate entire chunks of the grid
    /// simultaneously.
    ///
    /// **Performance Note**: If you need to iterate over the entire buffer while
    /// maintaining 2D coordinates (row and column indices), see the [Rule of Thumb for 1D
    /// vs 2D Memory Iteration] on how to properly use [`.chunks_exact_mut()`]. This
    /// effectively creates a cache-friendly double loop while explicitly eliminating the
    /// massive CPU pipeline stalls caused by division (`/`) and modulo (`%`) math.
    ///
    /// [`.chunks_exact_mut()`]: slice::chunks_exact_mut
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::core::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    pub fn as_simd_mut(&mut self) -> Flat1DSimdMut<'_, T> {
        Flat1DSimdMut {
            data: &mut self.data,
            width: self.width,
            height: self.height,
        }
    }

    /// Helper to get the 1D slice range for a specific row index.
    ///
    /// # 1D Slice Range Mapping
    ///
    /// The returned range `start..end` corresponds exactly to the flat 1D slice indices
    /// for that specific row. `row_idx` represents the zero-indexed row number (e.g., `0`
    /// for the first row).
    ///
    /// ```text
    /// 1D Grid:
    ///
    ///   row 0                   row 1                   row 2
    ///   col 0   col 1   col 2   col 0   col 1   col 2   col 0   col 1   col 2
    /// ┌───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┬───────┐
    /// │ idx 0 │ idx 1 │ idx 2 │ idx 3 │ idx 4 │ idx 5 │ idx 6 │ idx 7 │ idx 8 │
    /// └───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┴───────┘
    ///
    /// 2D Grid (equivalent):
    ///
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
    /// This function accepts a raw [`usize`] instead of a strongly-typed
    /// [`CRow`] because it serves as the foundational engine for the
    /// [`Index<usize>`] and [`IndexMut<usize>`] implementations. Those traits must
    /// accept [`usize`] to enable standard native `[row][col]` ergonomics.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    ///
    /// [`CRow`]: crate::CRow
    /// [`Index<usize>`]: std::ops::Index
    /// [`IndexMut<usize>`]: std::ops::IndexMut
    #[inline]
    #[must_use]
    pub fn row_idx_to_bounds(&self, row_idx: usize) -> RangeExclusive<usize> {
        assert!(
            c_row(row_idx).overflows(self.height) == ArrayOverflowResult::Within,
            "row index out of bounds: the height is {} but the index is {}",
            self.height.as_usize(),
            row_idx
        );

        let width_usize = self.width.as_usize();
        let row_offset_start_idx = row_idx * width_usize;
        let row_offset_end_idx = row_offset_start_idx + width_usize;

        row_offset_start_idx..row_offset_end_idx
    }
}

impl<T> Index<usize> for Flat2DArray<T> {
    type Output = [T];

    /// Allows indexing a specific row as a standard 1D slice. This enables
    /// `buffer[row][col]` syntax because the first index returns a `&[T]` slice, and the
    /// second index natively calls the standard library slice indexer.
    ///
    /// # Do not use [`Index`] or [`IndexMut`] for bulk iteration
    ///
    /// Please see the [Do not use `Index` or `IndexMut` for bulk iteration] section in
    /// [`Flat2DArray`] for why you should avoid nested `for` loops and use SIMD instead.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds, fulfilling the [`Index`] contract.
    ///
    /// [`Index`]: std::ops::Index
    /// [Do not use `Index` or `IndexMut` for bulk iteration]:
    ///     Flat2DArray#do-not-use-index-or-indexmut-for-bulk-iteration
    fn index(&self, row_idx: usize) -> &Self::Output {
        let range = self.row_idx_to_bounds(row_idx);
        &self.data[range]
    }
}

impl<T> IndexMut<usize> for Flat2DArray<T> {
    /// Allows mutable indexing of a specific row as a standard 1D slice. This enables
    /// `buffer[row][col] = val` syntax because the first index returns a `&mut [T]`
    /// slice, and the second index natively calls the standard library slice indexer.
    ///
    /// # Do not use [`Index`] or [`IndexMut`] for bulk iteration
    ///
    /// Please see the [Do not use `Index` or `IndexMut` for bulk iteration] section in
    /// [`Flat2DArray`] for why you should avoid nested `for` loops and use SIMD instead.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    ///
    /// [Do not use `Index` or `IndexMut` for bulk iteration]:
    ///     Flat2DArray#do-not-use-index-or-indexmut-for-bulk-iteration
    fn index_mut(&mut self, row_idx: usize) -> &mut Self::Output {
        let range = self.row_idx_to_bounds(row_idx);
        &mut self.data[range]
    }
}

impl<T> Index<CRow> for Flat2DArray<T> {
    type Output = [T];

    /// Allows indexing a specific row using a strongly-typed [`CRow`]. This
    /// enables `buffer[c_row(1)][col]` syntax because the first index returns a
    /// `&[T]` slice, and the second index natively calls the standard library slice
    /// indexer.
    ///
    /// # Do not use [`Index`] or [`IndexMut`] for bulk iteration
    ///
    /// Please see the [Do not use `Index` or `IndexMut` for bulk iteration] section in
    /// [`Flat2DArray`] for why you should avoid nested `for` loops and use SIMD instead.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    ///
    /// [Do not use `Index` or `IndexMut` for bulk iteration]:
    ///     Flat2DArray#do-not-use-index-or-indexmut-for-bulk-iteration
    fn index(&self, row: CRow) -> &Self::Output { &self[row.as_usize()] }
}

impl<T> IndexMut<CRow> for Flat2DArray<T> {
    /// Allows mutable indexing of a specific row using a strongly-typed
    /// [`CRow`].
    ///
    /// # Do not use [`Index`] or [`IndexMut`] for bulk iteration
    ///
    /// Please see the [Do not use `Index` or `IndexMut` for bulk iteration] section in
    /// [`Flat2DArray`] for why you should avoid nested `for` loops and use SIMD instead.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    ///
    /// [Do not use `Index` or `IndexMut` for bulk iteration]:
    ///     Flat2DArray#do-not-use-index-or-indexmut-for-bulk-iteration
    fn index_mut(&mut self, row: CRow) -> &mut Self::Output { &mut self[row.as_usize()] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CHeight, CSize, CWidth, c_row};

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_index_out_of_bounds_panics() {
        let grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(3usize), CHeight::from(2usize))),
            0,
        );
        let _ = &grid[2]; // Height is 2, so index 2 is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_index_mut_out_of_bounds_panics() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(3usize), CHeight::from(2usize))),
            0,
        );
        grid[2][0] = 99; // Height is 2, so index 2 is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_c_row_index_out_of_bounds_panics() {
        let grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(3usize), CHeight::from(2usize))),
            0,
        );
        let _ = &grid[c_row(2)]; // Height is 2, so c_row(2) is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_c_row_index_mut_out_of_bounds_panics() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(3usize), CHeight::from(2usize))),
            0,
        );
        grid[c_row(2)][0] = 99; // Height is 2, so c_row(2) is out of bounds
    }

    #[test]
    fn test_large_canvas_grid_beyond_u16_max() {
        // Create grid with 70,000 rows (exceeding u16::MAX = 65,535).
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(2usize), CHeight::from(70_000usize))),
            0u8,
        );
        grid[c_row(69_999)][1] = 42;
        assert_eq!(grid[c_row(69_999)][1], 42);
        assert_eq!(grid.height.as_usize(), 70_000);
    }

    #[test]
    fn test_canvas_indexing_with_c_row_and_col() {
        let mut grid = Flat2DArray::new_empty(
            CSize::from((CWidth::from(4usize), CHeight::from(4usize))),
            0u8,
        );
        grid[c_row(1)][2] = 99;
        assert_eq!(grid[c_row(1)][2], 99);
    }
}
