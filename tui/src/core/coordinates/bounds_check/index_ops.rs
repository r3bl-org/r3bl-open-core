// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words indexops lengthops

//! Zero-based position types and operations - see [`IndexOps`] trait.

use super::{length_ops::LengthOps, numeric_value::NumericValue};
use crate::{ArrayBoundsCheck, ArrayOverflowResult};
use std::ops::RangeInclusive;

/// Trait for 0-based position/index types, providing operations for working with
/// positions within content.
///
/// This trait provides the foundational operations for any type that represents a
/// position or index within content (arrays, buffers, terminal coordinates, etc.).
/// It enables type-safe position manipulation, bounds checking, and conversion between
/// index and length semantics.
///
/// # Purpose
///
/// This trait serves types that represent "where" something is located.
/// While [`LengthOps`] asks "how much space?", this trait asks "where am I?".
/// This semantic distinction enables clearer, more maintainable code by making
/// position-based operations explicit.
///
/// # Key Trait Capabilities
///
/// - **Type conversion**: Convert 0-based indices to 1-based lengths via
///   [`convert_to_length()`]
/// - **Upper bound clamping**: Keep positions within container size via
///   [`clamp_to_max_length()`]
/// - **Lower bound clamping**: Enforce minimum positions via [`clamp_to_min_index()`]
/// - **Range clamping**: Constrain positions to inclusive ranges via [`clamp_to_range()`]
/// - **Type-safe pairing**: Each index type pairs with a corresponding length type via
///   [`LengthType`]
///
/// # Type System Foundation
///
/// Every `IndexOps` type has an associated [`LengthType`] that represents the
/// corresponding 1-based size measurement. This creates a bidirectional type-safe
/// relationship preventing mismatched comparisons at compile time:
///
/// - [`VPIndex`] ↔ [`VPLength`]
/// - [`VPRow`] ↔ [`VPHeight`]
/// - [`VPCol`] ↔ [`VPWidth`]
/// - [`ByteIndex`] ↔ [`ByteLength`]
/// - [`SegIndex`] ↔ [`SegLength`]
///
/// This pairing prevents type mismatches like comparing [`VPRow`] with
/// [`VPWidth`].
///
/// # Implementing Types
///
/// The following types in this codebase implement `IndexOps`:
///
/// - [`VPIndex`] - Generic 0-based position in viewport (dimension-agnostic)
/// - [`VPRow`] - Vertical position in terminal grid
/// - [`VPCol`] - Horizontal position in terminal grid
/// - [`ByteIndex`] - Byte position in [`UTF-8`] strings
/// - [`SegIndex`] - Grapheme segment position
///
/// # Core Operations
///
/// ## Conversion
/// - [`convert_to_length()`] - Convert 0-based index to 1-based length (index + 1)
///
/// ## Clamping
/// - [`clamp_to_min_index()`] - Clamp to minimum bound (index-to-index). Use for scroll
///   positions with a minimum starting point. Takes an index because lower bounds are
///   naturally expressed as minimum positions.
///
/// - [`clamp_to_max_length()`] - Clamp to container bounds (index-to-length). Use for
///   array/buffer access within a container size. Takes a length because upper bounds are
///   naturally expressed as container sizes.
///
/// - [`clamp_to_range()`] - Clamp to inclusive range (index-to-range). Use for [`VT-100`]
///   scroll regions or text selections where both endpoints are valid positions.
///
/// # Visual Reference
///
/// ## Index-to-Length Conversion ([`convert_to_length()`])
///
/// ```text
/// Index=5 to length conversion:
///             0   1   2   3   4   5   6   7   8   9
/// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///           │   │   │   │   │   │ ▓ │   │   │   │   │
///           └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
/// (1-based)   1   2   3   4   5   6   7   8   9   10
///                                 ↑
///                      convert_to_length() = 6
/// ```
///
/// ## Clamping to Maximum Length ([`clamp_to_max_length()`])
///
/// ```text
/// Clamping index=15 to max_length=10:
///           0   1   2   3   4   5   6   7   8   9 │ 10..15
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼──────►
///         │   │   │   │   │   │   │   │   │   │ ← │      ▓ (clamped)
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///         └────────── max_length=10 ──────────┘
///
/// Result: clamp_to_max_length(15, max=10) = 9
/// ```
///
/// ## Clamping to Minimum Index ([`clamp_to_min_index()`])
///
/// ```text
/// Clamping index=2 to min_bound=5:
///           0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ▓ │   │   │ → │   │   │   │   │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑
///                          min_bound=5
///
/// Result: clamp_to_min_index(2, min=5) = 5
/// ```
///
/// ## Clamping to Inclusive Range ([`clamp_to_range()`])
///
/// ```text
/// Clamping to inclusive range [3..=7]:
///           0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ ← │ ← │ ← │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ → │ → │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                     └─  range [3..=7] ──┘
///
/// clamp_to_range(1, [3..=7]) = 3  (clamped to start)
/// clamp_to_range(5, [3..=7]) = 5  (within range)
/// clamp_to_range(9, [3..=7]) = 7  (clamped to end)
/// ```
///
/// # Design Philosophy
///
/// This trait embodies "index-centric thinking" - operations that naturally arise when
/// working with positions and locations. Index operations complement length operations,
/// providing two perspectives on the same bounds checking logic:
///
/// - Index-centric: "Does my position overflow this container?"
/// - Length-centric: "Does this container get overflowed by that position?"
///
/// Both perspectives are useful in different contexts and provide natural ways to express
/// bounds checking.
///
/// # Comparison with [`LengthOps`]
///
/// For a detailed side-by-side comparison of this trait (0-based positions) and
/// [`LengthOps`] (1-based sizes), see the [module-level comparison table].
/// This table clarifies when to use each trait based on your specific use case.
///
/// # Examples
///
/// This trait provides comprehensive position manipulation:
///
/// ```rust
/// use r3bl_tui::{IndexOps, vp_col, vp_width};
///
/// let position = vp_col(5);
/// let container_width = vp_width(10);
///
/// // Convert index to length (5 → 6, since 0-based → 1-based)
/// let length = position.convert_to_length();
/// assert_eq!(length.as_usize(), 6);
///
/// // Clamp to maximum container size
/// let large_pos = vp_col(15);
/// let clamped = large_pos.clamp_to_max_length(container_width);
/// assert_eq!(clamped, vp_col(9));  // Clamped to max valid index
///
/// // Clamp to minimum bound
/// let small_pos = vp_col(2);
/// let min_bound = vp_col(5);
/// let clamped_min = small_pos.clamp_to_min_index(min_bound);
/// assert_eq!(clamped_min, vp_col(5));  // Clamped up to minimum
///
/// // Clamp to inclusive range (e.g., VT-100 scroll region)
/// let scroll_region = vp_col(3)..=vp_col(7);
/// assert_eq!(vp_col(1).clamp_to_range(scroll_region.clone()), vp_col(3));  // Below range
/// assert_eq!(vp_col(5).clamp_to_range(scroll_region.clone()), vp_col(5));  // Within
/// assert_eq!(vp_col(9).clamp_to_range(scroll_region.clone()), vp_col(7));  // Above range
/// ```
///
/// # See Also
///
/// - [`LengthOps`] - Operations specific to length/size values (the paired trait)
/// - [`NumericValue`] - Base trait that `IndexOps` extends
/// - [`ArrayBoundsCheck`] - Array access safety using index-to-length comparisons
/// - [`CursorBoundsCheck`] - Cursor positioning using index-to-length comparisons
/// - [`ViewportBoundsCheck`] - Viewport visibility using hybrid operations
/// - [module-level comparison table] - `IndexOps` vs `LengthOps` at a glance
///
/// [`ArrayBoundsCheck`]: crate::core::ArrayBoundsCheck
/// [`ByteIndex`]: crate::ByteIndex
/// [`ByteLength`]: crate::ByteLength
/// [`clamp_to_max_length()`]: IndexOps::clamp_to_max_length
/// [`clamp_to_min_index()`]: IndexOps::clamp_to_min_index
/// [`clamp_to_range()`]: IndexOps::clamp_to_range
/// [`convert_to_length()`]: IndexOps::convert_to_length
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`LengthOps`]: crate::LengthOps
/// [`LengthType`]: Self::LengthType
/// [`NumericValue`]: crate::NumericValue
/// [`SegIndex`]: crate::SegIndex
/// [`SegLength`]: crate::SegLength
/// [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`VPCol`]: crate::VPCol
/// [`VPHeight`]: crate::VPHeight
/// [`VPIndex`]: crate::VPIndex
/// [`VPLength`]: crate::VPLength
/// [`VPRow`]: crate::VPRow
/// [`VPWidth`]: crate::VPWidth
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [module-level comparison table]: super#indexops-vs-lengthops-understanding-0-based-positions-vs-1-based-sizes
pub trait IndexOps: NumericValue {
    /// The corresponding "length" type for this "index" type.
    ///
    /// The constraint [`LengthOps<IndexType = Self>`] creates a bidirectional
    /// relationship: this ensures that the length type's [`Self::IndexType`] points back
    /// to this same index type, preventing type mismatches like [`VPCol`] ↔
    /// [`VPHeight`].
    ///
    /// [`LengthOps<IndexType = Self>`]: crate::LengthOps
    /// [`Self::IndexType`]: crate::LengthOps::IndexType
    /// [`VPCol`]: crate::VPCol
    /// [`VPHeight`]: crate::VPHeight
    type LengthType: LengthOps<IndexType = Self>;

