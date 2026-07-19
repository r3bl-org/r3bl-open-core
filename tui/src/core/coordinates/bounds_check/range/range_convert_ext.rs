// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for converting from [`RangeInclusive`] to [`RangeExclusive`] types -
//! see [`RangeConvertExt`] trait.

use super::range_construct_ext::RangeExclusive;
use std::ops::RangeInclusive;

/// Extension trait for converting from [`RangeInclusive`] to [`RangeExclusive`] types.
///
/// This trait provides type-safe conversion between inclusive and exclusive range
/// semantics for coordinate indices, wrapper ranges ([`VPRow`],
/// [`CRow`], etc.), and [`VT-100`] terminal scroll regions.
///
/// <div class="warning">
///
/// We cannot add inherent methods to [`RangeExclusive`] or [`RangeInclusive`] (orphan
/// rule, since they are in [`std`]), so we use an extension trait that can be implemented
/// on foreign types.
///
/// </div>
///
/// # Purpose
///
/// This trait answers the question: **"How do I convert an inclusive range into an
/// exclusive range for iteration?"**
///
/// [`VT-100`] scroll regions use inclusive bounds (`2..=5` = rows 2,3,4,5), but Rust's
/// iteration requires exclusive bounds (`2..6` = rows 2,3,4,5). This trait eliminates
/// error-prone manual `+1` arithmetic with explicit, type-safe conversion.
///
/// See the [Interval Notation] section in the module documentation for notation details.
///
/// # Range Type Semantics: Inclusive vs Exclusive
///
/// Understanding the difference between inclusive and exclusive ranges is crucial for
/// correct conversion:
///
/// - **[`RangeInclusive`] (inclusive)**: End value IS included - `2..=5` contains
///   `2,3,4,5`
/// - **[`RangeExclusive`] (exclusive)**: End value NOT included - `2..6` contains
///   `2,3,4,5`
///
/// # Wrapper Range Conversions
///
/// `RangeConvertExt` seamlessly integrates with [`canvas`] wrapper
/// types ([`VPRow`], [`CRow`], etc.). For example, constructing a
/// type-safe Viewport range from a 1-based [`VPHeight`]:
///
/// ```rust
/// use r3bl_tui::{vp_row, LengthOps, RangeConvertExt, VPHeight};
///
/// let height = VPHeight::from(24);
/// // 1. Inclusive valid index range: [0..=23]
/// let inclusive = vp_row(0)..=vp_row(height.convert_to_index());
/// // 2. Exclusive iteration range: [0..24)
/// let exclusive = inclusive.to_exclusive();
///
/// assert_eq!(exclusive, vp_row(0)..vp_row(24));
/// ```
///
/// ### Visual Comparison: Same Set of Indices, Different Syntax
///
/// #### Inclusive Range ([`RangeInclusive<Index>`]) - Both Ends Included
///
/// ```text
/// RangeInclusive 2..=5 (inclusive end) - processes indices [2, 5]
///
///       min_index=2       max_index=5
///           ↓                   ↓
/// Index:    0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │   │   │
///         └───┴───┼───┴───┴───┴───┼───┴───┴───┴───┘
///                 ╰─within range──╯
///                 (both ends included)
///
/// (2..=5).contains(&index):
/// - index=1 → false (before range)
/// - index=2 → true  (at start boundary - INCLUDED)
/// - index=4 → true  (within range)
/// - index=5 → true  (at end boundary - INCLUDED)
/// - index=6 → false (after range)
///
/// VT-100 usage: Scroll region rows 2,3,4,5 are ALL valid
/// ```
///
/// #### Exclusive Range ([`RangeExclusive<Index>`]) - End NOT Included
///
/// ```text
/// Range 2..6 (exclusive end) - processes indices [2, 6)
///
///            min_index=2       max_index=6
///                  ↓               ↓
/// Index:   0   1   2   3   4   5   6   7   8   9
///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///        │   │   │ ▓ │ ▓ │ ▓ │ ▓ │ O │   │   │   │
///        └───┴───┼───┴───┴───┴───┼───┴───┴───┴───┘
///                ╰─within range──╯
///
/// ▓ = Within (min_index <= index < max_index)
/// O = Overflowed (index >= max_index)
///
/// (2..6).contains(&index):
/// - index=1 → false (before range)
/// - index=2 → true  (at start boundary - INCLUDED)
/// - index=4 → true  (within range)
/// - index=5 → true  (within range)
/// - index=6 → false (at end boundary - EXCLUDED)
///
/// Rust usage: for i in 2..6 iterates over 2,3,4,5 (NOT 6)
/// ```
///
/// **Key insight**: Both `2..=5` and `2..6` process the **same indices** (2,3,4,5), just
/// with different syntax. This trait performs the conversion.
///
/// ## Conversion Mechanics
///
/// The conversion adds `+1` to the inclusive end to create an exclusive end:
///
/// ```text
/// Inclusive → Exclusive Conversion
///
///     Input: 2..=5 (inclusive)          Output: 2..6 (exclusive)
///            ↓   ↓                              ↓  ↓
///        start  end                         start  end
///                                                  ↑
///                                                 +1
///
/// Semantic Translation:
/// - start..=end  →  start..(end+1)
/// - 2..=5        →  2..6
///
/// Both process indices: 2, 3, 4, 5
/// ```
///
/// ## Common Mistake: NOT the Same as Direct Exclusive Range
///
/// **CRITICAL**: `(a..=b).to_exclusive()` is **NOT** the same as `(a..b)`!
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │ Given: VT-100 scroll region from row 2 to row 5             │
/// └─────────────────────────────────────────────────────────────┘
/// Legend: ■ Correct | □ Incorrect
///
/// ■ CORRECT: (vp_row(2)..=vp_row(5)).to_exclusive()
///
///   Inclusive range: vp_row(2)..=vp_row(5)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │  Rows 2,3,4,5 included
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///
///   Converts to: vp_row(2)..vp_row(6) (adds +1 to end)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │ X │   │  Rows 2,3,4,5 processed
///          └───┴───┴───┴───┴───┴───┴───┴───┘  (row 6 excluded)
///                                      ↑
///                                 end boundary
///   ■ Processes: vp_row(2), vp_row(3), vp_row(4), vp_row(5) - ALL CORRECT
///
/// □ WRONG: vp_row(2)..vp_row(5)
///
///   Exclusive range: vp_row(2)..vp_row(5)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ X │   │   │  Row 5 excluded!
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///                              ↑
///                         end boundary
///   □ Processes: vp_row(2), vp_row(3), vp_row(4) only
///   □ MISSING: vp_row(5) - BUG! Last row of scroll region not processed!
/// ```
///
/// **Why the confusion?** In [`VT-100`] scroll regions, `scroll_bottom` represents the
/// **last valid row** in the region (inclusive). Using it directly as an exclusive end
/// (`row_index..scroll_bottom`) excludes that last row, causing subtle bugs.
///
/// **Solution**: Always use [`to_exclusive()`] when converting from [`VT-100`]'s
/// inclusive semantics to Rust's exclusive iteration semantics.
///
/// ## [`VT-100`] Scroll Region Example
///
/// [`VT-100`] terminals define scroll regions with inclusive bounds. Here's how to
/// compose them for Rust buffer operations:
///
/// ```text
/// Terminal Buffer (10 rows):
///
/// Row:  0  ┌────────────────────────┐
///       1  │ Header line            │
///       2  │ ┌────────────────────┐ │ ← scroll_top (inclusive)
///       3  │ │ Scrollable content │ │
///       4  │ │ Line 1             │ │
///       5  │ │ Line 2             │ │
///       6  │ └────────────────────┘ │ ← scroll_bottom (inclusive)
///       7  │ Status line            │
///       8  │ Footer                 │
///       9  └────────────────────────┘
///
/// VT-100 scroll region: vp_row(2)..=vp_row(6) (inclusive - both endpoints valid)
///
/// To shift lines up within this region:
///   buffer.shift_lines_in_range(
///       ShiftLinesDirection::Up,
///       (vp_row(2)..=vp_row(6)).to_exclusive(),  // Converts to vp_row(2)..vp_row(7)
///       len(1)
///   );
///
/// This processes rows 2,3,4,5,6 (all 5 rows in the scroll region).
/// ```
///
/// ## Examples
///
/// **Basic conversion:**
/// ```rust
/// use r3bl_tui::{RangeConvertExt, vp_row};
///
/// // VT-100 scroll region: rows 2,3,4,5 (inclusive)
/// let scroll_region = vp_row(2)..=vp_row(5);
///
/// // Convert for Rust iteration: vp_row(2)..vp_row(6) (exclusive)
/// let iteration_range = scroll_region.to_exclusive();
///
/// assert_eq!(iteration_range.start, vp_row(2));
/// assert_eq!(iteration_range.end, vp_row(6));  // end+1
/// ```
///
/// **Buffer operations:**
/// ```rust
/// use r3bl_tui::{RangeConvertExt, vp_row};
///
/// // VT-100 scroll region from row 1 to row 4 (both inclusive)
/// let scroll_region = vp_row(1)..=vp_row(4);
///
/// // Shift lines within scroll region
/// // buffer.shift_lines_in_range(ShiftLinesDirection::Up, scroll_region.to_exclusive(), vp_row(1));
/// ```
///
/// **Single-element range:**
/// ```rust
/// # use r3bl_tui::{RangeConvertExt, vp_row};
/// let single = vp_row(3)..=vp_row(3);  // Just row 3
/// let exclusive = single.to_exclusive();  // vp_row(3)..vp_row(4)
///
/// assert_eq!(exclusive.start, vp_row(3));
/// assert_eq!(exclusive.end, vp_row(4));
/// ```
///
/// # See Also
///
/// - [`RangeBoundsExt`] - Range validation against content boundaries
/// - [Module documentation] - Overview of bounds checking architecture
///
/// [`canvas`]: mod@crate::canvas
/// [`CRow`]: crate::CRow
/// [`RangeBoundsExt`]: crate::RangeBoundsExt
/// [`RangeExclusive`]: crate::RangeExclusive
/// [`RangeInclusive<Index>`]: std::ops::RangeInclusive
/// [`RangeInclusive`]: std::ops::RangeInclusive
/// [`to_exclusive()`]: RangeConvertExt::to_exclusive
/// [`VPHeight`]: crate::VPHeight
/// [`VPRow`]: crate::VPRow
/// [`VT-100`]:
///     mod@crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_scroll_ops
/// [Interval Notation]: mod@crate::bounds_check#interval-notation
/// [Module documentation]: mod@crate::bounds_check
pub trait RangeConvertExt {
    /// The index type contained in this range.
    type IndexType;

