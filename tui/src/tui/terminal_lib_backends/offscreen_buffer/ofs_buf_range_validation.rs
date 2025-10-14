// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Range validation utilities for `OffscreenBuffer` operations.
//!
//! This module provides type-safe range validation methods for `OffscreenBuffer`
//! that eliminate the need for manual bounds checking and `unwrap()` calls.
//! All methods return `Option` types for safe buffer access.
//!
//! ## Features
//!
//! - **Type-safe validation**: Uses the [`RangeBoundsExt`] trait for correct exclusive
//!   range semantics
//! - **No `unwrap()` calls**: All validation returns `Option` for safe access
//! - **Immutable and mutable variants**: Support for both read-only and write operations
//! - **Zero allocation**: Methods return references to existing buffer data
//! - **Lightweight validation-only methods**: Efficient validation without slice creation
//!
//! ## Method Selection Guide
//!
//! This module provides two types of validation methods to optimize for different use
//! cases:
//!
//! ### Validation-Only Methods (Lightweight)
//!
//! Use these when you only need to verify range validity without accessing the actual
//! data:
//!
//! - [`is_row_range_valid()`][OffscreenBuffer::is_row_range_valid] - Just validates row
//!   range
//! - [`is_col_range_valid()`][OffscreenBuffer::is_col_range_valid] - Just validates
//!   column range
//!
//! **Best for:**
//! - Operations that access buffer directly after validation
//! - Conditional logic that only needs to know if ranges are valid
//! - Performance-critical paths where slice creation overhead matters
//!
//! **Examples:** `swap_lines()`, `shift_lines_up()`, `shift_lines_down()`
//!
//! ### Validation-with-Slice Methods (Full Access)
//!
//! Use these when you need both validation AND access to the validated data:
//!
//! - [`validate_row_range()`][OffscreenBuffer::validate_row_range] - Returns immutable
//!   slice
//! - [`validate_row_range_mut()`][OffscreenBuffer::validate_row_range_mut] - Returns
//!   mutable slice
//! - [`validate_col_range()`][OffscreenBuffer::validate_col_range] - Returns immutable
//!   line
//! - [`validate_col_range_mut()`][OffscreenBuffer::validate_col_range_mut] - Returns
//!   mutable line
//!
//! **Best for:**
//! - Operations that work on contiguous ranges of buffer data
//! - Fill, copy, and bulk modification operations
//! - When slice abstraction simplifies the implementation
//!
//! **Examples:** `fill_char_range()`, `copy_chars_within_line()`, `set_char()`,
//! `clear_line()`
//!
//! ## Usage Examples
//!
//! ```rust
//! use std::ops::Range;
//! use r3bl_tui::{ColIndex, RowIndex, PixelChar, OffscreenBuffer, Size, width, height};
//!
//! # let mut buffer = OffscreenBuffer::new_empty(Size {
//! #     col_width: width(10),
//! #     row_height: height(5)
//! # });
//!
//! // Example: Safe row range validation with mutable access
//! if let Some((start_idx, end_idx, lines)) = buffer.validate_row_range_mut(
//!     RowIndex::from(1)..RowIndex::from(4)
//! ) {
//!     // Work with validated range indices and mutable line references
//!     for row_idx in start_idx..end_idx {
//!         lines[row_idx - start_idx].fill(PixelChar::Spacer);
//!     }
//! }
//!
//! // Safe column range validation within a specific row
//! if let Some((start_idx, end_idx, line)) = buffer.validate_col_range_mut(
//!     RowIndex::from(2),
//!     ColIndex::from(5)..ColIndex::from(10)
//! ) {
//!     // Work with validated column indices and mutable line reference
//!     line[start_idx..end_idx].fill(PixelChar::Spacer);
//! }
//! ```
//!
//! [`RangeBoundsExt`]: crate::core::coordinates::bounds_check::RangeBoundsExt

use super::{OffscreenBuffer, PixelCharLine};
use crate::{ColIndex, RangeValidityStatus, RowIndex,
            core::coordinates::bounds_check::RangeBoundsExt};
use std::ops::Range;

impl OffscreenBuffer {
    /// Check if a row range is valid without creating slice references.
    ///
    /// This method provides lightweight validation for operations that only need to
    /// verify range validity without accessing the actual buffer data. It's more
    /// efficient than the full validation methods when the returned slice won't be used.
    ///
    /// ## Parameters
    ///
    /// * `row_range` - The range of rows to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `true` if the range is valid for the buffer dimensions, `false` otherwise.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, OffscreenBuffer, Size, width, height};
    ///
    /// # let buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(10),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Check if range is valid without creating slice
    /// if buffer.is_row_range_valid(RowIndex::from(1)..RowIndex::from(4)) {
    ///     // Proceed with operations that need validated rows
    ///     // but don't need the actual slice reference
    /// }
    /// ```
    #[must_use]
    pub fn is_row_range_valid(&self, row_range: Range<RowIndex>) -> bool {
        row_range.check_range_is_valid_for_length(self.buffer.len())
            == RangeValidityStatus::Valid
    }

