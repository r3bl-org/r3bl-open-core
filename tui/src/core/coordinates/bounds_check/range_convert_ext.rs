// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for converting from [`RangeInclusive`] to [`Range`] types - see
//! [`RangeConvertExt`] trait.

use super::index_ops::IndexOps;
use std::ops::{Add, Range, RangeInclusive};

/// Extension trait for converting from [`RangeInclusive`] to [`Range`] types.
///
/// This trait provides type-safe conversion between inclusive and exclusive range
/// semantics, primarily for [`VT-100`] terminal scroll regions.
///
/// <div class="warning">
///
/// We cannot add inherent methods to [`Range`] or [`RangeInclusive`] (orphan rule, since
/// they are in [`std`]), so we use an extension trait that can be implemented on foreign
/// types.
///
/// </div>
///
/// # Core Purpose
///
/// **Use case**: "I have a [`VT-100`] scroll region stored as [`RangeInclusive`] (both
/// endpoints valid), but I need a [`Range`] (exclusive end) for Rust iteration."
///
/// [`VT-100`] scroll regions use inclusive bounds (`2..=5` = rows 2,3,4,5), but Rust's
/// iteration requires exclusive bounds (`2..6` = rows 2,3,4,5). This trait eliminates
/// error-prone manual `+1` arithmetic with explicit, type-safe conversion.
///
/// See the [Interval Notation] section in the module documentation for notation details.
///
/// ## Range Type Semantics: Inclusive vs Exclusive
///
/// Understanding the difference between inclusive and exclusive ranges is crucial for
/// correct conversion:
///
/// - **[`RangeInclusive`] (inclusive)**: End value IS included - `2..=5` contains
///   `2,3,4,5`
/// - **[`Range`] (exclusive)**: End value NOT included - `2..6` contains `2,3,4,5`
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
/// #### Exclusive Range ([`Range<Index>`]) - End NOT Included
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
/// ■ CORRECT: (row(2)..=row(5)).to_exclusive()
///
///   Inclusive range: row(2)..=row(5)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │   │   │  Rows 2,3,4,5 included
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///
///   Converts to: row(2)..row(6) (adds +1 to end)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ ▓ │ X │   │  Rows 2,3,4,5 processed
///          └───┴───┴───┴───┴───┴───┴───┴───┘  (row 6 excluded)
///                                      ↑
///                                 end boundary
///   ■ Processes: row(2), row(3), row(4), row(5) - ALL CORRECT
///
/// □ WRONG: row(2)..row(5)
///
///   Exclusive range: row(2)..row(5)
///   Row:     0   1   2   3   4   5   6   7
///          ┌───┬───┬───┬───┬───┬───┬───┬───┐
///          │   │   │ ▓ │ ▓ │ ▓ │ X │   │   │  Row 5 excluded!
///          └───┴───┴───┴───┴───┴───┴───┴───┘
///                              ↑
///                         end boundary
///   □ Processes: row(2), row(3), row(4) only
///   □ MISSING: row(5) - BUG! Last row of scroll region not processed!
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
/// VT-100 scroll region: row(2)..=row(6) (inclusive - both endpoints valid)
///
/// To shift lines up within this region:
///   buffer.shift_lines_up(
///       (row(2)..=row(6)).to_exclusive(),  // Converts to row(2)..row(7)
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
/// use r3bl_tui::{row, RangeConvertExt};
///
/// // VT-100 scroll region: rows 2,3,4,5 (inclusive)
/// let scroll_region = row(2)..=row(5);
///
/// // Convert for Rust iteration: row(2)..row(6) (exclusive)
/// let iteration_range = scroll_region.to_exclusive();
///
/// assert_eq!(iteration_range.start, row(2));
/// assert_eq!(iteration_range.end, row(6));  // end+1
/// ```
///
/// **Buffer operations:**
/// ```rust
/// use r3bl_tui::{row, len, RangeConvertExt};
///
/// // VT-100 scroll region from row 1 to row 4 (both inclusive)
/// let scroll_region = row(1)..=row(4);
///
/// // Shift lines within scroll region
/// // buffer.shift_lines_up(scroll_region.to_exclusive(), len(1));
/// ```
///
/// **Single-element range:**
/// ```rust
/// # use r3bl_tui::{row, RangeConvertExt};
/// let single = row(3)..=row(3);  // Just row 3
/// let exclusive = single.to_exclusive();  // row(3)..row(4)
///
/// assert_eq!(exclusive.start, row(3));
/// assert_eq!(exclusive.end, row(4));
/// ```
///
/// # See Also
///
/// - [`RangeBoundsExt`] - Range validation against content boundaries
/// - [Module documentation] - Overview of bounds checking architecture
///
/// [Interval Notation]: mod@crate::core::coordinates::bounds_check#interval-notation
/// [Module documentation]: mod@crate::core::coordinates::bounds_check
/// [`Range<Index>`]: std::ops::Range
/// [`RangeBoundsExt`]: crate::RangeBoundsExt
/// [`RangeInclusive<Index>`]: std::ops::RangeInclusive
/// [`RangeInclusive`]: std::ops::RangeInclusive
/// [`Range`]: std::ops::Range
/// [`VT-100`]:
///     mod@crate::core::ansi::vt_100_pty_output_parser::operations::vt_100_shim_scroll_ops
/// [`to_exclusive()`]: RangeConvertExt::to_exclusive
pub trait RangeConvertExt {
    /// The index type contained in this range.
    type IndexType: IndexOps;