    /// Converts an inclusive range ([`RangeInclusive`], `[start, end]`) into an
    /// equivalent exclusive range ([`RangeExclusive`], `[start, end+1)`).
    ///
    /// # Core Purpose
    ///
    /// **Use case**: "I have a closed interval (such as a [`VT-100`] scroll region
    /// `2..=5` where both endpoints are valid indices), but I need an exclusive range
    /// `2..6` for standard Rust `for` loops or slice indexing."
    ///
    /// Both `range` (`2..=5`) and `range.to_exclusive()` (`2..6`) cover the exact same
    /// set of underlying indices (`2, 3, 4, 5`) and are semantically equivalent.
    ///
    /// # Visualizing Conversion `(2..=5` -> `2..6)`
    ///
    /// ```text
    /// Inclusive Input (RangeInclusive):
    /// - 2..=5
    /// - [2, 5]
    /// - covers 4 indices: 2, 3, 4, 5
    ///
    /// Index:      0   1   2   3   4   5   6   7
    ///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
    ///          └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                    ▲           ▲
    ///                 start=2     end=5 (inclusive)
    ///
    /// Exclusive Output via .to_exclusive():
    /// - 2..6
    /// - [2, 6)
    /// - covers 4 indices: 2, 3, 4, 5
    ///
    /// Index:      0   1   2   3   4   5   6   7
    ///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
    ///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
    ///          └───┴───┴───┴───┴───┴───┴───┴───┘
    ///                    ▲               ▲
    ///                 start=2         end=6 (exclusive)
    /// ```
    ///
    /// # Example Usage
    ///
    /// ```rust
    /// use r3bl_tui::{RangeConvertExt, RangeExclusive, VPRow, vp_row};
    ///
    /// let inclusive = vp_row(2)..=vp_row(5);
    /// let exclusive: RangeExclusive<VPRow> = inclusive.to_exclusive();
    ///
    /// assert_eq!(exclusive, vp_row(2)..vp_row(6));
    /// assert_eq!(exclusive.start, vp_row(2));
    /// assert_eq!(exclusive.end, vp_row(6));
    /// ```
    ///
    /// [`RangeExclusive`]: crate::RangeExclusive
    /// [`RangeInclusive`]: std::ops::RangeInclusive
    /// [`VT-100`]:
    ///     mod@crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_scroll_ops
    #[must_use]
    fn to_exclusive(self) -> RangeExclusive<Self::IndexType>;
}

