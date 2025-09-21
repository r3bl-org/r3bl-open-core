// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Range validation utilities for type-safe bounds checking with exclusive end semantics.
//!
//! This module provides the [`RangeValidation`] trait for validating Rust [`Range`]
//! objects against buffer bounds, handling the special case of exclusive range ends
//! correctly.
//!
//! This is particularly important for terminal operations where ranges often extend
//! to the edge of the screen buffer.

use std::ops::Range;

use super::{bounds_check_core::BoundsCheck,
            length_and_index_markers::{IndexMarker, LengthMarker},
            result_enums::{BoundsOverflowStatus, ContentPositionStatus}};

/// Range validation trait for type-safe bounds checking with exclusive end semantics.
///
/// ## Why this trait exists
///
/// Rust's `Range<T>` uses exclusive end semantics, meaning the end value is NOT included
/// in the range. This creates a special case when validating ranges against buffer
/// bounds: a range like `0..10` is valid for a buffer of length 10, even though index 10
/// itself would be out of bounds.
///
/// ## The Problem
///
/// Consider a buffer with 10 columns (indices 0-9):
/// ```text
///           ╭────── buffer.len()=10 ────────────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///           ╰─────────── valid indices ──────────╯
/// ```
///
/// When we want to clear columns 5-9 (inclusive),
/// we use Range `5..10` (end exclusive):
/// ```text
///           ╭────── buffer.len()=10 ────────────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ X │ X │ X │ X │ X │ ! │
///         └───┴───┴───┴───┴───┴─▲─┴───┴───┴───┴─▲─┴───┘
///                               ╰─── Range 5..10 ───╯
///                                start=5        end=10 (exclusive)
/// ```
///
/// The end index 10 equals `buffer.len()`, which would normally fail bounds checking,
/// but is valid for an "exclusive range end".
///
/// ## ANSI Terminal Operations Example
///
/// Many ANSI operations work with ranges that go up to the screen edge:
/// ```text
/// ANSI ECH (Erase Characters) with count=5 at cursor position 5:
///           ╭────── screen width=10 ────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
/// Before: │ H │ e │ l │ l │ o │ W │ o │ r │ l │ d │ ! │
///         └───┴───┴───┴───┴───┴─▲─┴───┴───┴───┴───┴───┘
///                               ╰ cursor at col 5
///
/// Range to erase: 5..10 (erase from cursor to end of line)
///                 ╰─────────── 5 characters ────────╯
///
/// After:  │ H │ e │ l │ l │ o │   │   │   │   │   │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
/// ```
///
/// Without this trait, we'd need special-case handling everywhere:
/// ```rust,ignore
/// // Without RangeValidation trait (error-prone):
/// if range.start < range.end
///     && range.start < buffer.len()
///     && range.end <= buffer.len()  // Note the <= for end!
/// { /* logic here */ }
///
/// // With RangeValidation trait (clean and correct):
/// if range.is_valid_for(buffer_length) { /* logic here */ }
/// ```
///
/// ## Usage Examples
///
/// ```rust
/// use std::ops::Range;
/// use r3bl_tui::{ColIndex, ColWidth, RangeValidation};
///
/// let buffer_width = ColWidth::new(10);  // Buffer has 10 columns (0-9)
///
/// // Valid ranges:
/// let range1: Range<ColIndex> = ColIndex::new(0)..ColIndex::new(5);   // columns 0-4
/// let range2: Range<ColIndex> = ColIndex::new(5)..ColIndex::new(10);  // columns 5-9 (end=10 is valid!)
/// let range3: Range<ColIndex> = ColIndex::new(0)..ColIndex::new(10);  // entire buffer
///
/// assert!(range1.is_valid_for(buffer_width));
/// assert!(range2.is_valid_for(buffer_width));  // end=10 <= len=10 ✓
/// assert!(range3.is_valid_for(buffer_width));
///
/// // Invalid ranges:
/// let bad_range1: Range<ColIndex> = ColIndex::new(5)..ColIndex::new(11);  // end > len
/// let bad_range2: Range<ColIndex> = ColIndex::new(8)..ColIndex::new(3);   // start >= end
///
/// assert!(!bad_range1.is_valid_for(buffer_width));
/// assert!(!bad_range2.is_valid_for(buffer_width));
/// ```
pub trait RangeValidation {
    /// The length type that this range can be validated against.
    type LengthType: LengthMarker;

    /// Validates that this range is valid for a buffer of the given length.
    ///
    /// Returns `true` if:
    /// - The range is not empty (start < end)
    /// - The start index is within bounds (< length)
    /// - The end index is valid for exclusive ranges (<= length)
    ///
    /// # Arguments
    ///
    /// * `length` - The buffer length to validate against
    ///
    /// # Returns
    ///
    /// `true` if the range is valid, `false` otherwise.
    fn is_valid_for(&self, length: Self::LengthType) -> bool;
}