    /// Converts this 0-based index to 1-based length (adds 1).
    ///
    /// See the [trait documentation][Self] for index/length relationship.
    ///
    /// # Implementation Notes
    ///
    /// This method does not have a default implementation because `Self::LengthType` no
    /// longer guarantees an infallible `From<usize>` conversion (to prevent silent
    /// truncation hazards on [`u16`]-backed screen coordinates). Each type must implement
    /// this safely using its native underlying type (e.g., [`u16`] for screen indices,
    /// [`usize`] for storage indices).
    fn convert_to_length(&self) -> Self::LengthType;

    /// Clamp this index to stay within container bounds `[0, length)`.
    ///
    /// Takes a length parameter since upper bounds are naturally expressed as container
    /// sizes. See the [trait documentation][Self] for clamping operation details.
    ///
    /// # Returns
    /// The index unchanged if within bounds, or `length - 1` if it overflows.
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

    /// Ensure this index is at least the given minimum bound.
    ///
    /// Takes an index parameter since lower bounds are naturally expressed as positions.
    /// See the [trait documentation][Self] for clamping operation details.
    ///
    /// # Returns
    /// The minimum bound if this index is less than it, otherwise self unchanged.
    #[must_use]
    fn clamp_to_min_index(&self, min_bound: impl Into<Self>) -> Self {
        let min: Self = min_bound.into();
        (*self).max(min)
    }

