// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Range validation utilities for [`Flat2DArray`] operations.
//!
//! This module provides type-safe range validation methods for [`Flat2DArray`] that
//! eliminate the need for manual bounds checking and `unwrap()` calls. All methods return
//! [`Option`] types for safe buffer access.

use crate::{ColIndex, Flat2DArray, RangeBoundsExt, RangeValidityStatus, RowIndex, RangeExt};
use std::ops::Range;

impl<T> Flat2DArray<T> {
    /// Checks if a row range is valid without creating slice references.
    #[must_use]
    pub fn is_row_range_valid(&self, row_range: Range<RowIndex>) -> bool {
        row_range.check_range_is_valid_for_length(self.height)
            == RangeValidityStatus::Valid
    }

    /// Checks if a column range within a specific row is valid without creating
    /// references.
    #[must_use]
    pub fn is_col_range_valid(&self, row: RowIndex, col_range: Range<ColIndex>) -> bool {
        let row_idx = row.as_usize();
        self.get_row(row_idx).is_some_and(|_line| {
            col_range.check_range_is_valid_for_length(self.width)
                == RangeValidityStatus::Valid
        })
    }

    /// Validate a row range and return immutable access to the buffer lines as a 1D
    /// slice.
    #[must_use]
    pub fn validate_row_range(
        &self,
        row_range: Range<RowIndex>,
    ) -> Option<(usize, usize, &[T])> {
        if row_range.check_range_is_valid_for_length(self.height)
            != RangeValidityStatus::Valid
        {
            return None;
        }

        let usize_range = row_range.as_usize_range();
        let start_idx = usize_range.start;
        let end_idx = usize_range.end;

        let width = self.width.as_usize();
        let start_offset = start_idx * width;
        let end_offset = end_idx * width;
        let slice = &self.data[start_offset..end_offset];

        Some((start_idx, end_idx, slice))
    }

    /// Validate a row range and return mutable access to the buffer lines as a 1D slice.
    pub fn validate_row_range_mut(
        &mut self,
        row_range: Range<RowIndex>,
    ) -> Option<(usize, usize, &mut [T])> {
        if row_range.check_range_is_valid_for_length(self.height)
            != RangeValidityStatus::Valid
        {
            return None;
        }

        let usize_range = row_range.as_usize_range();
        let start_idx = usize_range.start;
        let end_idx = usize_range.end;

        let width = self.width.as_usize();
        let start_offset = start_idx * width;
        let end_offset = end_idx * width;
        let slice_mut = &mut self.data[start_offset..end_offset];

        Some((start_idx, end_idx, slice_mut))
    }

    /// Validate a column range within a specific row and return immutable access.
    #[must_use]
    pub fn validate_col_range(
        &self,
        row: RowIndex,
        col_range: Range<ColIndex>,
    ) -> Option<(usize, usize, &[T])> {
        let row_idx = row.as_usize();
        self.get_row(row_idx).and_then(|line| {
            if col_range.check_range_is_valid_for_length(self.width)
                == RangeValidityStatus::Valid
            {
                let usize_range = col_range.as_usize_range();
                let start_idx = usize_range.start;
                let end_idx = usize_range.end;
                Some((start_idx, end_idx, line))
            } else {
                None
            }
        })
    }

    /// Validate a column range within a specific row and return mutable access.
    pub fn validate_col_range_mut(
        &mut self,
        row: RowIndex,
        col_range: Range<ColIndex>,
    ) -> Option<(usize, usize, &mut [T])> {
        let row_idx = row.as_usize();
        let width = self.width;
        self.get_row_mut(row_idx).and_then(|line| {
            if col_range.check_range_is_valid_for_length(width)
                == RangeValidityStatus::Valid
            {
                let usize_range = col_range.as_usize_range();
                let start_idx = usize_range.start;
                let end_idx = usize_range.end;
                Some((start_idx, end_idx, line))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{col, height, row, width};

    #[test]
    fn test_is_row_range_valid() {
        let grid = Flat2DArray::<u8>::new_empty((width(3), height(3)), 0);

        // Valid ranges
        assert!(grid.is_row_range_valid(row(0)..row(3)));
        assert!(grid.is_row_range_valid(row(1)..row(2)));

        // Invalid ranges
        assert!(!grid.is_row_range_valid(row(0)..row(4)));
        assert!(!grid.is_row_range_valid(row(3)..row(4)));
        assert!(!grid.is_row_range_valid(row(2)..row(1))); // start > end
    }

    #[test]
    fn test_is_col_range_valid() {
        let grid = Flat2DArray::<u8>::new_empty((width(3), height(3)), 0);

        // Valid ranges on valid row
        assert!(grid.is_col_range_valid(row(1), col(0)..col(3)));
        assert!(grid.is_col_range_valid(row(1), col(1)..col(2)));

        // Invalid col ranges on valid row
        assert!(!grid.is_col_range_valid(row(1), col(0)..col(4)));
        assert!(!grid.is_col_range_valid(row(1), col(2)..col(1)));

        // Valid col ranges on invalid row
        assert!(!grid.is_col_range_valid(row(4), col(0)..col(3)));
    }

    #[test]
    fn test_validate_row_range() {
        let grid = Flat2DArray::<u8>::new_empty((width(3), height(3)), 0);

        // Valid
        let res = grid.validate_row_range(row(1)..row(3));
        assert!(res.is_some());
        let (start, end, slice) = res.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 3);
        assert_eq!(slice.len(), 6); // 2 rows * 3 cols

        // Invalid
        assert!(grid.validate_row_range(row(1)..row(4)).is_none());
    }

    #[test]
    fn test_validate_row_range_mut() {
        let mut grid = Flat2DArray::<u8>::new_empty((width(3), height(3)), 0);

        let res = grid.validate_row_range_mut(row(0)..row(2));
        assert!(res.is_some());
        let (start, end, slice) = res.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 2);
        assert_eq!(slice.len(), 6);

        slice.fill(99);
        assert_eq!(grid.try_get(row(0) + col(0)), Ok(&99));
        assert_eq!(grid.try_get(row(2) + col(0)), Ok(&0));
    }

    #[test]
    fn test_validate_col_range() {
        let grid = Flat2DArray::<u8>::new_empty((width(4), height(3)), 0);

        // Valid
        let res = grid.validate_col_range(row(2), col(1)..col(3));
        assert!(res.is_some());
        let (start, end, slice) = res.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 3);
        assert_eq!(slice.len(), 4); // The whole row slice is returned!
        // Wait, the API returns (start_idx, end_idx, line)
        // so line is the whole row.

        // Invalid row
        assert!(grid.validate_col_range(row(4), col(1)..col(3)).is_none());

        // Invalid col
        assert!(grid.validate_col_range(row(2), col(1)..col(5)).is_none());
    }

    #[test]
    fn test_validate_col_range_mut() {
        let mut grid = Flat2DArray::<u8>::new_empty((width(4), height(3)), 0);

        let res = grid.validate_col_range_mut(row(1), col(1)..col(3));
        assert!(res.is_some());
        let (start, end, line) = res.unwrap();

        line[start..end].fill(42);

        assert_eq!(grid.try_get(row(1) + col(0)), Ok(&0));
        assert_eq!(grid.try_get(row(1) + col(1)), Ok(&42));
        assert_eq!(grid.try_get(row(1) + col(2)), Ok(&42));
        assert_eq!(grid.try_get(row(1) + col(3)), Ok(&0));
    }
}
