// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::{cmp::min, ops::Sub};

use super::{index_marker::IndexMarker, result_enums::ArrayOverflowResult,
            unit_marker::UnitMarker};
use crate::{ArrayBoundsCheck, Length, len};

/// If you're working with sizes/counts, this trait has what you need.
///
/// This trait identifies types that represent sizes or lengths of content,
/// such as [`RowHeight`], [`ColWidth`], [`ByteLength`], and [`Length`]. These are
/// 1-based values where a length of 1 means "one unit of size".
///
/// Length types represent the "how many" or "how big" aspect of data structures:
/// - How many items in a container
/// - How wide is a terminal window
/// - How tall is a text buffer
/// - How much space remains
///
/// Each length type has a corresponding index type via [`IndexType`](Self::IndexType),
/// enabling safe bounds checking operations in both directions.
///
/// # Core Operations
///
/// - [`convert_to_index()`] - Convert 1-based length to 0-based index (length - 1). Use
///   when finding the last valid position in a container.
///
/// - [`is_overflowed_by()`] - "Does this length get overflowed by this index?" Same check
///   as [`index.overflows(length)`] but from the container's perspective.
///
/// - [`remaining_from()`] - Calculate remaining space from a position. Essential for
///   rendering, text wrapping, and buffer management.
///
/// - [`clamp_to_max()`] - Ensure length doesn't exceed maximum bounds. Use for
///   constraining operations to available space.
///
/// # Design Philosophy
///
/// This trait embodies "length-centric thinking" - operations that naturally arise when
/// working with container sizes and measurements. While indices ask "where am I?",
/// lengths ask "how much space do I have?"
///
/// Length operations complement index operations, providing two perspectives on the same
/// bounds checking logic:
/// - Index-centric: "Does my position overflow this container?"
/// - Length-centric: "Does this container get overflowed by that position?"
///
/// Both perspectives are useful in different contexts and provide natural ways to express
/// bounds checking.
///
/// # Type System Foundation
///
/// See the [Module documentation] for details on how length types relate to index types
/// and the type safety guarantees.
///
/// # See Also
///
/// - [`IndexMarker`] - Position-based operations and comparisons
/// - [`ArrayBoundsCheck`] - Array access safety using length constraints
/// - [`CursorBoundsCheck`] - Cursor positioning using length constraints
///
/// [`RowHeight`]: crate::RowHeight
/// [`ColWidth`]: crate::ColWidth
/// [`ByteLength`]: crate::ByteLength
/// [`Length`]: crate::Length
/// [`IndexMarker`]: crate::IndexMarker
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`convert_to_index()`]: LengthMarker::convert_to_index
/// [`is_overflowed_by()`]: LengthMarker::is_overflowed_by
/// [`remaining_from()`]: LengthMarker::remaining_from
/// [`clamp_to_max()`]: LengthMarker::clamp_to_max
/// [`index.overflows(length)`]: crate::ArrayBoundsCheck::overflows
/// [Module documentation]: mod@crate::core::units::bounds_check
pub trait LengthMarker: UnitMarker {
    /// The corresponding index type for this length type.
    ///
    /// The constraint `IndexMarker<LengthType = Self>` creates a bidirectional
    /// relationship: this ensures that the index type's `LengthType` points back to
    /// this same length type, preventing type mismatches like [`ColWidth`] ↔
    /// [`RowIndex`].
    ///
    /// [`ColWidth`]: crate::ColWidth
    /// [`RowIndex`]: crate::RowIndex
    type IndexType: IndexMarker<LengthType = Self>;

