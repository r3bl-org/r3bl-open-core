// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! End-of-line cursor positioning utilities for text editing and range operations.
//!
//! This module provides unified traits for handling cursor positioning semantics where
//! cursors can be placed at the end-of-line position (index == length). This is essential
//! for:
//! - Cursor positioning after the last character in text editing
//! - Range operations with exclusive end semantics (e.g., 0..10 for buffer length 10)
//! - Text editor interactions where cursors naturally sit after content
//!
//! The module provides the [`EOLCursorPosition`] trait for end-of-line cursor positioning
//! and the [`RangeBoundary`] trait for range validation with text editing semantics.

use std::ops::Range;

use super::{array_bounds::BoundsCheck,
            length_and_index_markers::{IndexMarker, LengthMarker},
            result_enums::{ArrayAccessBoundsStatus, CursorPositionBoundsStatus}};
use crate::ArrayAccessBoundsStatus::Overflowed;

/// Trait for determining end-of-line cursor positioning in text editing contexts.
///
/// This trait provides the capability to determine where a cursor can be positioned
/// at the end of a line or buffer. It handles the special case where a cursor position
/// can equal the content length, representing the position "after" the last character.
/// This is essential for:
/// - Cursor positioning after the last character in text editing
/// - Range operations with exclusive end semantics
/// - Natural text editor interactions where cursors sit after content
///
/// # Semantic Meaning
///
/// For content of length N, valid indices are 0..N-1 for content access, but
/// position N is valid for cursor placement and range boundaries:
///
/// ```text
///           ╭── length=5 ───╮
/// Index:    0   1   2   3   4   5
///         ┌───┬───┬───┬───┬───┬───┐
/// Content:│ h │ e │ l │ l │ o │ ! │
///         └───┴───┴───┴───┴───┴───┘
///           ╰─valid indices─╯   │
///           ╰───────────────────╯ valid cursor positions
///                               ↑
///                      "after last position"
/// ```
///
/// # Examples
///
/// ```
/// use r3bl_tui::{EOLCursorPosition, ColWidth, col, width};
///
/// let w = width(5);
/// assert_eq!(w.eol_cursor_position(), col(5));
///
/// let zero_w = width(0);
/// assert_eq!(zero_w.eol_cursor_position(), col(0));
/// ```
pub trait EOLCursorPosition: LengthMarker {
    /// Get the cursor position at end-of-line (after the last character).
    ///
    /// This is the position where a cursor can be placed to continue typing,
    /// equivalent to where the cursor sits after pressing End in a text editor.
    /// For content of length N, this returns position N.
    ///
    /// This position is fundamental for text editing operations as it represents
    /// the natural place where new text would be appended to existing content.
    ///
    /// Returns the position where index equals the content length.
    fn eol_cursor_position(&self) -> Self::IndexType;

    /// Check if a cursor position is valid for this line/buffer.
    ///
    /// Returns true for positions in the range [0, length] (inclusive of EOL position).
    /// This allows cursors to be positioned anywhere from the start to after the last
    /// character.
    fn is_valid_cursor_position(&self, pos: Self::IndexType) -> bool;

    /// Clamp a cursor position to valid bounds for this line/buffer.
    ///
    /// Ensures the cursor position is valid for text editing operations.
    /// Positions beyond the EOL are clamped to the EOL position.
    fn clamp_cursor_position(&self, pos: Self::IndexType) -> Self::IndexType;
}

/// Blanket implementation for all types that implement `LengthMarker`.
///
/// This provides consistent EOL cursor positioning for all length types
/// (Length, `ColWidth`, `RowHeight`) without code duplication.
impl<T: LengthMarker> EOLCursorPosition for T
where
    T: Copy,
    T::IndexType: From<usize> + std::ops::Add<Output = T::IndexType> + PartialOrd + Copy,
{
    fn eol_cursor_position(&self) -> Self::IndexType {
        let length_val = self.as_usize();

        if length_val == 0 {
            // Use From<usize> for type-safe construction.
            T::IndexType::from(0)
        } else {
            // Normal case: last valid index + 1.
            self.convert_to_index() + T::IndexType::from(1)
        }
    }

    fn is_valid_cursor_position(&self, pos: Self::IndexType) -> bool {
        // Position is valid if it's not beyond the boundary
        pos.check_cursor_position_bounds(*self) != CursorPositionBoundsStatus::Beyond
    }

    fn clamp_cursor_position(&self, pos: Self::IndexType) -> Self::IndexType {
        if self.is_valid_cursor_position(pos) {
            pos
        } else {
            self.eol_cursor_position()
        }
    }
}