/// Implementation of range validation for `Range<IndexType>`.
///
/// This provides type-safe validation for ranges of any index type (`ColIndex`,
/// `RowIndex`, etc.) against their corresponding length types.
impl<I> RangeValidation for Range<I>
where
    I: IndexMarker + PartialOrd + Copy,
    I::LengthType: Copy,
{
    type LengthType = I::LengthType;

    fn is_valid_for(&self, length: I::LengthType) -> bool {
        // Range must not be empty.
        if self.start >= self.end {
            return false;
        }

        // Start must be within bounds (standard index check).
        if self.start.check_overflows(length) != BoundsOverflowStatus::Within {
            return false;
        }

        // End can be equal to length for exclusive ranges (special case).
        // Use ContentPositionStatus to handle this correctly.
        !matches!(
            self.end.check_content_position(length),
            ContentPositionStatus::Beyond
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, RowIndex, col, height, row, width};

    #[test]
    fn test_range_validation_valid_ranges() {
        let buffer_width = width(10); // Columns 0-9
        let buffer_height = height(5); // Rows 0-4

        // Valid column ranges.
        let col_range1: Range<ColIndex> = col(0)..col(5); // columns 0-4
        let col_range2: Range<ColIndex> = col(5)..col(10); // columns 5-9 (end=10 is valid!)
        let col_range3: Range<ColIndex> = col(0)..col(10); // entire buffer
        let col_range4: Range<ColIndex> = col(7)..col(9); // middle range

        assert!(
            col_range1.is_valid_for(buffer_width),
            "Range 0..5 should be valid for width 10"
        );
        assert!(
            col_range2.is_valid_for(buffer_width),
            "Range 5..10 should be valid for width 10 (exclusive end)"
        );
        assert!(
            col_range3.is_valid_for(buffer_width),
            "Range 0..10 should be valid for width 10 (full buffer)"
        );
        assert!(
            col_range4.is_valid_for(buffer_width),
            "Range 7..9 should be valid for width 10"
        );

        // Valid row ranges.
        let row_range1: Range<RowIndex> = row(0)..row(3); // rows 0-2
        let row_range2: Range<RowIndex> = row(2)..row(5); // rows 2-4 (end=5 is valid!)
        let row_range3: Range<RowIndex> = row(0)..row(5); // entire buffer

        assert!(
            row_range1.is_valid_for(buffer_height),
            "Range 0..3 should be valid for height 5"
        );
        assert!(
            row_range2.is_valid_for(buffer_height),
            "Range 2..5 should be valid for height 5 (exclusive end)"
        );
        assert!(
            row_range3.is_valid_for(buffer_height),
            "Range 0..5 should be valid for height 5 (full buffer)"
        );
    }

    #[test]
    fn test_range_validation_invalid_ranges() {
        let buffer_width = width(10); // Columns 0-9
        let buffer_height = height(5); // Rows 0-4

        // Invalid column ranges - end beyond buffer.
        let bad_col_range1: Range<ColIndex> = col(5)..col(11); // end > len
        let bad_col_range2: Range<ColIndex> = col(0)..col(11); // end > len
        let bad_col_range3: Range<ColIndex> = col(15)..col(20); // start beyond buffer

        assert!(
            !bad_col_range1.is_valid_for(buffer_width),
            "Range 5..11 should be invalid for width 10"
        );
        assert!(
            !bad_col_range2.is_valid_for(buffer_width),
            "Range 0..11 should be invalid for width 10"
        );
        assert!(
            !bad_col_range3.is_valid_for(buffer_width),
            "Range 15..20 should be invalid for width 10"
        );

        // Invalid ranges - empty/inverted.
        let empty_col_range1: Range<ColIndex> = col(5)..col(5); // empty range
        let empty_col_range2: Range<ColIndex> = col(8)..col(3); // inverted range

        assert!(
            !empty_col_range1.is_valid_for(buffer_width),
            "Empty range 5..5 should be invalid"
        );
        assert!(
            !empty_col_range2.is_valid_for(buffer_width),
            "Inverted range 8..3 should be invalid"
        );

        // Invalid row ranges.
        let bad_row_range1: Range<RowIndex> = row(3)..row(6); // end > len
        let bad_row_range2: Range<RowIndex> = row(10)..row(12); // start beyond buffer
        let empty_row_range: Range<RowIndex> = row(2)..row(2); // empty range

        assert!(
            !bad_row_range1.is_valid_for(buffer_height),
            "Range 3..6 should be invalid for height 5"
        );
        assert!(
            !bad_row_range2.is_valid_for(buffer_height),
            "Range 10..12 should be invalid for height 5"
        );
        assert!(
            !empty_row_range.is_valid_for(buffer_height),
            "Empty range 2..2 should be invalid"
        );
    }

    #[test]
    fn test_range_validation_boundary_cases() {
        // Edge case: empty buffer.
        let empty_width = width(0);
        let empty_height = height(0);

        let any_range_col: Range<ColIndex> = col(0)..col(1);
        let any_range_row: Range<RowIndex> = row(0)..row(1);

        assert!(
            !any_range_col.is_valid_for(empty_width),
            "Any range should be invalid for empty buffer"
        );
        assert!(
            !any_range_row.is_valid_for(empty_height),
            "Any range should be invalid for empty buffer"
        );

        // Edge case: single element buffer.
        let tiny_width = width(1); // Only column 0
        let tiny_height = height(1); // Only row 0

        let valid_tiny_col: Range<ColIndex> = col(0)..col(1); // only valid range for width 1
        let valid_tiny_row: Range<RowIndex> = row(0)..row(1); // only valid range for height 1
        let invalid_tiny_col: Range<ColIndex> = col(0)..col(2); // end beyond buffer
        let invalid_tiny_row: Range<RowIndex> = row(0)..row(2); // end beyond buffer

        assert!(
            valid_tiny_col.is_valid_for(tiny_width),
            "Range 0..1 should be valid for width 1"
        );
        assert!(
            valid_tiny_row.is_valid_for(tiny_height),
            "Range 0..1 should be valid for height 1"
        );
        assert!(
            !invalid_tiny_col.is_valid_for(tiny_width),
            "Range 0..2 should be invalid for width 1"
        );
        assert!(
            !invalid_tiny_row.is_valid_for(tiny_height),
            "Range 0..2 should be invalid for height 1"
        );
    }
}