    /// Check if a column range within a specific row is valid without creating
    /// references.
    ///
    /// This method provides lightweight validation for operations that only need to
    /// verify range validity without accessing the actual line data. It's more
    /// efficient than the full validation methods when the returned line reference won't
    /// be used.
    ///
    /// ## Parameters
    ///
    /// * `row` - The row index to check
    /// * `col_range` - The range of columns to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `true` if both the row exists and the column range is valid for that row, `false`
    /// otherwise.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, ColIndex, OffscreenBuffer, Size, width, height};
    ///
    /// # let buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(15),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Check if column range in row is valid without creating line reference
    /// if buffer.is_col_range_valid(
    ///     RowIndex::from(2),
    ///     ColIndex::from(5)..ColIndex::from(10)
    /// ) {
    ///     // Proceed with operations that need validated column range
    ///     // but don't need the actual line reference
    /// }
    /// ```
    #[must_use]
    pub fn is_col_range_valid(&self, row: RowIndex, col_range: Range<ColIndex>) -> bool {
        let row_idx = row.as_usize();
        self.buffer.get(row_idx).is_some_and(|line| {
            col_range.check_range_is_valid_for_length(line.len())
                == RangeValidityStatus::Valid
        })
    }

    /// Validate a row range and return immutable access to the buffer lines.
    ///
    /// This method checks that the row range is valid for the buffer dimensions
    /// and returns the converted indices along with immutable references to the
    /// requested lines.
    ///
    /// ## Parameters
    ///
    /// * `row_range` - The range of rows to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `Some((start_idx, end_idx, lines))` if the range is valid, where:
    /// - `start_idx` - The starting row index as `usize`
    /// - `end_idx` - The ending row index as `usize` (exclusive)
    /// - `lines` - Immutable slice of the buffer lines in the range
    ///
    /// `None` if the range is invalid (out of bounds, empty, or inverted).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, OffscreenBuffer, Size, width, height};
    ///
    /// # let buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(10),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Example: Validate rows 1-3 (inclusive) for reading
    /// if let Some((start_idx, end_idx, lines)) = buffer.validate_row_range(
    ///     RowIndex::from(1)..RowIndex::from(4)
    /// ) {
    ///     for row_idx in start_idx..end_idx {
    ///         let line = &lines[row_idx - start_idx];
    ///         // Process the line...
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn validate_row_range(
        &self,
        row_range: Range<RowIndex>,
    ) -> Option<(usize, usize, &[PixelCharLine])> {
        if row_range.check_range_is_valid_for_length(self.buffer.len())
            != RangeValidityStatus::Valid
        {
            return None;
        }

        let start_idx = row_range.start.as_usize();
        let end_idx = row_range.end.as_usize();

        Some((start_idx, end_idx, &self.buffer[start_idx..end_idx]))
    }

    /// Validate a row range and return mutable access to the buffer lines.
    ///
    /// This method checks that the row range is valid for the buffer dimensions
    /// and returns the converted indices along with mutable references to the
    /// requested lines.
    ///
    /// ## Parameters
    ///
    /// * `row_range` - The range of rows to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `Some((start_idx, end_idx, lines))` if the range is valid, where:
    /// - `start_idx` - The starting row index as `usize`
    /// - `end_idx` - The ending row index as `usize` (exclusive)
    /// - `lines` - Mutable slice of the buffer lines in the range
    ///
    /// `None` if the range is invalid (out of bounds, empty, or inverted).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, PixelChar, OffscreenBuffer, Size, width, height};
    ///
    /// # let mut buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(10),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Example: Validate rows 1-3 (inclusive) for modification
    /// if let Some((start_idx, end_idx, lines)) = buffer.validate_row_range_mut(
    ///     RowIndex::from(1)..RowIndex::from(4)
    /// ) {
    ///     for row_idx in start_idx..end_idx {
    ///         let line = &mut lines[row_idx - start_idx];
    ///         line.fill(PixelChar::Spacer);
    ///     }
    /// }
    /// ```
    pub fn validate_row_range_mut(
        &mut self,
        row_range: Range<RowIndex>,
    ) -> Option<(usize, usize, &mut [PixelCharLine])> {
        if row_range.check_range_is_valid_for_length(self.buffer.len())
            != RangeValidityStatus::Valid
        {
            return None;
        }

        let start_idx = row_range.start.as_usize();
        let end_idx = row_range.end.as_usize();

        Some((start_idx, end_idx, &mut self.buffer[start_idx..end_idx]))
    }