/// Range operations that respect content boundary semantics.
///
/// ## Why content boundary semantics matter for ranges
///
/// Rust's `Range<T>` uses exclusive end semantics, meaning the end value is NOT included
/// in the range. This creates a special case when validating ranges against content
/// bounds: a range like `0..10` is valid for content of length 10, even though index 10
/// itself would be out of bounds for content access.
///
/// ## Example
///
/// Consider content with 10 columns (indices 0-9):
/// ```text
///           ╭────── content.len()=10 ───────────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///           ╰─────────── valid indices ─────────╯
/// ```
///
/// When we want to process columns 5-9 (inclusive), we use Range `5..10`:
/// ```text
///                               ╭─── Range 5..10 ───╮
///                               ▼                   ▼
/// Column:   0   1   2   3   4   5   6   7   8   9   10
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ X │ X │ X │ X │ X │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑                   ↑
///                             start=5            end=10 (exclusive)
/// ```
///
/// The end index 10 equals content length, which is valid for exclusive range semantics.
///
/// # Usage Examples
///
/// ```rust
/// use std::ops::Range;
/// use r3bl_tui::{ColIndex, ColWidth, RangeBoundary};
///
/// let content_width = ColWidth::new(10);  // Content has 10 columns (0-9)
///
/// // Valid ranges:
/// let range1: Range<ColIndex> = ColIndex::new(0)..ColIndex::new(5);   // columns 0-4
/// let range2: Range<ColIndex> = ColIndex::new(5)..ColIndex::new(10);  // columns 5-9 (end=10 is valid!)
/// let range3: Range<ColIndex> = ColIndex::new(0)..ColIndex::new(10);  // entire content
///
/// assert!(range1.is_valid(content_width));
/// assert!(range2.is_valid(content_width));  // end=10 <= len=10 ✓
/// assert!(range3.is_valid(content_width));
///
/// // Invalid ranges:
/// let bad_range1: Range<ColIndex> = ColIndex::new(5)..ColIndex::new(11);  // end > len
/// let bad_range2: Range<ColIndex> = ColIndex::new(8)..ColIndex::new(3);   // start > end
///
/// assert!(!bad_range1.is_valid(content_width));
/// assert!(!bad_range2.is_valid(content_width));
/// ```
pub trait RangeBoundary {
    /// The length type that this range can be validated against.
    type LengthType: EOLCursorPosition;

    /// Check if this range is valid for the given buffer/line length.
    ///
    /// Returns `true` if:
    /// - The range is not inverted (start <= end) - empty ranges are valid
    /// - The start position is within buffer bounds (< length)
    /// - The end position is valid for EOL cursor placement (<= length)
    ///
    /// # Arguments
    ///
    /// * `buffer_length` - The buffer length to validate against
    ///
    /// # Returns
    ///
    /// `true` if the range is valid for buffer operations, `false` otherwise.
    fn is_valid(&self, buffer_length: impl Into<Self::LengthType>) -> bool;

    /// Clamp this range to fit within buffer/line bounds.
    ///
    /// This method ensures that both the start and end of the range are valid
    /// for the given buffer length, while preserving Rust's exclusive end semantics
    /// and EOL cursor positioning rules.
    ///
    /// # Behavior
    ///
    /// - Start is clamped to `[0, length)`
    /// - End is clamped to `[start, length]` (note: end can equal length for exclusive
    ///   ranges and EOL cursor positioning)
    /// - Empty ranges are preserved as empty
    /// - If the original range was invalid, returns a valid empty range at the start
    ///
    /// # Arguments
    ///
    /// * `buffer_length` - The buffer length to clamp against
    ///
    /// # Returns
    ///
    /// A new range that is guaranteed to be valid for the given buffer length.
    #[must_use]
    fn clamp_range_to(self, buffer_length: Self::LengthType) -> Self;
}