/// Implementation for [`RangeInclusive<I>`] - the primary use case.
///
/// This implementation converts [`VT-100`] style inclusive ranges (where both endpoints
/// are valid positions) to Rust's exclusive ranges (where the end is not included) for
/// use with iteration and slice operations.
///
/// [`VT-100`]:
///     mod@crate::core::ansi::vt_100_pty_output_parser::ops::vt_100_shim_scroll_ops
macro_rules! impl_range_convert_ext {
    ($idx_ty:ident, $prim_ty:ty) => {
        impl RangeConvertExt for RangeInclusive<crate::$idx_ty> {
            type IndexType = crate::$idx_ty;

            fn to_exclusive(self) -> RangeExclusive<crate::$idx_ty> {
                let start = *self.start();
                let step: $prim_ty = 1;
                let end = *self.end() + step;
                start..end
            }
        }
    };
}

// 16-bit physical screen indices
impl_range_convert_ext!(VPRow, u16);
impl_range_convert_ext!(VPCol, u16);
impl_range_convert_ext!(VPIndex, u16);
impl_range_convert_ext!(SegIndex, u16);

// 64-bit logical storage indices
impl_range_convert_ext!(ByteIndex, usize);

#[cfg(test)]
mod range_conversion_tests {
    #[test]
    fn test_range_conversion_inclusive_to_exclusive() {
        use crate::{RangeConvertExt, vp_row};

        // VT-100 scroll region: rows 2,3,4,5 (inclusive)
        let inclusive = vp_row(2)..=vp_row(5);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, vp_row(2));
        assert_eq!(exclusive.end, vp_row(6)); // end+1 for exclusive semantics
    }

    #[test]
    fn test_range_conversion_single_element() {
        use crate::{RangeConvertExt, vp_row};

        // Single row region
        let inclusive = vp_row(3)..=vp_row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, vp_row(3));
        assert_eq!(exclusive.end, vp_row(4));
    }

    #[test]
    fn test_range_conversion_vt100_scroll_region() {
        use crate::{RangeConvertExt, vp_row};

        // Simulate VT-100 scroll region lines 1-4 (both inclusive)
        let scroll_region = vp_row(1)..=vp_row(4);
        let iter_range = scroll_region.to_exclusive();

        // Should convert to 1..5 for iteration
        assert_eq!(iter_range.start, vp_row(1));
        assert_eq!(iter_range.end, vp_row(5));
    }

    #[test]
    fn test_range_conversion_zero_based() {
        use crate::{RangeConvertExt, vp_row};

        // Range starting from 0
        let inclusive = vp_row(0)..=vp_row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, vp_row(0));
        assert_eq!(exclusive.end, vp_row(4));
    }

    #[test]
    fn test_range_conversion_64bit_storage_index() {
        use crate::{ByteIndex, RangeConvertExt};

        let inclusive = ByteIndex::from(10)..=ByteIndex::from(20);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, ByteIndex::from(10));
        assert_eq!(exclusive.end, ByteIndex::from(21));
    }
}