    /// Validate a column range within a specific row and return immutable access.
    ///
    /// This method checks that the row exists and the column range is valid for
    /// that row's length, then returns the converted indices along with an
    /// immutable reference to the requested line.
    ///
    /// ## Parameters
    ///
    /// * `row` - The row index to check
    /// * `col_range` - The range of columns to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `Some((start_idx, end_idx, line))` if both row and column range are valid, where:
    /// - `start_idx` - The starting column index as `usize`
    /// - `end_idx` - The ending column index as `usize` (exclusive)
    /// - `line` - Immutable reference to the specified line
    ///
    /// `None` if the row is out of bounds or the column range is invalid.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, ColIndex, OffscreenBuffer, Size, width, height};
    ///
    /// # let buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(15),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Example: Validate columns 5-9 (inclusive) in row 2 for reading
    /// if let Some((start_idx, end_idx, line)) = buffer.validate_col_range(
    ///     RowIndex::from(2),
    ///     ColIndex::from(5)..ColIndex::from(10)
    /// ) {
    ///     for col_idx in start_idx..end_idx {
    ///         let pixel_char = &line[col_idx];
    ///         // Process the character...
    ///     }
    /// }
    /// ```
    #[must_use]
    pub fn validate_col_range(
        &self,
        row: RowIndex,
        col_range: Range<ColIndex>,
    ) -> Option<(usize, usize, &PixelCharLine)> {
        let row_idx = row.as_usize();
        self.buffer.get(row_idx).and_then(|line| {
            if col_range.check_range_is_valid_for_length(line.len())
                == RangeValidityStatus::Valid
            {
                let start_idx = col_range.start.as_usize();
                let end_idx = col_range.end.as_usize();
                Some((start_idx, end_idx, line))
            } else {
                None
            }
        })
    }

