// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::RangeInclusive;

use super::{length_marker::LengthMarker, unit_marker::UnitMarker};
use crate::{ArrayBoundsCheck, ArrayOverflowResult};

/// If you're working with positions/indices, this trait has what you need.
///
/// This trait serves as the foundation for all index operations in the bounds checking
/// system. It identifies types that represent positions or indices within content,
/// such as [`RowIndex`], [`ColIndex`], [`ByteIndex`], and [`Index`]. These are 0-based
/// values where the first position is index 0.
///
/// Higher-level bounds checking traits (in [`array_bounds.rs`], [`cursor_bounds.rs`])
/// build upon the methods defined here. Each index type has a corresponding length
/// type via [`LengthType`](Self::LengthType), enabling safe bounds checking operations
/// in both directions.
///
/// # Core Operations
///
/// - [`convert_to_length()`] - Convert 0-based index to 1-based length
///
/// - [`clamp_to_min_index()`] - Clamp to minimum bound (index-to-index). Use for scroll
///   positions with a minimum starting point. Takes an index because lower bounds are
///   naturally expressed as minimum positions ("What's the lowest position allowed?")
///
/// - [`clamp_to_max_length()`] - Clamp to container bounds (index-to-length). Use for
///   array/buffer access within a container size. Takes a length because upper bounds are
///   naturally expressed as container sizes ("How many items are there?")
///
/// - [`clamp_to_range()`] - Clamp to inclusive range (index-to-range). Use for VT-100
///   scroll regions or text selections where both endpoints are valid positions.
///
/// # Type System Foundation
///
/// See the [Module documentation] for details on how index types relate to length types
/// and the type safety guarantees.
///
/// # See Also
///
/// - [`LengthMarker`] - Operations specific to length/size values
/// - [`ArrayBoundsCheck`] - Array access safety using index-to-length comparisons
/// - [`CursorBoundsCheck`] - Cursor positioning using index-to-length comparisons
/// - [`ViewportBoundsCheck`] - Viewport visibility using hybrid operations
///
/// [`RowIndex`]: crate::RowIndex
/// [`ColIndex`]: crate::ColIndex
/// [`ByteIndex`]: crate::ByteIndex
/// [`Index`]: crate::Index
/// [`LengthMarker`]: crate::LengthMarker
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`array_bounds.rs`]: crate::core::units::bounds_check::array_bounds
/// [`cursor_bounds.rs`]: crate::core::units::bounds_check::cursor_bounds
/// [`ArrayBoundsCheck::overflows()`]: crate::ArrayBoundsCheck::overflows
/// [`ArrayBoundsCheck::underflows()`]: crate::ArrayBoundsCheck::underflows
/// [`convert_to_length()`]: IndexMarker::convert_to_length
/// [`clamp_to_max_length()`]: IndexMarker::clamp_to_max_length
/// [`clamp_to_min_index()`]: IndexMarker::clamp_to_min_index
/// [`clamp_to_range()`]: IndexMarker::clamp_to_range
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: crate::core::units::bounds_check#interval-notation
pub trait IndexMarker: UnitMarker {
    /// The corresponding "length" type for this "index" type.
    ///
    /// The constraint [`LengthMarker<IndexType = Self>`] creates a bidirectional
    /// relationship: this ensures that the length type's [`Self::IndexType`] points back
    /// to this same index type, preventing type mismatches like [`ColIndex`] ↔
    /// [`RowHeight`].
    ///
    /// [`ColIndex`]: crate::ColIndex
    /// [`RowHeight`]: crate::RowHeight
    /// [`LengthMarker<IndexType = Self>`]: crate::LengthMarker
    /// [`Self::IndexType`]: crate::LengthMarker::IndexType
    type LengthType: LengthMarker<IndexType = Self>;