    /// Clamp this index to stay within an inclusive range `[start..=end]`.
    ///
    /// Useful for [`VT-100`] scroll regions and text selections where both boundaries are
    /// valid. See the [trait documentation][Self] for clamping operation details.
    ///
    /// # Returns
    /// The index clamped to the range boundaries (inclusive).
    ///
    /// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
    #[must_use]
    fn clamp_to_range(&self, range: RangeInclusive<Self>) -> Self {
        (*self).clamp(*range.start(), *range.end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vp_idx, vp_len};

    #[test]
    fn test_convert_to_length() {
        let index = vp_idx(5u16);
        let length = index.convert_to_length();
        assert_eq!(length.as_usize(), 6);
    }

    #[test]
    fn test_clamp_to_max_length() {
        let index = vp_idx(5u16);
        let max_length = vp_len(10);
        assert_eq!(index.clamp_to_max_length(max_length), index);

        let large_index = vp_idx(15u16);
        let clamped = large_index.clamp_to_max_length(max_length);
        assert_eq!(clamped.as_usize(), 9); // max_length - 1
    }

    #[test]
    fn test_clamp_to_min_index() {
        let index = vp_idx(5u16);
        let min_bound = vp_idx(3u16);
        assert_eq!(index.clamp_to_min_index(min_bound), index);

        let small_index = vp_idx(1u16);
        let clamped = small_index.clamp_to_min_index(min_bound);
        assert_eq!(clamped, min_bound);
    }

    #[test]
    fn test_clamp_to_range() {
        let range = vp_idx(3u16)..=vp_idx(7u16);

        // Below range - clamped to start
        assert_eq!(vp_idx(1u16).clamp_to_range(range.clone()), vp_idx(3u16));
        assert_eq!(vp_idx(0u16).clamp_to_range(range.clone()), vp_idx(3u16));

        // Within range - unchanged
        assert_eq!(vp_idx(3u16).clamp_to_range(range.clone()), vp_idx(3u16));
        assert_eq!(vp_idx(5u16).clamp_to_range(range.clone()), vp_idx(5u16));
        assert_eq!(vp_idx(7u16).clamp_to_range(range.clone()), vp_idx(7u16));

        // Above range - clamped to end
        assert_eq!(vp_idx(8u16).clamp_to_range(range.clone()), vp_idx(7u16));
        assert_eq!(vp_idx(10u16).clamp_to_range(range.clone()), vp_idx(7u16));

        // Single-element range
        let single = vp_idx(5u16)..=vp_idx(5u16);
        assert_eq!(vp_idx(3u16).clamp_to_range(single.clone()), vp_idx(5u16));
        assert_eq!(vp_idx(5u16).clamp_to_range(single.clone()), vp_idx(5u16));
        assert_eq!(vp_idx(7u16).clamp_to_range(single.clone()), vp_idx(5u16));
    }
}