/// Implementation of range operations for `Range<IndexType>`.
///
/// This provides type-safe validation and clamping for ranges of any index type
/// (`ColIndex`, `RowIndex`, etc.) against their corresponding length types.
impl<I> RangeBoundary for Range<I>
where
    I: IndexMarker + PartialOrd + Copy + From<usize> + std::ops::Add<Output = I>,
    I::LengthType: Copy,
{
    type LengthType = I::LengthType;

    fn is_valid(&self, buffer_length: impl Into<Self::LengthType>) -> bool {
        let length = buffer_length.into();

        // Check for inverted ranges (start > end).
        if self.start > self.end {
            return false;
        }

        // Start must be within bounds (standard index check).
        if self.start.check_array_access_bounds(length) != ArrayAccessBoundsStatus::Within
        {
            return false;
        }

        // End can be equal to length for exclusive ranges (special case).
        // Use CursorPositionBoundsStatus to handle this correctly.
        self.end.check_cursor_position_bounds(length)
            != CursorPositionBoundsStatus::Beyond
    }

    fn clamp_range_to(self, buffer_length: Self::LengthType) -> Range<I> {
        // If start is beyond bounds, return empty range at start.
        if self.start.check_array_access_bounds(buffer_length) == Overflowed {
            let zero = I::LengthType::from(0usize).convert_to_index();
            return zero..zero;
        }

        // Clamp start to valid bounds (already checked it's within bounds above).
        let clamped_start = self.start;

        // For end, we need to handle exclusive range semantics:
        // - End can equal content_length (exclusive ranges allow this)
        // - End beyond content_length should be clamped to content_length
        let clamped_end = if self.end.check_cursor_position_bounds(buffer_length)
            == CursorPositionBoundsStatus::Beyond
        {
            // For exclusive ranges, the end can equal the length (unlike regular index
            // bounds checking). Use EOLCursorPosition to get the position where
            // index == length, which is the valid exclusive range end.
            buffer_length.eol_cursor_position()
        } else {
            self.end
        };

        // Ensure range is not inverted (start >= end).
        if clamped_start >= clamped_end {
            clamped_start..clamped_start // Empty range.
        } else {
            clamped_start..clamped_end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, col, height, idx, len, row,
                width};

    mod eol_cursor_position_tests {
        use super::*;

        #[test]
        fn test_eol_cursor_position_trait() {
            // Test with ColWidth.
            {
                let width_5 = ColWidth::new(5);
                assert_eq!(
                    width_5.eol_cursor_position(),
                    ColIndex::new(5),
                    "Width 5 should give boundary position at col 5"
                );

                let width_0 = ColWidth::new(0);
                assert_eq!(
                    width_0.eol_cursor_position(),
                    ColIndex::new(0),
                    "Zero width should give position 0"
                );

                let width_1 = ColWidth::new(1);
                assert_eq!(
                    width_1.eol_cursor_position(),
                    ColIndex::new(1),
                    "Width 1 should give boundary position at col 1"
                );
            }

            // Test with RowHeight.
            {
                let height_3 = RowHeight::new(3);
                assert_eq!(
                    height_3.eol_cursor_position(),
                    RowIndex::new(3),
                    "Height 3 should give boundary position at row 3"
                );

                let height_0 = RowHeight::new(0);
                assert_eq!(
                    height_0.eol_cursor_position(),
                    RowIndex::new(0),
                    "Zero height should give position 0"
                );
            }

            // Test with generic Length.
            {
                let len_10 = len(10);
                assert_eq!(
                    len_10.eol_cursor_position(),
                    idx(10),
                    "Length 10 should give boundary position at index 10"
                );

                let len_0 = len(0);
                assert_eq!(
                    len_0.eol_cursor_position(),
                    idx(0),
                    "Zero length should give position 0"
                );
            }
        }

        #[test]
        fn test_is_valid_cursor_position_trait() {
            let content_length = len(5);

            // Within boundary
            assert!(content_length.is_valid_cursor_position(idx(0)));
            assert!(content_length.is_valid_cursor_position(idx(3)));
            assert!(content_length.is_valid_cursor_position(idx(5))); // EOL position

            // Beyond boundary
            assert!(!content_length.is_valid_cursor_position(idx(6)));
            assert!(!content_length.is_valid_cursor_position(idx(10)));
        }

        #[test]
        fn test_clamp_cursor_position_trait() {
            let content_length = len(5);

            // Within boundary - no change
            assert_eq!(content_length.clamp_cursor_position(idx(0)), idx(0));
            assert_eq!(content_length.clamp_cursor_position(idx(3)), idx(3));
            assert_eq!(content_length.clamp_cursor_position(idx(5)), idx(5));

            // Beyond boundary - clamp to boundary
            assert_eq!(content_length.clamp_cursor_position(idx(6)), idx(5));
            assert_eq!(content_length.clamp_cursor_position(idx(10)), idx(5));
        }

        #[test]
        fn test_boundary_semantic_equivalence() {
            // Verify the semantic: eol_cursor_position() == convert_to_index()
            // + 1 (for non-zero).
            for i in 1..=10 {
                let w = ColWidth::new(i);
                let expected = w.convert_to_index() + ColIndex::new(1);
                let actual = w.eol_cursor_position();
                assert_eq!(
                    actual, expected,
                    "For width {i}, boundary should be convert_to_index() + 1"
                );
            }

            // Verify zero edge case
            let zero_width = ColWidth::new(0);
            assert_eq!(
                zero_width.eol_cursor_position(),
                ColIndex::new(0),
                "Zero width should give position 0, not -1 or error"
            );
        }
    }

    mod range_boundary_tests {
        use super::*;

        #[test]
        fn test_range_validation_valid_ranges() {
            let content_width = width(10); // Columns 0-9
            let content_height = height(5); // Rows 0-4

            // Valid column ranges.
            let col_range1: Range<ColIndex> = col(0)..col(5); // columns 0-4
            let col_range2: Range<ColIndex> = col(5)..col(10); // columns 5-9 (end=10 is valid!)
            let col_range3: Range<ColIndex> = col(0)..col(10); // entire content
            let col_range4: Range<ColIndex> = col(7)..col(9); // middle range

            assert!(
                col_range1.is_valid(content_width),
                "Range 0..5 should be valid for width 10"
            );
            assert!(
                col_range2.is_valid(content_width),
                "Range 5..10 should be valid for width 10 (exclusive end)"
            );
            assert!(
                col_range3.is_valid(content_width),
                "Range 0..10 should be valid for width 10 (full content)"
            );
            assert!(
                col_range4.is_valid(content_width),
                "Range 7..9 should be valid for width 10"
            );

            // Valid row ranges.
            let row_range1: Range<RowIndex> = row(0)..row(3); // rows 0-2
            let row_range2: Range<RowIndex> = row(2)..row(5); // rows 2-4 (end=5 is valid!)
            let row_range3: Range<RowIndex> = row(0)..row(5); // entire content

            assert!(
                row_range1.is_valid(content_height),
                "Range 0..3 should be valid for height 5"
            );
            assert!(
                row_range2.is_valid(content_height),
                "Range 2..5 should be valid for height 5 (exclusive end)"
            );
            assert!(
                row_range3.is_valid(content_height),
                "Range 0..5 should be valid for height 5 (full content)"
            );
        }

        #[test]
        fn test_range_validation_invalid_ranges() {
            let content_width = width(10); // Columns 0-9

            // Invalid column ranges - end beyond content.
            let bad_col_range1: Range<ColIndex> = col(5)..col(11); // end > len
            let bad_col_range2: Range<ColIndex> = col(0)..col(11); // end > len
            let bad_col_range3: Range<ColIndex> = col(15)..col(20); // start beyond content

            assert!(
                !bad_col_range1.is_valid(content_width),
                "Range 5..11 should be invalid for width 10"
            );
            assert!(
                !bad_col_range2.is_valid(content_width),
                "Range 0..11 should be invalid for width 10"
            );
            assert!(
                !bad_col_range3.is_valid(content_width),
                "Range 15..20 should be invalid for width 10"
            );

            // Invalid ranges - inverted only (empty ranges are valid).
            let empty_col_range1: Range<ColIndex> = col(5)..col(5); // empty range
            let empty_col_range2: Range<ColIndex> = col(8)..col(3); // inverted range

            assert!(
                empty_col_range1.is_valid(content_width),
                "Empty range 5..5 should be valid (within bounds)"
            );
            assert!(
                !empty_col_range2.is_valid(content_width),
                "Inverted range 8..3 should be invalid"
            );
        }

        #[test]
        fn test_range_clamp_to_content_normal_cases() {
            let content_width = width(10); // Columns 0-9, length 10

            // Normal range within bounds - should remain unchanged.
            let col_range1: Range<ColIndex> = col(2)..col(7);
            let clamped1 = col_range1.clamp_range_to(content_width);
            assert_eq!(
                clamped1,
                col(2)..col(7),
                "Normal range should remain unchanged"
            );

            // Range that exactly fits content (0..length) - should remain unchanged.
            let full_col_range: Range<ColIndex> = col(0)..col(10);
            let clamped_full = full_col_range.clamp_range_to(content_width);
            assert_eq!(
                clamped_full,
                col(0)..col(10),
                "Full content range should remain unchanged"
            );

            // Range to end of content (start..length) - should remain unchanged.
            let to_end_range: Range<ColIndex> = col(5)..col(10);
            let clamped_to_end = to_end_range.clamp_range_to(content_width);
            assert_eq!(
                clamped_to_end,
                col(5)..col(10),
                "Range to end should remain unchanged"
            );
        }

        #[test]
        fn test_range_clamp_to_content_end_beyond_bounds() {
            let content_width = width(10); // Columns 0-9, length 10

            // Range with end beyond bounds - should clamp end to content length.
            let col_range1: Range<ColIndex> = col(5)..col(15);
            let clamped1 = col_range1.clamp_range_to(content_width);
            assert_eq!(
                clamped1,
                col(5)..col(10),
                "End should be clamped to content length"
            );

            // Range starting from 0 with end beyond bounds.
            let col_range2: Range<ColIndex> = col(0)..col(15);
            let clamped2 = col_range2.clamp_range_to(content_width);
            assert_eq!(
                clamped2,
                col(0)..col(10),
                "Full range with end beyond should clamp to content"
            );

            // Range with both start and end way beyond bounds.
            let col_range3: Range<ColIndex> = col(20)..col(30);
            let clamped3 = col_range3.clamp_range_to(content_width);
            assert_eq!(
                clamped3,
                col(0)..col(0),
                "Range beyond bounds should become empty at start"
            );
        }

        #[test]
        fn test_range_clamp_to_content_exclusive_end_semantics() {
            let content_width = width(10); // Columns 0-9, length 10

            // Test that exclusive end semantics are preserved.
            // Range 5..10 should remain 5..10 (end == length is valid for exclusive
            // ranges).
            let range_to_end: Range<ColIndex> = col(5)..col(10);
            let clamped_to_end = range_to_end.clamp_range_to(content_width);
            assert_eq!(
                clamped_to_end,
                col(5)..col(10),
                "Range to content end should preserve exclusive end semantics"
            );

            // Range 0..10 should remain 0..10 (full content range).
            let full_range: Range<ColIndex> = col(0)..col(10);
            let clamped_full = full_range.clamp_range_to(content_width);
            assert_eq!(
                clamped_full,
                col(0)..col(10),
                "Full content range should preserve exclusive end semantics"
            );

            // Range 9..10 should remain 9..10 (single element range at end).
            let last_element: Range<ColIndex> = col(9)..col(10);
            let clamped_last = last_element.clamp_range_to(content_width);
            assert_eq!(
                clamped_last,
                col(9)..col(10),
                "Range for last element should preserve exclusive end semantics"
            );
        }
    }
}