    /// Convert this length to the corresponding index type.
    ///
    /// This typically involves subtracting 1 from the length value since
    /// lengths are 1-based and indices are 0-based.
    ///
    /// ```text
    /// Length=10 to index conversion:
    ///           ┌────────── length=10 (1-based) ────────┐
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │   │   │   │   │ X │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based)                                       ↑
    ///                                         convert_to_index() = 9
    /// ```
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, len, idx};
    ///
    /// // Standard conversion: length=10 → index=9
    /// let length = len(10);
    /// assert_eq!(length.convert_to_index(), idx(9));
    ///
    /// // Edge case: length=1 → index=0
    /// let unit_length = len(1);
    /// assert_eq!(unit_length.convert_to_index(), idx(0));
    ///
    /// // Edge case: length=0 saturates to index=0
    /// let zero_length = len(0);
    /// assert_eq!(zero_length.convert_to_index(), idx(0));
    /// ```
    #[must_use]
    fn convert_to_index(&self) -> Self::IndexType {
        let value = self.as_usize().saturating_sub(1);
        Self::IndexType::from(value)
    }

    /// Answers the question: "Does this length get overflowed by this index?"
    ///
    /// Check if the given index would overflow this length's bounds.
    ///
    /// Example - Checking overflow for length=10
    ///
    /// ```text
    ///                                             boundary
    ///                                                 │
    /// Index:    0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///         │ W │ W │ W │ W │ W │ W │ W │ W │ W │ W │ O │ O │ O │
    ///         ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///         ├─────────── valid indices ─────────────┼─ overflow ┘
    ///         └────────── length=10 (1-based) ────────┘
    ///
    /// W: within bounds (valid access)
    /// O: overflowed (invalid access)
    ///
    /// is_overflowed_by(5)  = Within
    /// is_overflowed_by(9)  = Within (last valid index)
    /// is_overflowed_by(10) = Overflowed (at boundary)
    /// is_overflowed_by(11) = Overflowed (beyond boundary)
    /// ```
    ///
    /// # Returns
    /// - [`ArrayOverflowResult::Within`] if the index is valid
    /// - [`ArrayOverflowResult::Overflowed`] if the index would exceed bounds
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, ArrayOverflowResult, col, row, width};
    ///
    /// let max_col = width(10);
    /// assert_eq!(
    ///     max_col.is_overflowed_by(col(5)),
    ///     ArrayOverflowResult::Within
    /// );
    /// assert_eq!(
    ///     max_col.is_overflowed_by(col(10)),
    ///     ArrayOverflowResult::Overflowed
    /// );
    /// assert_eq!(
    ///     max_col.is_overflowed_by(col(15)),
    ///     ArrayOverflowResult::Overflowed
    /// );
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert_eq!(
    ///     max_col.is_overflowed_by(row(0) + col(5)),
    ///     ArrayOverflowResult::Within
    /// );
    /// assert_eq!(
    ///     max_col.is_overflowed_by(row(2) + col(10)),
    ///     ArrayOverflowResult::Overflowed
    /// );
    /// ```
    #[must_use]
    fn is_overflowed_by(
        &self,
        arg_index: impl Into<Self::IndexType>,
    ) -> ArrayOverflowResult
    where
        Self::IndexType: ArrayBoundsCheck<Self>,
    {
        let index: Self::IndexType = arg_index.into();
        // Delegate to overflows() for single source of truth
        index.overflows(*self)
    }

    /// Calculate the remaining space **from** "the given index" **to** "the end of this
    /// length".
    ///
    /// ```text
    /// With max_width=10:
    ///
    ///                 index=3 (0-based)
    ///                       ↓
    /// Column:   0   1   2   3   4   5   6   7   8   9
    ///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///         │   │   │   │ ✗ │ ✗ │ ✗ │ ✗ │ ✗ │ ✗ │ ✗ │
    ///         ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///         │           └───── 7 chars remain ──────┤
    ///         └────────── width=10 (1-based) ─────────┘
    ///
    /// remaining_from(3)  = 7 (chars from index 3 to 9)
    /// remaining_from(9)  = 1 (only position 9 remains)
    /// remaining_from(10) = 0 (at boundary, nothing remains)
    /// ```
    ///
    /// # Returns
    /// The number of units between the index and the boundary defined by this
    /// length. For example, if this is a [`ColWidth`] of 10 and the index is at column 3,
    /// this returns a [`Length`] of 7 (columns 3-9, inclusive).
    ///
    /// Returns `Length(0)` if the index is at or beyond the boundary.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, col, row, width, len};
    ///
    /// let max_width = width(10);
    /// assert_eq!(max_width.remaining_from(col(3)), len(7));  // 7 columns remain
    /// assert_eq!(max_width.remaining_from(col(10)), len(0)); // At boundary
    /// assert_eq!(max_width.remaining_from(col(15)), len(0)); // Beyond boundary
    ///
    /// // Pos (row + col) automatically converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(0) + col(3)), len(7));  // Pos converts to ColIndex
    /// assert_eq!(max_width.remaining_from(row(1) + col(10)), len(0)); // Pos at boundary
    /// ```
    ///
    /// [`ColWidth`]: crate::ColWidth
    /// [`Length`]: crate::Length
    #[must_use]
    fn remaining_from(&self, arg_index: impl Into<Self::IndexType>) -> Length
    where
        Self::IndexType: Sub<Output = Self::IndexType> + ArrayBoundsCheck<Self>,
        <Self::IndexType as IndexMarker>::LengthType: Into<Length>,
    {
        let index: Self::IndexType = arg_index.into();
        if self.is_overflowed_by(index) == ArrayOverflowResult::Overflowed {
            len(0)
        } else {
            // Get max index for this length.
            let max_index = self.convert_to_index();
            // Calculate num of chars from cursor to boundary (as index difference).
            let chars_remaining_as_index = max_index - index;
            // Convert from 0-based index difference to 1-based length.
            chars_remaining_as_index.convert_to_length().into()
        }
    }

    /// Clamps this length to a maximum value.
    ///
    /// ```text
    /// Clamping operation with max_length=7:
    ///
    /// Case 1: length=5 (within bounds)
    /// ┌───── length=5 ────┬───────┐
    /// │ 1   2   3   4   5 │ 6   7 ← max_length boundary
    /// ├───┬───┬───┬───┬───┼───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │   │   │
    /// └───┴───┴───┴───┴───┴───┴───┘
    ///
    /// Result: clamp_to_max_length(5, max=7) = 5 (no change - within bounds)
    ///
    /// Case 2: length=10 (exceeds bounds)
    /// ┌───────────── length=10 ───┬───────────┐
    /// │ 1   2   3   4   5   6   7 │ 8   9   10 (trimmed)
    /// ├───┬───┬───┬───┬───┬───┬───┼───┬───┬───┤
    /// │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✓ │ ✗ │ ✗ │ ✗ │
    /// └───┴───┴───┴───┴───┴───┴───┼───┴───┴───┘
    ///                             └─ max_length=7 boundary
    ///
    /// Result: clamp_to_max_length(10, max=7) = 7 (clamped to maximum)
    /// ```
    ///
    /// # Returns
    /// The smaller of this length or the maximum length provided. This is commonly used
    /// when constraining operations to available space or buffer boundaries.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthMarker, len};
    ///
    /// // Length within bounds - no change
    /// let small_length = len(5);
    /// let max_allowed = len(10);
    /// assert_eq!(small_length.clamp_to_max(max_allowed), len(5));
    ///
    /// // Length exceeds bounds - gets clamped
    /// let large_length = len(15);
    /// let max_allowed = len(10);
    /// assert_eq!(large_length.clamp_to_max(max_allowed), len(10));
    ///
    /// // Equal lengths - returns the same value
    /// let equal_length = len(8);
    /// let max_allowed = len(8);
    /// assert_eq!(equal_length.clamp_to_max(max_allowed), len(8));
    /// ```
    #[must_use]
    fn clamp_to_max(&self, arg_max_length: impl Into<Self>) -> Self {
        let max_length: Self = arg_max_length.into();
        min(*self, max_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idx;

    #[test]
    fn test_convert_to_index() {
        let length = len(10);
        let index = length.convert_to_index();
        assert_eq!(index, idx(9));

        // Edge case: length of 1
        let small_length = len(1);
        let small_index = small_length.convert_to_index();
        assert_eq!(small_index, idx(0));

        // Edge case: length of 0 (should saturate to 0)
        let zero_length = len(0);
        let zero_index = zero_length.convert_to_index();
        assert_eq!(zero_index, idx(0));
    }

    #[test]
    fn test_is_overflowed_by() {
        let length = len(10);

        // Within bounds
        let within_index = idx(5);
        assert_eq!(
            length.is_overflowed_by(within_index),
            ArrayOverflowResult::Within
        );

        // At boundary (last valid index)
        let boundary_index = idx(9);
        assert_eq!(
            length.is_overflowed_by(boundary_index),
            ArrayOverflowResult::Within
        );

        // Overflow cases
        let overflow_index = idx(10);
        assert_eq!(
            length.is_overflowed_by(overflow_index),
            ArrayOverflowResult::Overflowed
        );

        let large_overflow_index = idx(15);
        assert_eq!(
            length.is_overflowed_by(large_overflow_index),
            ArrayOverflowResult::Overflowed
        );
    }

    #[test]
    fn test_remaining_from() {
        let length = len(10);

        // Index in the middle
        let middle_index = idx(3);
        let remaining = length.remaining_from(middle_index);
        assert_eq!(remaining.as_usize(), 7); // positions 3,4,5,6,7,8,9 = 7 positions

        // Index at the beginning
        let start_index = idx(0);
        let remaining_from_start = length.remaining_from(start_index);
        assert_eq!(remaining_from_start.as_usize(), 10); // All positions remain

        // Index at the last valid position
        let last_index = idx(9);
        let remaining_from_last = length.remaining_from(last_index);
        assert_eq!(remaining_from_last.as_usize(), 1); // Only position 9 remains

        // Index at boundary (overflow case)
        let boundary_index = idx(10);
        let remaining_from_boundary = length.remaining_from(boundary_index);
        assert_eq!(remaining_from_boundary.as_usize(), 0); // Nothing remains

        // Index beyond boundary
        let beyond_index = idx(15);
        let remaining_from_beyond = length.remaining_from(beyond_index);
        assert_eq!(remaining_from_beyond.as_usize(), 0); // Nothing remains
    }

    #[test]
    fn test_clamp_to_max() {
        let max_length = len(10);

        // Length within bounds
        let small_length = len(5);
        assert_eq!(small_length.clamp_to_max(max_length), small_length);

        // Length at bounds
        let equal_length = len(10);
        assert_eq!(equal_length.clamp_to_max(max_length), max_length);

        // Length exceeding bounds
        let large_length = len(15);
        assert_eq!(large_length.clamp_to_max(max_length), max_length);

        // Zero length
        let zero_length = len(0);
        assert_eq!(zero_length.clamp_to_max(max_length), zero_length);
    }

    #[test]
    fn test_edge_cases_zero_length() {
        let zero_length = len(0);

        // Any index should overflow zero length
        let zero_index = idx(0);
        assert_eq!(
            zero_length.is_overflowed_by(zero_index),
            ArrayOverflowResult::Overflowed
        );

        let small_index = idx(1);
        assert_eq!(
            zero_length.is_overflowed_by(small_index),
            ArrayOverflowResult::Overflowed
        );

        // Remaining from zero length should always be zero
        let remaining = zero_length.remaining_from(zero_index);
        assert_eq!(remaining.as_usize(), 0);
    }

    #[test]
    fn test_edge_cases_unit_length() {
        let unit_length = len(1);

        // Only index 0 should be valid
        let zero_index = idx(0);
        assert_eq!(
            unit_length.is_overflowed_by(zero_index),
            ArrayOverflowResult::Within
        );

        let one_index = idx(1);
        assert_eq!(
            unit_length.is_overflowed_by(one_index),
            ArrayOverflowResult::Overflowed
        );

        // Remaining from valid position
        let remaining_from_zero = unit_length.remaining_from(zero_index);
        assert_eq!(remaining_from_zero.as_usize(), 1); // One position remains

        // Remaining from overflow position
        let remaining_from_one = unit_length.remaining_from(one_index);
        assert_eq!(remaining_from_one.as_usize(), 0); // Nothing remains
    }
}