    /// Converts inclusive range to exclusive range by adding 1 to the end bound.
    ///
    /// See the [trait documentation] for detailed explanations, visual diagrams,
    /// [`VT-100`] examples, and common pitfalls.
    ///
    /// # Returns
    /// A [`Range`] with the same start and `end + 1`.
    ///
    /// [`Range`]: std::ops::Range
    /// [`VT-100`]:
    ///     mod@crate::core::ansi::vt_100_pty_output_parser::operations::vt_100_shim_scroll_ops
    /// [trait documentation]: Self
    #[must_use]
    fn to_exclusive(self) -> Range<Self::IndexType>;
}

/// Implementation for [`RangeInclusive<I>`] - the primary use case.
///
/// This implementation converts [`VT-100`] style inclusive ranges (where both endpoints
/// are valid positions) to Rust's exclusive ranges (where the end is not included) for
/// use with iteration and slice operations.
///
/// [`VT-100`]:
///     mod@crate::core::ansi::vt_100_pty_output_parser::operations::vt_100_shim_scroll_ops
impl<I> RangeConvertExt for RangeInclusive<I>
where
    I: IndexOps + Add<Output = I>,
{
    type IndexType = I;

    fn to_exclusive(self) -> Range<I> {
        let start = *self.start();
        let end = *self.end() + I::from(1u16);
        start..end
    }
}

#[cfg(test)]
mod range_conversion_tests {
    #[test]
    fn test_range_conversion_inclusive_to_exclusive() {
        use crate::{RangeConvertExt, row};

        // VT-100 scroll region: rows 2,3,4,5 (inclusive)
        let inclusive = row(2)..=row(5);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(2));
        assert_eq!(exclusive.end, row(6)); // end+1 for exclusive semantics
    }

    #[test]
    fn test_range_conversion_single_element() {
        use crate::{RangeConvertExt, row};

        // Single row region
        let inclusive = row(3)..=row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(3));
        assert_eq!(exclusive.end, row(4));
    }

    #[test]
    fn test_range_conversion_vt100_scroll_region() {
        use crate::{RangeConvertExt, row};

        // Simulate VT-100 scroll region lines 1-4 (both inclusive)
        let scroll_region = row(1)..=row(4);
        let iter_range = scroll_region.to_exclusive();

        // Should convert to 1..5 for iteration
        assert_eq!(iter_range.start, row(1));
        assert_eq!(iter_range.end, row(5));
    }

    #[test]
    fn test_range_conversion_zero_based() {
        use crate::{RangeConvertExt, row};

        // Range starting from 0
        let inclusive = row(0)..=row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(0));
        assert_eq!(exclusive.end, row(4));
    }
}
