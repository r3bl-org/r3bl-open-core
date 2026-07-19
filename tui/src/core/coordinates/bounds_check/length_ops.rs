// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words lengthops indexops

//! One-based size types and operations - see [`LengthOps`] trait.

use super::{index_ops::IndexOps, numeric_value::NumericValue,
            result_enums::ArrayOverflowResult};
use crate::{ArrayBoundsCheck, VPLength, vp_len};
use std::{cmp::{max, min},
          ops::Sub};

/// Trait for 1-based size/length types, providing operations for working with sizes and
/// measurements.
///
/// This trait provides the foundational operations for any type that represents a size,
/// count, or measurement of content (container sizes, buffer lengths, terminal
/// dimensions, etc.). It enables type-safe size calculations, space management, and
/// conversion between length and index semantics.
///
/// # Purpose
///
/// This trait serves types that represent "how much" or "how big" something is. While
/// [`IndexOps`] asks "where am I?", this trait asks "how much space do I have?". This
/// semantic distinction enables clearer, more maintainable code by making size-based
/// operations explicit.
///
/// # Key Trait Capabilities
///
/// - **Type conversion**: Convert 1-based lengths to 0-based indices via
///   [`convert_to_index()`]
/// - **Overflow checking**: Check if an index exceeds this size via
///   [`is_overflowed_by()`]
/// - **Space calculation**: Calculate remaining space from a position via
///   [`remaining_from()`]
/// - **Size clamping**: Ensure lengths don't exceed bounds via [`clamp_to_max()`]
/// - **Type-safe pairing**: Each length type pairs with a corresponding index type via
///   [`Self::IndexType`]
///
/// # Type System Foundation
///
/// Every [`LengthOps`] type has an associated [`Self::IndexType`] that represents the
/// corresponding 0-based position measurement. This creates a bidirectional type-safe
/// relationship preventing mismatched comparisons at compile time:
///
/// - [`VPLength`] ↔ [`VPIndex`]
/// - [`VPHeight`] ↔ [`VPRow`]
/// - [`VPWidth`] ↔ [`VPCol`]
/// - [`ByteLength`] ↔ [`ByteIndex`]
/// - [`SegLength`] ↔ [`SegIndex`]
///
/// This pairing prevents type mismatches like comparing [`VPWidth`] with
/// [`VPRow`].
///
/// # Implementing Types
///
/// The following types in this codebase implement [`LengthOps`]:
///
/// - [`VPLength`] - Generic 1-based size in viewport (dimension-agnostic)
/// - [`VPHeight`] - Vertical size in terminal grid
/// - [`VPWidth`] - Horizontal size in terminal grid
/// - [`ByteLength`] - Byte count in [`UTF-8`] strings
/// - [`SegLength`] - Grapheme segment count
///
/// ## Core Operations
///
/// ### Conversion
/// - [`convert_to_index()`] - Convert 1-based length to 0-based index (length - 1). Use
///   when finding the last valid position in a container.
/// - [`index_from_end()`] - Calculate an index positioned at a specific offset from the
///   end. Use for positioning elements relative to container boundaries (status bars,
///   bottom-aligned UI elements).
///
/// ### Overflow Checking
/// - [`is_overflowed_by()`] - "Does this length get overflowed by this index?" Same check
///   as `index.overflows(length)` but from the container's perspective.
///
/// ### Space Calculation
/// - [`remaining_from()`] - Calculate remaining space from a position. Essential for
///   rendering, text wrapping, and buffer management.
///
/// ### Size Clamping
/// - [`clamp_to_max()`] - Ensure length doesn't exceed maximum bounds. Use for
///   constraining operations to available space.
///
/// ## Visual Reference
///
/// ### Length-to-Index Conversion ([`convert_to_index()`])
///
/// ```text
/// Length=10 to index conversion:
///           ┌────────── length=10 (1-based) ────────┐
/// Length:     1   2   3   4   5   6   7   8   9   10
/// (1-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///           │   │   │   │   │   │   │   │   │   │ ▓ │
///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
/// Index:      0   1   2   3   4   5   6   7   8   9
/// (0-based)                                       ↑
///                                         convert_to_index() = 9
/// ```
///
/// ### Position from End Calculation ([`index_from_end()`])
///
/// ```text
/// Computing index 2 units from end (length=10, offset=2):
///
///     0   1   2   3   4   5   6   7   8   9     (indices, 0-based)
///   ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///   │   │   │   │   │   │   │   │ ▓ │   │   │   ← result: index 7
///   └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///     1   2   3   4   5   6   7   8   9   10    (lengths, 1-based)
///                                 └─ofs=2─┘
///
/// Calculation: (length=10 - offset=2).convert_to_index() = 8 - 1 = 7
///
/// Common use cases:
/// • Status bars/HUDs positioned from screen bottom
/// • Buffer navigation relative to end
/// • Layout calculations from bottom edge
///
/// Semantic clarity comparison:
/// ■ Unclear: length.convert_to_index() - vp_row(1)  // mixing domains
/// □ Clear:   length.index_from_end(vp_height(1))    // explicit intent
/// ```
///
/// ### Overflow Checking ([`is_overflowed_by()`])
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
/// ### Remaining Space Calculation ([`remaining_from()`])
///
/// ```text
/// With max_width=10:
///
///                 index=3 (0-based)
///                       ↓
/// Column:   0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │   │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │
///         ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┤
///         │           └───── 7 chars remain ──────┤
///         └────────── width=10 (1-based) ─────────┘
///
/// remaining_from(3)  = 7 (chars from index 3 to 9)
/// remaining_from(9)  = 1 (only position 9 remains)
/// remaining_from(10) = 0 (at boundary, nothing remains)
/// ```
///
/// ### Length Clamping ([`clamp_to_max()`])
///
/// ```text
/// Clamping operation with max_length=7:
///
/// Case 1: length=5 (within bounds)
/// ┌───── length=5 ────┬───────┐
/// │ 1   2   3   4   5 │ 6   7 ← max_length boundary
/// ├───┬───┬───┬───┬───┼───┬───┤
/// │ ■ │ ■ │ ■ │ ■ │ ■ │ □ │ □ │
/// └───┴───┴───┴───┴───┴───┴───┘
/// Legend: ■ Within | □ Exceeds
///
/// Result: clamp_to_max(5, max=7) = 5 (no change - within bounds)
///
/// Case 2: length=10 (exceeds bounds)
/// ┌───────────── length=10 ───┬───────────┐
/// │ 1   2   3   4   5   6   7 │ 8   9   10 (trimmed)
/// ├───┬───┬───┬───┬───┬───┬───┼───┬───┬───┤
/// │ ■ │ ■ │ ■ │ ■ │ ■ │ ■ │ ■ │ □ │ □ │ □ │
/// └───┴───┴───┴───┴───┴───┴───┼───┴───┴───┘
///                             └─ max_length=7 boundary
/// Legend: ■ Within | □ Exceeds
///
/// Result: clamp_to_max(10, max=7) = 7 (clamped to maximum)
/// ```
///
/// ## Design Philosophy
///
/// This trait embodies "length-centric thinking" - operations that naturally arise when
/// working with container sizes and measurements. Length operations complement index
/// operations, providing two perspectives on the same bounds checking logic:
///
/// - Index-centric: "Does my position overflow this container?"
/// - Length-centric: "Does this container get overflowed by that position?"
///
/// Both perspectives are useful in different contexts and provide natural ways to express
/// bounds checking.
///
/// ## Comparison with [`IndexOps`]
///
/// For a detailed side-by-side comparison of this trait (1-based sizes) and [`IndexOps`]
/// (0-based positions), see the [module-level comparison table]. This table clarifies
/// when to use each trait based on your specific use case.
///
/// ## Examples
///
/// This trait provides comprehensive size manipulation:
///
/// ```rust
/// use r3bl_tui::{ArrayOverflowResult, LengthOps, vp_col, vp_len, vp_width};
///
/// let container_width = vp_width(10);
/// let position = vp_col(5);
///
/// // Convert length to index (10 → 9, since 1-based → 0-based)
/// let last_index = container_width.convert_to_index();
/// assert_eq!(last_index.as_usize(), 9);
///
/// // Check if index overflows this length
/// assert_eq!(
///     container_width.is_overflowed_by(vp_col(5)),
///     ArrayOverflowResult::Within
/// );
/// assert_eq!(
///     container_width.is_overflowed_by(vp_col(10)),
///     ArrayOverflowResult::Overflowed
/// );
///
/// // Calculate remaining space from position
/// let remaining = container_width.remaining_from(position);
/// assert_eq!(remaining, vp_len(5));  // 5 positions remain (5,6,7,8,9)
///
/// // Clamp length to maximum
/// let large_length = vp_width(15);
/// let max_allowed = vp_width(10);
/// assert_eq!(large_length.clamp_to_max(max_allowed), vp_width(10));
/// ```
///
/// ## See Also
///
/// - [`IndexOps`] - Position-based operations and comparisons (the paired trait)
/// - [`NumericValue`] - Base trait that [`LengthOps`] extends
/// - [`ArrayBoundsCheck`] - Array access safety using length constraints
/// - [`CursorBoundsCheck`] - Cursor positioning using length constraints
///
/// [`ArrayBoundsCheck`]: crate::core::ArrayBoundsCheck
/// [`ByteIndex`]: crate::ByteIndex
/// [`ByteLength`]: crate::ByteLength
/// [`clamp_to_max()`]: LengthOps::clamp_to_max
/// [`convert_to_index()`]: LengthOps::convert_to_index
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`index_from_end()`]: LengthOps::index_from_end
/// [`IndexOps`]: crate::IndexOps
/// [`is_overflowed_by()`]: LengthOps::is_overflowed_by
/// [`LengthOps`]: crate::LengthOps
/// [`NumericValue`]: crate::NumericValue
/// [`remaining_from()`]: LengthOps::remaining_from
/// [`SegIndex`]: crate::SegIndex
/// [`SegLength`]: crate::SegLength
/// [`Self::IndexType`]: crate::IndexOps
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
/// [module-level comparison table]:
///     super#indexops-vs-lengthops-understanding-0-based-positions-vs-1-based-sizes
pub trait LengthOps: NumericValue {
    /// The corresponding index type for this length type.
    ///
    /// The constraint [`IndexOps<LengthType = Self>`] creates a bidirectional
    /// relationship: this ensures that the index type's [`LengthType`] points back to
    /// this same length type, preventing type mismatches like [`VPWidth`] ↔
    /// [`VPRow`].
    ///
    /// [`LengthType`]: crate::IndexOps::LengthType
    /// [`VPRow`]: crate::VPRow
    /// [`VPWidth`]: crate::VPWidth
    type IndexType: IndexOps<LengthType = Self>;