    /// Convert this index to the corresponding length type.
    ///
    /// This typically involves adding 1 to the index value since indices are 0-based and
    /// lengths are 1-based.
    ///
    /// ```text
    /// Index=5 (0-based) to length (1-based) conversion:
    ///
    ///                           index=5 (0-based)
    ///                                 ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///           │   │   │   │   │   │ X │   │   │   │   │
    ///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
    /// Length:     1   2   3   4   5   6   7   8   9   10
    /// (1-based)                       ↑
    ///                  convert_to_length() = 6 (1-based)
    /// ```
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, idx};
    ///
    /// let index = idx(5);
    /// let length = index.convert_to_length();
    /// assert_eq!(length.as_usize(), 6); // 0-based index 5 → 1-based length 6
    ///
    /// // Zero index converts to length 1
    /// assert_eq!(idx(0).convert_to_length().as_usize(), 1);
    /// ```
    fn convert_to_length(&self) -> Self::LengthType {
        Self::LengthType::from(self.as_usize() + 1)
    }

    /// Clamp this index to stay within the bounds defined by a container length.
    ///
    /// - Upper bounds are naturally expressed as container sizes (how many items exist),
    ///   which is why this method takes a `Self::LengthType` parameter.
    /// - This contrasts with [`clamp_to_min_index()`] which takes an index parameter,
    ///   since lower bounds are naturally expressed as minimum positions.
    ///
    /// ```text
    /// Clamping operation with max_length=10:
    ///
    ///                           index=5 (within bounds)     index=15 (overflows)
    ///                                 ↓                                       ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12  13  14  15
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┬───┬───┬───┐
    ///           │ W │ W │ W │ W │ W │ W │ W │ W │ W │ W │ O │ O │ O │ O │ O │ O │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┴───┴───┴───┤
    ///           ├────────── valid indices ──────────────┼──── overflow ─────────┘
    ///           └─────────── length=10 (1-based) ───────┘
    ///
    /// W = Within (index < length)
    /// O = Overflowed (index >= length)
    ///
    /// clamp_to_max_length(index=5, max_length=10)  = 5 (unchanged - within bounds)
    /// clamp_to_max_length(index=15, max_length=10) = 9 (clamped to max valid index)
    /// ```
    ///
    /// # Returns
    /// The index unchanged if it's within bounds, or the maximum valid index
    /// (length - 1) if the index would overflow the bounds.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, width};
    ///
    /// let max_width = width(10);
    ///
    /// // Index within bounds - returned unchanged
    /// assert_eq!(col(5).clamp_to_max_length(max_width), col(5));
    /// // Index at boundary - returned unchanged (9 is valid for width 10)
    /// assert_eq!(col(9).clamp_to_max_length(max_width), col(9));
    ///
    /// // Index overflows - clamped to maximum valid index
    /// assert_eq!(col(15).clamp_to_max_length(max_width), col(9));
    ///
    /// // Zero index - always valid
    /// assert_eq!(col(0).clamp_to_max_length(max_width), col(0));
    /// ```
    ///
    /// [`clamp_to_min_index()`]: IndexMarker::clamp_to_min_index
    #[must_use]
    fn clamp_to_max_length(&self, max_length: Self::LengthType) -> Self
    where
        Self: ArrayBoundsCheck<Self::LengthType>,
    {
        if self.overflows(max_length) == ArrayOverflowResult::Overflowed {
            max_length.convert_to_index()
        } else {
            *self
        }
    }

    /// Ensures this index is at least the given minimum index bound.
    ///
    /// - This is useful for ensuring indices don't go below a starting position, such as
    ///   in scrolling logic.
    /// - Lower bounds are naturally expressed as minimum positions (index-to-index),
    ///   which is why this method takes an index (`Self`) parameter.
    /// - This contrasts with [`clamp_to_max_length()`] which takes a length parameter,
    ///   since upper bounds are naturally expressed as container sizes.
    ///
    /// ```text
    /// Clamping operation with min_bound=3:
    ///
    ///                 min_bound=3
    ///    current index=2   │   current index=7
    ///                  │   │               │
    ///                  ▼   ▼               ▼
    /// Index:   0   1   2   3   4   5   6   7   8   9
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ U │ U │ U │ W │ W │ W │ W │ W │ W │ W │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
    ///        └ underflow ┴──────── valid range ──────┘
    ///
    /// W = Within valid range (index ≥ min_bound)
    /// U = Underflowed (index < min_bound)
    ///
    /// clamp_to_min_index(index=2, min_index=3) = 3 (clamped up to minimum)
    /// clamp_to_min_index(index=7, min_index=3) = 7 (unchanged - above minimum)
    /// ```
    ///
    /// # Returns
    /// The minimum bound if this index is less than it, otherwise returns self unchanged.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{IndexMarker, col, row};
    ///
    /// let min_col = col(3);
    ///
    /// // Index below minimum - clamped up
    /// assert_eq!(col(1).clamp_to_min_index(min_col), col(3));
    ///
    /// // Index above minimum - unchanged
    /// assert_eq!(col(5).clamp_to_min_index(min_col), col(5));
    /// // Index at minimum - unchanged
    /// assert_eq!(col(3).clamp_to_min_index(min_col), col(3));
    ///
    /// // Zero index below minimum - clamped up
    /// assert_eq!(col(0).clamp_to_min_index(min_col), col(3));
    /// ```
    ///
    /// [`clamp_to_max_length()`]: IndexMarker::clamp_to_max_length
    #[must_use]
    fn clamp_to_min_index(&self, min_bound: impl Into<Self>) -> Self {
        let min: Self = min_bound.into();
        (*self).max(min)
    }

    /// Clamp this index to stay within an inclusive range.
    ///
    /// Common use cases include VT-100 scroll regions (rows 2-5 inclusive) and
    /// text selections (columns 10-20 inclusive) where both boundaries are valid
    /// positions.
    ///
    /// > <div class="warning">
    /// >
    /// > See [Interval Notation] in the [Module Documentation] for mathematical range
    /// > notation used in the diagrams. This method operates on inclusive ranges:
    /// > `[min..=max]`.
    /// >
    /// > </div>
    ///
    /// ```text
    /// Clamping to inclusive range [3..=7]:
    ///
    ///                    min=3           max=7
    ///                      ↓               ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9   10
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ U │ U │ U │ W │ W │ W │ W │ W │ O │ O │ O │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┼───┴───┴───┤
    ///        └ underflow ┴── valid range ───┴─ overflow ┘
    ///
    /// W = Within (min ≤ index ≤ max)
    /// U = Underflowed (index < min)
    /// O = Overflowed (index > max)
    ///
    /// clamp_to_range(index=1, range=3..=7) = 3 (clamped to start)
    /// clamp_to_range(index=5, range=3..=7) = 5 (unchanged)
    /// clamp_to_range(index=9, range=3..=7) = 7 (clamped to end)
    /// ```
    ///
    /// # Returns
    ///
    /// - The index unchanged if within range (inclusive)
    /// - The range start if below range
    /// - The range end if above range
    ///
    /// # Examples
    ///
    /// ```rust
    /// use r3bl_tui::{IndexMarker, row, col};
    /// use std::ops::RangeInclusive;
    ///
    /// // VT-100 scroll region clamping
    /// let scroll_region = row(2)..=row(5);
    /// assert_eq!(row(8).clamp_to_range(scroll_region), row(5));
    ///
    /// // Text selection bounds
    /// let selection = col(10)..=col(20);
    /// assert_eq!(col(25).clamp_to_range(selection), col(20));
    ///
    /// // Basic clamping
    /// let range = col(3)..=col(7);
    /// assert_eq!(col(1).clamp_to_range(range.clone()), col(3));  // below
    /// assert_eq!(col(5).clamp_to_range(range.clone()), col(5));  // within
    /// assert_eq!(col(9).clamp_to_range(range), col(7));          // above
    /// ```
    ///
    /// # Comparison with Standard Library `clamp()`
    ///
    /// This method is semantically equivalent to `clamp()`, and the only difference is
    /// intent and ergonomics:
    /// - `clamp_to_range(range)` - Domain-specific, works with [`RangeInclusive`] types
    /// - [`clamp(min, max)`] - General-purpose, works with separate min/max values
    ///
    /// ```rust
    /// # use r3bl_tui::{IndexMarker, col};
    /// let range = col(3)..=col(7);
    /// let index = col(9);
    /// // These two are equivalent:
    /// let result1 = index.clamp_to_range(range.clone());
    /// let result2 = index.clamp(*range.start(), *range.end());
    /// # assert_eq!(result1, result2);
    /// # assert_eq!(result1, col(7));  // Both clamp to the end
    /// ```
    ///
    /// [Interval Notation]: crate::core::units::bounds_check#interval-notation
    /// [Module Documentation]: mod@crate::core::units::bounds_check
    /// [`RangeInclusive`]: std::ops::RangeInclusive
    /// [`clamp(min, max)`]: std::cmp::Ord::clamp
    #[must_use]
    fn clamp_to_range(&self, range: RangeInclusive<Self>) -> Self {
        (*self).clamp(*range.start(), *range.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{idx, len};

    #[test]
    fn test_convert_to_length() {
        let index = idx(5);
        let length = index.convert_to_length();
        assert_eq!(length.as_usize(), 6);
    }

    #[test]
    fn test_clamp_to_max_length() {
        let index = idx(5);
        let max_length = len(10);
        assert_eq!(index.clamp_to_max_length(max_length), index);

        let large_index = idx(15);
        let clamped = large_index.clamp_to_max_length(max_length);
        assert_eq!(clamped.as_usize(), 9); // max_length - 1
    }

    #[test]
    fn test_clamp_to_min_index() {
        let index = idx(5);
        let min_bound = idx(3);
        assert_eq!(index.clamp_to_min_index(min_bound), index);

        let small_index = idx(1);
        let clamped = small_index.clamp_to_min_index(min_bound);
        assert_eq!(clamped, min_bound);
    }

    #[test]
    fn test_clamp_to_range() {
        let range = idx(3)..=idx(7);

        // Below range - clamped to start
        assert_eq!(idx(1).clamp_to_range(range.clone()), idx(3));
        assert_eq!(idx(0).clamp_to_range(range.clone()), idx(3));

        // Within range - unchanged
        assert_eq!(idx(3).clamp_to_range(range.clone()), idx(3));
        assert_eq!(idx(5).clamp_to_range(range.clone()), idx(5));
        assert_eq!(idx(7).clamp_to_range(range.clone()), idx(7));

        // Above range - clamped to end
        assert_eq!(idx(8).clamp_to_range(range.clone()), idx(7));
        assert_eq!(idx(10).clamp_to_range(range.clone()), idx(7));

        // Single-element range
        let single = idx(5)..=idx(5);
        assert_eq!(idx(3).clamp_to_range(single.clone()), idx(5));
        assert_eq!(idx(5).clamp_to_range(single.clone()), idx(5));
        assert_eq!(idx(7).clamp_to_range(single.clone()), idx(5));
    }
}