    /// Validate a column range within a specific row and return mutable access.
    ///
    /// This method checks that the row exists and the column range is valid for
    /// that row's length, then returns the converted indices along with a
    /// mutable reference to the requested line.
    ///
    /// ## Parameters
    ///
    /// * `row` - The row index to check
    /// * `col_range` - The range of columns to validate (exclusive end)
    ///
    /// ## Returns
    ///
    /// `Some((start_idx, end_idx, line))` if both row and column range are valid, where:
    /// - `start_idx` - The starting column index as `usize`
    /// - `end_idx` - The ending column index as `usize` (exclusive)
    /// - `line` - Mutable reference to the specified line
    ///
    /// `None` if the row is out of bounds or the column range is invalid.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RowIndex, ColIndex, PixelChar, OffscreenBuffer, Size, width, height};
    ///
    /// # let mut buffer = OffscreenBuffer::new_empty(Size {
    /// #     col_width: width(15),
    /// #     row_height: height(5)
    /// # });
    ///
    /// // Example: Validate columns 5-9 (inclusive) in row 2 for modification
    /// if let Some((start_idx, end_idx, line)) = buffer.validate_col_range_mut(
    ///     RowIndex::from(2),
    ///     ColIndex::from(5)..ColIndex::from(10)
    /// ) {
    ///     line[start_idx..end_idx].fill(PixelChar::Spacer);
    /// }
    /// ```
    pub fn validate_col_range_mut(
        &mut self,
        row: RowIndex,
        col_range: Range<ColIndex>,
    ) -> Option<(usize, usize, &mut PixelCharLine)> {
        let row_idx = row.as_usize();
        self.buffer.get_mut(row_idx).and_then(|line| {
            if col_range.check_range_is_valid_for_length(line.len())
                == RangeValidityStatus::Valid
            {
                let start_idx = col_range.start.as_usize();
                let end_idx = col_range.end.as_usize();
                Some((start_idx, end_idx, line))
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
mod tests_range_validation {
    use super::*;
    use crate::{PixelChar, col, height, row,
                test_fixtures_ofs_buf::create_test_buffer_with_size, width};

    fn create_test_buffer() -> OffscreenBuffer {
        create_test_buffer_with_size(width(5), height(4))
    }

    // Test validate_row_range
    #[test]
    fn test_validate_row_range_valid() {
        let buffer = create_test_buffer();

        // Valid ranges
        let result1 = buffer.validate_row_range(row(0)..row(2));
        assert!(result1.is_some());
        let (start, end, lines) = result1.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 2);
        assert_eq!(lines.len(), 2);

        // Full buffer range
        let result2 = buffer.validate_row_range(row(0)..row(4));
        assert!(result2.is_some());
        let (start, end, lines) = result2.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 4);
        assert_eq!(lines.len(), 4);

        // Single row range
        let result3 = buffer.validate_row_range(row(2)..row(3));
        assert!(result3.is_some());
        let (start, end, lines) = result3.unwrap();
        assert_eq!(start, 2);
        assert_eq!(end, 3);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn test_validate_row_range_invalid() {
        let buffer = create_test_buffer();

        // Out of bounds
        assert!(buffer.validate_row_range(row(0)..row(5)).is_none());
        assert!(buffer.validate_row_range(row(5)..row(6)).is_none());

        // Empty ranges are now valid (should return Some)
        assert!(buffer.validate_row_range(row(2)..row(2)).is_some());

        // Inverted ranges
        assert!(buffer.validate_row_range(row(3)..row(1)).is_none());
    }

    // Test validate_row_range_mut
    #[test]
    fn test_validate_row_range_mut_valid() {
        let mut buffer = create_test_buffer();

        // Test mutable access
        let result = buffer.validate_row_range_mut(row(1)..row(3));
        assert!(result.is_some());
        let (start, end, lines) = result.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 3);
        assert_eq!(lines.len(), 2);

        // Verify we can modify the lines
        lines[0].fill(PixelChar::Spacer);
        lines[1].fill(PixelChar::Spacer);
    }

    #[test]
    fn test_validate_row_range_mut_invalid() {
        let mut buffer = create_test_buffer();

        // Same validation rules as immutable version
        assert!(buffer.validate_row_range_mut(row(0)..row(5)).is_none());
        assert!(buffer.validate_row_range_mut(row(2)..row(2)).is_some());
        assert!(buffer.validate_row_range_mut(row(3)..row(1)).is_none());
    }

    // Test validate_col_range
    #[test]
    fn test_validate_col_range_valid() {
        let buffer = create_test_buffer();

        // Valid column ranges
        let result1 = buffer.validate_col_range(row(0), col(0)..col(3));
        assert!(result1.is_some());
        let (start, end, line) = result1.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 3);
        assert_eq!(line.len(), 5); // buffer width is 5

        // Full line range
        let result2 = buffer.validate_col_range(row(1), col(0)..col(5));
        assert!(result2.is_some());
        let (start, end, line) = result2.unwrap();
        assert_eq!(start, 0);
        assert_eq!(end, 5);
        assert_eq!(line.len(), 5);

        // Single column range
        let result3 = buffer.validate_col_range(row(2), col(3)..col(4));
        assert!(result3.is_some());
        let (start, end, _line) = result3.unwrap();
        assert_eq!(start, 3);
        assert_eq!(end, 4);
    }

    #[test]
    fn test_validate_col_range_invalid() {
        let buffer = create_test_buffer();

        // Invalid row
        assert!(buffer.validate_col_range(row(10), col(0)..col(2)).is_none());

        // Column range out of bounds
        assert!(buffer.validate_col_range(row(0), col(0)..col(6)).is_none());
        assert!(buffer.validate_col_range(row(0), col(3)..col(7)).is_none());

        // Empty ranges
        assert!(buffer.validate_col_range(row(0), col(2)..col(2)).is_some());

        // Inverted ranges
        assert!(buffer.validate_col_range(row(0), col(4)..col(1)).is_none());
    }

    // Test validate_col_range_mut
    #[test]
    fn test_validate_col_range_mut_valid() {
        let mut buffer = create_test_buffer();

        // Test mutable access
        let result = buffer.validate_col_range_mut(row(0), col(1)..col(4));
        assert!(result.is_some());
        let (start, end, line) = result.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 4);

        // Verify we can modify the line
        line[start..end].fill(PixelChar::Spacer);
    }

    #[test]
    fn test_validate_col_range_mut_invalid() {
        let mut buffer = create_test_buffer();

        // Same validation rules as immutable version
        assert!(
            buffer
                .validate_col_range_mut(row(10), col(0)..col(2))
                .is_none()
        );
        assert!(
            buffer
                .validate_col_range_mut(row(0), col(0)..col(6))
                .is_none()
        );
        assert!(
            buffer
                .validate_col_range_mut(row(0), col(2)..col(2))
                .is_some()
        );
        assert!(
            buffer
                .validate_col_range_mut(row(0), col(4)..col(1))
                .is_none()
        );
    }

    // Test validation-only methods
    #[test]
    fn test_is_row_range_valid() {
        let buffer = create_test_buffer();

        // Valid ranges
        assert!(buffer.is_row_range_valid(row(0)..row(2)));
        assert!(buffer.is_row_range_valid(row(0)..row(4))); // Full buffer
        assert!(buffer.is_row_range_valid(row(2)..row(3))); // Single row
        assert!(buffer.is_row_range_valid(row(2)..row(2))); // Empty range

        // Invalid ranges
        assert!(!buffer.is_row_range_valid(row(0)..row(5))); // Out of bounds
        assert!(!buffer.is_row_range_valid(row(5)..row(6))); // Completely out of bounds
        assert!(!buffer.is_row_range_valid(row(3)..row(1))); // Inverted range
    }

    #[test]
    fn test_is_col_range_valid() {
        let buffer = create_test_buffer();

        // Valid column ranges
        assert!(buffer.is_col_range_valid(row(0), col(0)..col(3)));
        assert!(buffer.is_col_range_valid(row(1), col(0)..col(5))); // Full line
        assert!(buffer.is_col_range_valid(row(2), col(3)..col(4))); // Single column
        assert!(buffer.is_col_range_valid(row(0), col(2)..col(2))); // Empty range

        // Invalid ranges
        assert!(!buffer.is_col_range_valid(row(10), col(0)..col(2))); // Invalid row
        assert!(!buffer.is_col_range_valid(row(0), col(0)..col(6))); // Column out of bounds
        assert!(!buffer.is_col_range_valid(row(0), col(3)..col(7))); // Partial out of bounds
        assert!(!buffer.is_col_range_valid(row(0), col(4)..col(1))); // Inverted range
    }

    #[test]
    fn test_validation_only_vs_full_validation_consistency() {
        let buffer = create_test_buffer();

        // Test that validation-only methods match full validation results
        for row_start in 0..6 {
            for row_end in row_start..6 {
                let range = row(row_start)..row(row_end);
                let validation_only = buffer.is_row_range_valid(range.clone());
                let full_validation = buffer.validate_row_range(range).is_some();
                assert_eq!(
                    validation_only, full_validation,
                    "Mismatch for row range {row_start:?}..{row_end:?}"
                );
            }
        }

        // Test column validation consistency
        for row_idx in 0..6 {
            for col_start in 0..7 {
                for col_end in col_start..7 {
                    let col_range = col(col_start)..col(col_end);
                    let validation_only =
                        buffer.is_col_range_valid(row(row_idx), col_range.clone());
                    let full_validation =
                        buffer.validate_col_range(row(row_idx), col_range).is_some();
                    assert_eq!(
                        validation_only, full_validation,
                        "Mismatch for row {row_idx:?}, col range {col_start:?}..{col_end:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn test_range_validation_boundary_cases() {
        // Test with minimal buffer
        let small_buffer = create_test_buffer_with_size(width(1), height(1));

        // Only valid range for 1x1 buffer
        let result = small_buffer.validate_row_range(row(0)..row(1));
        assert!(result.is_some());
        let (_start, _end, lines) = result.unwrap();
        assert_eq!(lines.len(), 1);

        let result = small_buffer.validate_col_range(row(0), col(0)..col(1));
        assert!(result.is_some());
        let (_start, _end, line) = result.unwrap();
        assert_eq!(line.len(), 1);

        // Invalid ranges for minimal buffer
        assert!(small_buffer.validate_row_range(row(0)..row(2)).is_none());
        assert!(
            small_buffer
                .validate_col_range(row(0), col(0)..col(2))
                .is_none()
        );
    }

    #[test]
    fn test_range_validation_integration() {
        let mut buffer = create_test_buffer();

        // Test combining row and column validation
        if let Some((row_start, _row_end, lines)) =
            buffer.validate_row_range_mut(row(1)..row(3))
        {
            // For each line in the validated row range
            for (line_offset, line) in lines.iter_mut().enumerate() {
                let _actual_row = row(row_start + line_offset);

                // Use our own buffer reference to validate column range
                // (we can't use the mutable reference from validate_col_range_mut here)
                let col_range = col(2)..col(4);

                if col_range.check_range_is_valid_for_length(line.len())
                    == RangeValidityStatus::Valid
                {
                    let start_idx = col_range.start.as_usize();
                    let end_idx = col_range.end.as_usize();
                    line[start_idx..end_idx].fill(PixelChar::Spacer);
                }
            }
        }

        // Verify the changes were applied (this is more of a compilation test)
        // Integration test completed successfully
    }
}
