// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{Flat1DSimd, Flat1DSimdMut, Flat2DArray, Flat2DArrayError,
            address_translation};
use crate::{ColWidth, Pos, RowHeight, RowIndex, Size};
use std::ops::{Index, IndexMut};

impl<T> Flat2DArray<T> {
    /// Returns the fixed width (columns) of the 2D grid.
    #[must_use]
    pub fn get_width(&self) -> ColWidth { self.width }

    /// Returns the fixed height (rows) of the 2D grid.
    #[must_use]
    pub fn get_height(&self) -> RowHeight { self.height }

    /// Returns an immutable slice representing the entire row at the specified index.
    ///
    /// This provides standard slice-based access to a single row. For highly optimized
    /// bulk operations (like filling or swapping multiple rows simultaneously), use the
    /// [SIMD] methods instead (e.g., [`as_simd`]).
    ///
    /// [`as_simd`]: Flat2DArray::as_simd
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    #[inline]
    #[must_use]
    pub fn get_row(&self, row_index: usize) -> Option<&[T]> {
        let width = self.width.as_usize();
        if row_index < self.height.as_usize() {
            Some(&self.data[row_index * width..(row_index + 1) * width])
        } else {
            None
        }
    }

    /// Returns a mutable slice representing the entire row at the specified index.
    ///
    /// This provides standard slice-based access to a single row. For highly optimized
    /// bulk operations (like filling or swapping multiple rows simultaneously), use the
    /// [SIMD] methods instead (e.g., [`as_simd_mut`]).
    ///
    /// [`as_simd_mut`]: Flat2DArray::as_simd_mut
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    pub fn get_row_mut(&mut self, row_index: usize) -> Option<&mut [T]> {
        let width = self.width.as_usize();
        if row_index < self.height.as_usize() {
            Some(&mut self.data[row_index * width..(row_index + 1) * width])
        } else {
            None
        }
    }

    /// Grants access to the [SIMD]-optimized read-only fast paths.
    ///
    /// Unlike standard row access methods (e.g., [`get_row`]), which are intended for
    /// cell-by-cell iteration, these methods operate on the underlying memory in bulk.
    ///
    /// **Performance Note**: If you need to iterate over the entire buffer while
    /// maintaining 2D coordinates (row and column indices), see the [Rule of Thumb for 1D
    /// vs 2D Memory Iteration] on how to properly use [`.chunks_exact()`]. This
    /// effectively creates a cache-friendly double loop while explicitly eliminating the
    /// massive CPU pipeline stalls caused by division (`/`) and modulo (`%`) math.
    ///
    /// [`.chunks_exact()`]: slice::chunks_exact
    /// [`get_row`]: Flat2DArray::get_row
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
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
    /// Unlike standard row access methods (e.g., [`get_row_mut`]), which are intended for
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
    /// [`get_row_mut`]: Flat2DArray::get_row_mut
    /// [Rule of Thumb for 1D vs 2D Memory Iteration]:
    ///     crate::Flat1DSimd#rule-of-thumb-for-1d-vs-2d-memory-iteration
    /// [SIMD]: https://en.wikipedia.org/wiki/SIMD
    pub fn as_simd_mut(&mut self) -> Flat1DSimdMut<'_, T> {
        Flat1DSimdMut {
            data: &mut self.data,
            width: self.width,
            height: self.height,
        }
    }
}

impl<T: Clone> Flat2DArray<T> {
    /// Creates a new grid filled with the provided default value.
    ///
    /// By converting the [`Vec`]`<T>` into a [`Box`]`<[T]>` (a wide pointer) upon
    /// initialization, we eliminate its ability to grow or shrink. [`Box`]`<[T]>` only
    /// contains the wide pointer:
    /// - the start address of the memory allocation, and the length,
    /// - but not the capacity (which [`Vec`]`<T>` stores).
    ///
    /// Note the [`Clone`] trait bound on `T` is required in order for [`vec!`] to fill
    /// the array with copies of the `default_val`.
    pub fn new_empty(arg_size: impl Into<Size>, default_val: T) -> Self {
        let Size {
            col_width: width,
            row_height: height,
        } = arg_size.into();

        Self {
            data: {
                // Allocate a Vec on the heap to hold the 1D array.
                let vec = vec![default_val; width.as_usize() * height.as_usize()];
                // Lock the length by dropping the capacity field, so it cannot grow or
                // shrink.
                vec.into_boxed_slice()
            },
            width,
            height,
        }
    }
}

impl<T> Flat2DArray<T> {
    /// Gets a reference to the element at the specified 2D coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`Flat2DArrayError::OutOfBounds`] if the coordinates are outside the grid
    /// dimensions.
    pub fn try_get(&self, arg_pos: impl Into<Pos>) -> Result<&T, Flat2DArrayError> {
        let Some(index) =
            address_translation::pos_to_index(arg_pos, self.width, self.height)
        else {
            return Err(Flat2DArrayError::OutOfBounds);
        };
        // get_index already bounds-checked the coordinates, so direct indexing is safe.
        Ok(&self.data[index])
    }

    /// Gets a mutable reference to the element at the specified 2D coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`Flat2DArrayError::OutOfBounds`] if the coordinates are outside the grid
    /// dimensions.
    pub fn try_get_mut(
        &mut self,
        arg_pos: impl Into<Pos>,
    ) -> Result<&mut T, Flat2DArrayError> {
        let Some(index) =
            address_translation::pos_to_index(arg_pos, self.width, self.height)
        else {
            return Err(Flat2DArrayError::OutOfBounds);
        };
        // get_index already bounds-checked the coordinates, so direct indexing is safe.
        Ok(&mut self.data[index])
    }