    /// Returns `true` if this length has 0 size.
    ///
    /// Length types represent dimensions, spans, or quantities of items. Semantically,
    /// checking if a dimension has no size means checking whether it is empty
    /// (`length.is_empty()`), matching standard Rust conventions where types with a
    /// `len()` method also provide `is_empty()`.
    ///
    /// For checking if a coordinate index is at origin `0`, see
    /// [`NumericValue::is_zero()`].
    ///
    /// [`NumericValue::is_zero()`]: crate::NumericValue::is_zero
    #[must_use]
    fn is_empty(&self) -> bool { self.is_zero() }

    /// Convert a 1-based length to the corresponding 0-based index of the last element.
    ///
    /// For any positive length $L > 0$, the last element is at index $L - 1$.
    /// For zero length, this saturates to index 0 (the beginning).
    ///
    /// Visual mapping:
    /// ```text
    /// Length = 5
    /// ┌───┬───┬───┬───┬───┐
    /// │ 0 │ 1 │ 2 │ 3 │ 4 │  ◄── Last element is at index 4 (5 - 1)
    /// └───┴───┴───┴───┴───┘
    ///
    /// Length = 1
    /// ┌───┐
    /// │ 0 │                  ◄── Last element is at index 0 (1 - 1)
    /// └───┘
    ///
    /// Length = 0 (empty)
    /// (empty)               ◄── Saturates to index 0
    /// ```
    ///
    /// This method is the inverse of [`IndexOps::convert_to_length`].
    ///
    /// This method does not have a default implementation because `Self::IndexType`
    /// no longer guarantees an infallible `From<usize>` conversion (to prevent silent
    /// truncation hazards on [`u16`]-backed screen coordinates). Each type must
    /// implement this safely using its native underlying type (e.g., [`u16`] for screen
    /// lengths, [`usize`] for storage lengths).
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthOps, vp_idx, vp_len};
    ///
    /// // Standard conversion: length=10 → index=9
    /// let length = vp_len(10);
    /// assert_eq!(length.convert_to_index(), vp_idx(9));
    ///
    /// // Edge case: length=0 saturates to index=0
    /// let zero_length = vp_len(0);
    /// assert_eq!(zero_length.convert_to_index(), vp_idx(0));
    /// ```
    ///
    /// [trait documentation]: Self
    #[must_use]
    fn convert_to_index(&self) -> Self::IndexType;

    /// Calculate an index positioned a specific offset from the end of this length.
    ///
    /// This method provides a semantic way to position elements relative to the end of a
    /// container, common in UI layouts (status bars, HUD elements) and buffer navigation.
    /// Instead of manually computing `(length - offset).convert_to_index()`, this method
    /// expresses the intent clearly.
    ///
    /// See the [trait documentation] for visual diagrams showing how positions from
    /// the end are calculated.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthOps, vp_idx, vp_len};
    ///
    /// let length = vp_len(10);
    ///
    /// // Position at the very end (last valid index)
    /// assert_eq!(length.index_from_end(vp_len(0)), vp_idx(9));
    ///
    /// // Position one unit from the end
    /// assert_eq!(length.index_from_end(vp_len(1)), vp_idx(8));
    ///
    /// // Position two units from the end
    /// assert_eq!(length.index_from_end(vp_len(2)), vp_idx(7));
    ///
    /// // Edge case: offset equals length (results in beginning)
    /// assert_eq!(length.index_from_end(vp_len(10)), vp_idx(0));
    /// ```
    ///
    /// [trait documentation]: Self
    #[must_use]
    fn index_from_end(&self, arg_offset: impl Into<Self>) -> Self::IndexType
    where
        Self: Sub<Output = Self>,
    {
        let offset: Self = arg_offset.into();
        (*self - offset).convert_to_index()
    }

    /// Checks if the given index would overflow this length's bounds.
    ///
    /// Answers the question: "Does this length get overflowed by this index?" This is the
    /// same check as [`index.overflows(length)`] but from the container's perspective.
    ///
    /// See the [trait documentation] for visual diagrams showing overflow
    /// boundaries and validation logic.
    ///
    /// # Returns
    /// - [`ArrayOverflowResult::Within`] if the index is valid
    /// - [`ArrayOverflowResult::Overflowed`] if the index would exceed bounds
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::{ArrayOverflowResult, LengthOps, vp_col, vp_width};
    ///
    /// let max_col = vp_width(10);
    /// assert_eq!(
    ///     max_col.is_overflowed_by(vp_col(5)),
    ///     ArrayOverflowResult::Within
    /// );
    /// assert_eq!(
    ///     max_col.is_overflowed_by(vp_col(10)),
    ///     ArrayOverflowResult::Overflowed
    /// );
    /// ```
    ///
    /// [`ArrayOverflowResult::Overflowed`]: crate::ArrayOverflowResult::Overflowed
    /// [`ArrayOverflowResult::Within`]: crate::ArrayOverflowResult::Within
    /// [`index.overflows(length)`]: crate::core::ArrayBoundsCheck::overflows
    /// [trait documentation]: Self
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

    /// Calculate the remaining space from the given index to the end of this length.
    ///
    /// See the [trait documentation] for visual diagrams showing how remaining space is
    /// calculated from different positions.
    ///
    /// # Returns
    /// - The number of units between the index and the boundary defined by this length.
    /// - If the index is at or beyond the boundary, returns `vp_len(0)`
    ///
    /// # Examples
    /// ```rust
    /// use r3bl_tui::{LengthOps, vp_col, vp_len, vp_width};
    ///
    /// let max_width = vp_width(10);
    /// assert_eq!(max_width.remaining_from(vp_col(3)), vp_len(7));  // 7 columns remain
    /// assert_eq!(max_width.remaining_from(vp_col(10)), vp_len(0)); // At boundary
    /// assert_eq!(max_width.remaining_from(vp_col(15)), vp_len(0)); // Beyond boundary
    /// ```
    ///
    /// [trait documentation]: Self
    #[must_use]
    #[rustfmt::skip]
    fn remaining_from(&self, arg_index: impl Into<Self::IndexType>) -> VPLength
    where
        Self::IndexType:
            ArrayBoundsCheck<Self>          /* Ensure index can check overflow against this length */
            + Sub<Output = Self::IndexType> /* Allow subtracting index types for distance */
            + IndexOps,                     /* Base index operations */
        Self:
              Into<VPLength>,               /* Resulting distance must convert to generic VPLength */
    {
        let index: Self::IndexType = arg_index.into();
        if self.is_overflowed_by(index) == ArrayOverflowResult::Overflowed {
            vp_len(0)
        } else {
            // Get max index for this length.
            let max_index = self.convert_to_index();
            // Calculate num of chars from cursor to boundary (as index difference).
            let chars_remaining_as_index = max_index - index;
            // Convert from 0-based index difference to 1-based length.
            chars_remaining_as_index.convert_to_length().into()
        }
    }

    /// Clamp this length to not exceed the given maximum value.
    ///
    /// See the [trait documentation][Self] for visual diagrams showing how
    /// length clamping works for both within-bounds and exceeding cases.
    ///
    /// # Returns
    /// The smaller of this length or the maximum length provided.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{LengthOps, vp_len};
    ///
    /// // Length within bounds - no change
    /// let small_length = vp_len(5);
    /// let max_allowed = vp_len(10);
    /// assert_eq!(small_length.clamp_to_max(max_allowed), vp_len(5));
    ///
    /// // Length exceeds bounds - gets clamped
    /// let large_length = vp_len(15);
    /// assert_eq!(large_length.clamp_to_max(vp_len(10)), vp_len(10));
    /// ```
    #[must_use]
    fn clamp_to_max(&self, arg_max_length: impl Into<Self>) -> Self {
        let max_length: Self = arg_max_length.into();
        min(*self, max_length)
    }

    /// Enforces a minimum length bound on this length.
    ///
    /// # Arguments
    ///
    /// * `arg_min_length` - The minimum allowed length
    ///
    /// # Returns
    ///
    /// Returns the greater of:
    /// - This length
    /// - The provided minimum length
    ///
    /// # Examples
    ///
    /// ```
    /// use r3bl_tui::{LengthOps, vp_len};
    ///
    /// // Length is smaller than minimum - gets clamped up
    /// let small_length = vp_len(5);
    /// let min_allowed = vp_len(10);
    /// assert_eq!(small_length.clamp_to_min(min_allowed), vp_len(10));
    ///
    /// // Length already exceeds minimum - remains unchanged
    /// let large_length = vp_len(15);
    /// assert_eq!(large_length.clamp_to_min(vp_len(10)), vp_len(15));
    /// ```
    #[must_use]
    fn clamp_to_min(&self, arg_min_length: impl Into<Self>) -> Self {
        let min_length: Self = arg_min_length.into();
        max(*self, min_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vp_idx;

    #[test]
    fn test_convert_to_index() {
        let length = vp_len(10);
        let index = length.convert_to_index();
        assert_eq!(index, vp_idx(9u16));

        // Edge case: length of 1
        let small_length = vp_len(1);
        let small_index = small_length.convert_to_index();
        assert_eq!(small_index, vp_idx(0u16));

        // Edge case: length of 0 (should saturate to 0)
        let zero_length = vp_len(0);
        let zero_index = zero_length.convert_to_index();
        assert_eq!(zero_index, vp_idx(0u16));
    }

    #[test]
    fn test_is_overflowed_by() {
        let length = vp_len(10);

        // Within bounds
        let within_index = vp_idx(5u16);
        assert_eq!(
            length.is_overflowed_by(within_index),
            ArrayOverflowResult::Within
        );

        // At boundary (last valid index)
        let boundary_index = vp_idx(9u16);
        assert_eq!(
            length.is_overflowed_by(boundary_index),
            ArrayOverflowResult::Within
        );

        // Overflow cases
        let overflow_index = vp_idx(10u16);
        assert_eq!(
            length.is_overflowed_by(overflow_index),
            ArrayOverflowResult::Overflowed
        );

        let large_overflow_index = vp_idx(15u16);
        assert_eq!(
            length.is_overflowed_by(large_overflow_index),
            ArrayOverflowResult::Overflowed
        );
    }

    #[test]
    fn test_remaining_from() {
        let length = vp_len(10);

        // Index in the middle
        let middle_index = vp_idx(3u16);
        let remaining = length.remaining_from(middle_index);
        assert_eq!(remaining.as_usize(), 7); // positions 3,4,5,6,7,8,9 = 7 positions

        // Index at the beginning
        let start_index = vp_idx(0u16);
        let remaining_from_start = length.remaining_from(start_index);
        assert_eq!(remaining_from_start.as_usize(), 10); // All positions remain

        // Index at the last valid position
        let last_index = vp_idx(9u16);
        let remaining_from_last = length.remaining_from(last_index);
        assert_eq!(remaining_from_last.as_usize(), 1); // Only position 9 remains

        // Index at boundary (overflow case)
        let boundary_index = vp_idx(10u16);
        let remaining_from_boundary = length.remaining_from(boundary_index);
        assert_eq!(remaining_from_boundary.as_usize(), 0); // Nothing remains

        // Index beyond boundary
        let beyond_index = vp_idx(15u16);
        let remaining_from_beyond = length.remaining_from(beyond_index);
        assert_eq!(remaining_from_beyond.as_usize(), 0); // Nothing remains
    }

    #[test]
    fn test_clamp_to_max() {
        let max_length = vp_len(10);

        // Length within bounds
        let small_length = vp_len(5);
        assert_eq!(small_length.clamp_to_max(max_length), small_length);

        // Length at bounds
        let equal_length = vp_len(10);
        assert_eq!(equal_length.clamp_to_max(max_length), max_length);

        // Length exceeding bounds
        let large_length = vp_len(15);
        assert_eq!(large_length.clamp_to_max(max_length), max_length);

        // Zero length
        let zero_length = vp_len(0);
        assert_eq!(zero_length.clamp_to_max(max_length), zero_length);
    }

    #[test]
    fn test_clamp_to_min() {
        let min_length = vp_len(10);

        // Length smaller than bounds
        let small_length = vp_len(5);
        assert_eq!(small_length.clamp_to_min(min_length), min_length);

        // Length at bounds
        let equal_length = vp_len(10);
        assert_eq!(equal_length.clamp_to_min(min_length), min_length);

        // Length exceeding bounds
        let large_length = vp_len(15);
        assert_eq!(large_length.clamp_to_min(min_length), large_length);

        // Zero length
        let zero_length = vp_len(0);
        assert_eq!(zero_length.clamp_to_min(min_length), min_length);
    }

    #[test]
    fn test_edge_cases_zero_length() {
        let zero_length = vp_len(0);

        // Any index should overflow zero length
        let zero_index = vp_idx(0u16);
        assert_eq!(
            zero_length.is_overflowed_by(zero_index),
            ArrayOverflowResult::Overflowed
        );

        let small_index = vp_idx(1u16);
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
        let unit_length = vp_len(1);

        // Only index 0 should be valid
        let zero_index = vp_idx(0u16);
        assert_eq!(
            unit_length.is_overflowed_by(zero_index),
            ArrayOverflowResult::Within
        );

        let one_index = vp_idx(1u16);
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

    #[test]
    fn test_index_from_end() {
        // Bottom position (offset=0)
        assert_eq!(vp_len(10).index_from_end(vp_len(0)), vp_idx(9u16));

        // One from bottom (offset=1)
        assert_eq!(vp_len(10).index_from_end(vp_len(1)), vp_idx(8u16));

        // Two from bottom (offset=2)
        assert_eq!(vp_len(10).index_from_end(vp_len(2)), vp_idx(7u16));

        // Edge case: offset equals length
        assert_eq!(vp_len(5).index_from_end(vp_len(5)), vp_idx(0u16));
    }
}