    /// Sets the element at the specified 2D coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`Flat2DArrayError::OutOfBounds`] if the coordinates are outside the grid
    /// dimensions.
    pub fn try_set(
        &mut self,
        arg_pos: impl Into<Pos>,
        val: T,
    ) -> Result<(), Flat2DArrayError> {
        let target = self.try_get_mut(arg_pos)?;
        *target = val;
        Ok(())
    }
}

impl<T> Index<usize> for Flat2DArray<T> {
    type Output = [T];

    /// Allows indexing a specific row as a standard 1D slice. This enables
    /// `buffer[row][col]` syntax because the first index returns a `&[T]` slice, and the
    /// second index natively calls the standard library slice indexer.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds, fulfilling the [`Index`] contract.
    ///
    /// [`Index`]: std::ops::Index
    fn index(&self, row_idx: usize) -> &Self::Output {
        let range =
            address_translation::row_idx_to_bounds(row_idx, self.width, self.height);
        &self.data[range]
    }
}

impl<T> IndexMut<usize> for Flat2DArray<T> {
    /// Allows mutable indexing of a specific row as a standard 1D slice. This enables
    /// `buffer[row][col] = val` syntax because the first index returns a `&mut [T]`
    /// slice, and the second index natively calls the standard library slice indexer.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    fn index_mut(&mut self, row_idx: usize) -> &mut Self::Output {
        let range =
            address_translation::row_idx_to_bounds(row_idx, self.width, self.height);
        &mut self.data[range]
    }
}

impl<T> Index<RowIndex> for Flat2DArray<T> {
    type Output = [T];

    /// Allows indexing a specific row using a strongly-typed [`RowIndex`]. This enables
    /// `buffer[row(1)][col]` syntax because the first index returns a `&[T]` slice, and
    /// the second index natively calls the standard library slice indexer.
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    fn index(&self, row: RowIndex) -> &Self::Output { &self[row.as_usize()] }
}

impl<T> IndexMut<RowIndex> for Flat2DArray<T> {
    /// Allows mutable indexing of a specific row using a strongly-typed [`RowIndex`].
    ///
    /// # Panics
    ///
    /// Panics if the row index is out of bounds.
    fn index_mut(&mut self, row: RowIndex) -> &mut Self::Output {
        &mut self[row.as_usize()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_get_set_bounds() {
        let mut grid = Flat2DArray::new_empty((width(3), height(2)), 0);

        // Valid set
        assert_eq!(grid.try_set(row(0) + col(0), 1), Ok(()));
        assert_eq!(grid.try_set(row(1) + col(2), 2), Ok(()));

        // Invalid set
        assert_eq!(
            grid.try_set(row(2) + col(0), 3),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_set(row(0) + col(3), 4),
            Err(Flat2DArrayError::OutOfBounds)
        );

        // Valid get
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&1));
        assert_eq!(grid.try_get(row(1) + col(2)), Ok(&2));

        // Invalid get
        assert_eq!(
            grid.try_get(row(2) + col(0)),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_get(row(0) + col(3)),
            Err(Flat2DArrayError::OutOfBounds)
        );
    }

    #[test]
    fn test_get_row_range() {
        let mut grid = Flat2DArray::new_empty((width(3), height(2)), 0);

        // Valid set
        assert!(grid.try_set(row(1) + col(2), 42).is_ok());
        assert_eq!(grid.try_get(row(1) + col(2)), Ok(&42));

        // Valid try_get_mut
        if let Ok(val) = grid.try_get_mut(row(1) + col(2)) {
            *val = 45;
        }
        assert_eq!(grid.try_get(row(1) + col(2)), Ok(&45));

        // Test Index trait as well
        grid[1][2] = 43;
        assert_eq!(grid[1][2], 43);

        grid[row(1)][2] = 44;
        assert_eq!(grid[row(1)][2], 44);

        // Invalid set (out of bounds col)
        assert_eq!(
            grid.try_set(row(0) + col(3), 99),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_get(row(0) + col(3)),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_get_mut(row(0) + col(3)),
            Err(Flat2DArrayError::OutOfBounds)
        );

        // Invalid set (out of bounds row)
        assert_eq!(
            grid.try_set(row(2) + col(0), 99),
            Err(Flat2DArrayError::OutOfBounds)
        );
        assert_eq!(
            grid.try_get(row(2) + col(0)),
            Err(Flat2DArrayError::OutOfBounds)
        );
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_index_out_of_bounds_panics() {
        let grid = Flat2DArray::new_empty((width(3), height(2)), 0);
        let _ = &grid[2]; // Height is 2, so index 2 is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_index_mut_out_of_bounds_panics() {
        let mut grid = Flat2DArray::new_empty((width(3), height(2)), 0);
        grid[2][0] = 99; // Height is 2, so index 2 is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_row_index_out_of_bounds_panics() {
        let grid = Flat2DArray::new_empty((width(3), height(2)), 0);
        let _ = &grid[row(2)]; // Height is 2, so row(2) is out of bounds
    }

    #[test]
    #[should_panic(expected = "row index out of bounds")]
    fn test_row_index_mut_out_of_bounds_panics() {
        let mut grid = Flat2DArray::new_empty((width(3), height(2)), 0);
        grid[row(2)][0] = 99; // Height is 2, so row(2) is out of bounds
    }
}
