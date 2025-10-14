// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Extension trait for [`Range`] and [`RangeInclusive`] validation for iteration and
//! algorithms - see [`RangeBoundsExt`] trait.

use super::{cursor_bounds_check::CursorBoundsCheck,
            index_ops::IndexOps,
            length_ops::LengthOps,
            numeric_value::NumericValue,
            result_enums::{RangeBoundsResult, RangeValidityStatus}};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorPositionBoundsStatus};
use std::ops::{Add, Range, RangeInclusive};

/// Extension trait for [`Range<Index>`] and [`RangeInclusive<Index>`] validation for
/// iteration and algorithms.
///
/// > <div class="warning">
/// >
/// > We cannot add inherent methods to [`Range`] or [`RangeInclusive`] (orphan rule,
/// > since they are in [`std`]), so we use an extension trait that can be implemented on
/// > foreign types.
/// >
/// > </div>
///
/// ## Core Purpose
///
/// **Use case**: "Is this [`Range<Index>`] or [`RangeInclusive<Index>`] well-formed and
/// valid **for a buffer/content of the given length**?"
///
/// This trait validates range objects against content boundaries, answering:
/// - Is the range structurally valid? (start ≤ end)
/// - Do start and end respect the buffer/content length?
/// - Does the range type (exclusive vs inclusive) match its intended use?
///
/// Range validation serves a different purpose from position checking:
/// - **Position checking**: "Is this single index safe/valid?" (via [`ArrayBoundsCheck`],
///   [`CursorBoundsCheck`])
/// - **Range validation**: "Is this Range object structurally valid for the given content
///   length?"
///
/// # Key Distinction from Other Bounds Traits
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | [`CursorBoundsCheck`]         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | [`ViewportBoundsCheck`]       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | `RangeBoundsExt`📍            | `start <= end <= length`      | Iteration     | Range object validation for content of given length  |
///
/// ### When to Use Which Type
///
/// - **Use [`Range`] (exclusive)**: Iteration, slicing, viewport windows where end is a
///   limit
/// - **Use [`RangeInclusive`] (inclusive)**: VT-100 scroll regions, text selections, when
///   both endpoints are valid positions
///
/// ## Common Use Cases
///
/// - **Iterator bounds validation**: Ensuring `for i in range` is safe
/// - **Algorithm parameter validation**: Checking slice ranges, processing windows
/// - **Buffer operation validation**: Ensuring operations stay within allocated space
/// - **Selection range validation**: Validating text selection boundaries
/// - **Viewport range validation**: Ensuring rendering ranges are well-formed
///
/// ## Design Philosophy
///
/// This trait focuses on **structural validation** of range objects against content
/// boundaries. It answers "Is this Range well-formed for content of this length?" rather
/// than "Is it safe to use this Range for operation X?"
///
/// The validation is type-aware, automatically applying correct rules based on whether
/// you're using [`Range`] (exclusive) or [`RangeInclusive`] (inclusive).
///
/// ## Core Methods
///
/// - [`check_range_is_valid_for_length()`] - Validate range structure and bounds against
///   content length
/// - [`check_index_is_within()`] - Check if an index is within the range's bounds
/// - [`clamp_range_to()`] - Ensure range fits within content bounds (preserving
///   semantics)
///
/// For detailed information on each method's behavior, see the method documentation.
///
/// ## Range Type Semantics: Exclusive vs Inclusive
///
/// This trait handles both [`Range`] (exclusive) and [`RangeInclusive`] (inclusive) with
/// their different boundary semantics. Understanding these differences is crucial:
///
/// - **[`Range`] (exclusive)**: End value NOT included - `5..10` contains `5,6,7,8,9`
/// - **[`RangeInclusive`] (inclusive)**: End value IS included - `5..=10` contains
///   `5,6,7,8,9,10`
///
/// ### Visual Comparison: Boundary Treatment
///
/// #### Exclusive Range ([`Range<Index>`]) - End NOT Included
///
/// ```text
/// Range 2..7 (exclusive end) - processes indices [2, 7)
///
///            min_index=2           max_index=7
///                  ↓                   ↓
/// Index:   0   1   2   3   4   5   6   7   8   9
///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///        │ U │ U │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ O │ O │ O │
///        └───┴───┼───┴───┴───┴───┴───┼───┴───┴───┘
///                ╰─── within range ──╯
///
/// U = Underflowed (index < min_index)
/// ▓ = Within (min_index <= index < max_index)
/// O = Overflowed (index >= max_index)
///
/// Key point: Index 7 is NOT included (Overflowed)
/// ```
///
/// #### Inclusive Range ([`RangeInclusive<Index>`]) - Both Ends Included
///
/// ```text
/// RangeInclusive 2..=7 (inclusive end) - processes indices [2, 7]
///
///       min_index=2              max_index=7
///           ↓                          ↓
/// Index:    0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │   │   │
///         └───┴───┼───┴───┴───┴───┴───┴───┼───┴───┘
///                 ╰───── within range ────╯
///                 (both ends included)
///
/// (2..=7).contains(&index):
/// - index=1 → false (before range)
/// - index=2 → true  (at start boundary)
/// - index=5 → true  (within range)
/// - index=7 → true  (at end boundary - INCLUDED)
/// - index=8 → false (after range)
///
/// Key point: Index 7 IS included (Within)
/// ```
///
/// ### Content Boundary Validation Rules
///
/// The critical insight: **end boundary validation depends on range type**.
///
/// #### Exclusive Range ([`Range<Index>`]) Validation
///
/// For [`Range<Index>`], the end can equal content length because it's NOT included:
///
/// ```text
/// Content with 10 columns (indices 0-9):
///           ╭────── content.len()=10 ───────────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///           ╰─────────── valid indices ─────────╯
///
/// Valid Range 5..10 (processes columns 5-9):
///                               ╭─── Range 5..10 ───╮
///                               ▼                   ▼
/// Column:   0   1   2   3   4   5   6   7   8   9   10
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑                   ↑
///                             start=5            end=10 (exclusive - NOT included)
///
/// ✅ Range 5..10 is VALID: end=10 equals length (cursor EOL semantics)
/// ❌ Range 5..11 is INVALID: end=11 exceeds length
/// ```
///
/// **Validation rules for [`Range<Index>`]**:
/// 1. `start <= end` (not inverted)
/// 2. `start < content.length` (start is valid array index)
/// 3. `end <= content.length` (end can equal length for exclusive ranges)
///
/// #### Inclusive Range ([`RangeInclusive<Index>`]) Validation
///
/// For [`RangeInclusive<Index>`], the end is INCLUDED, so it must be a valid index:
///
/// ```text
/// Content with 10 columns (indices 0-9):
///           ╭────── content.len()=10 ───────────╮
/// Column:   0   1   2   3   4   5   6   7   8   9   10 (invalid index)
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ F │ G │ H │ I │ J │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///           ╰─────────── valid indices ─────────╯
///
/// To process columns 5-9, use RangeInclusive 5..=9:
///                               ╭─RangeInclusive╮
///                               │ 5..=9         │
///                               │ (inclusive)   │
///                               ▼               ▼
/// Column:   0   1   2   3   4   5   6   7   8   9   10
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ ▓ │ ▓ │ ▓ │ ▓ │ ▓ │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑               ↑
///                            start=5        end=9 (IS included)
///
/// ✅ Range 5..=9 is VALID: end=9 < length (both endpoints accessible)
/// ❌ Range 5..=10 is INVALID: end=10 would access out-of-bounds index
/// ```
///
/// **Validation rules for [`RangeInclusive<Index>`]**:
/// 1. `start <= end` (not inverted)
/// 2. `start < content.length` (start is valid array index)
/// 3. `end < content.length` (end must ALSO be valid array index, NOT `<=`)
///
/// ## See Also
///
/// - [`IndexOps`] - Index operations and comparisons
/// - [`ArrayBoundsCheck`] - Array access safety validation
/// - [`CursorBoundsCheck`] - Cursor positioning validation
/// - [`ViewportBoundsCheck`] - Viewport visibility checking
/// - [`RangeConvertExt`] - Converting between inclusive and exclusive range types
/// - [Module documentation] - Overview of the complete bounds checking architecture
/// - [Interval Notation] - Reference for interval notation used throughout
///
/// [`check_range_is_valid_for_length()`]: RangeBoundsExt::check_range_is_valid_for_length
/// [`check_index_is_within()`]: RangeBoundsExt::check_index_is_within
/// [`clamp_range_to()`]: RangeBoundsExt::clamp_range_to
/// [`IndexOps`]: crate::IndexOps
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`RangeConvertExt`]: crate::RangeConvertExt
/// [Module documentation]: mod@crate::core::coordinates::bounds_check
/// [Interval Notation]: mod@crate::core::coordinates::bounds_check#interval-notation
/// [`Range<Index>`]: std::ops::Range
/// [`RangeInclusive<Index>`]: std::ops::RangeInclusive
/// [`Range`]: std::ops::Range
/// [`RangeInclusive`]: std::ops::RangeInclusive
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
pub trait RangeBoundsExt
where
    // Ensure index type supports addition (e.g., for range end
    // calculations).
    <Self::LengthType as LengthOps>::IndexType:
        Add<Output = <Self::LengthType as LengthOps>::IndexType>,
{
    /// The index type contained in this range.
    type IndexType: NumericValue;

    /// The length type that this range can be validated against.
    type LengthType: CursorBoundsCheck;

    /// Check if this range is valid for the given buffer/line length.
    ///
    /// See the [trait documentation][Self] for validation rules and range type semantics.
    ///
    /// # Returns
    ///
    /// [`RangeValidityStatus`] indicating validity or specific failure reason.
    fn check_range_is_valid_for_length(
        &self,
        buffer_length: impl Into<Self::LengthType>,
    ) -> RangeValidityStatus;

    /// Clamp this range to fit within buffer/line bounds.
    ///
    /// Ensures both start and end are valid for the given buffer length, preserving
    /// range type semantics (exclusive vs inclusive). Inverted ranges become empty.
    ///
    /// See the [trait documentation][Self] for detailed semantics.
    ///
    /// # Returns
    ///
    /// A new range guaranteed to be valid for the given buffer length.
    #[must_use]
    fn clamp_range_to(self, buffer_length: Self::LengthType) -> Self;

    /// Check if an index is within this range's bounds.
    ///
    /// Boundary semantics depend on range type (exclusive vs inclusive).
    /// See the [trait documentation][Self] for detailed semantics.
    ///
    /// # Returns
    /// - [`RangeBoundsResult::Underflowed`] if index < start
    /// - [`RangeBoundsResult::Within`] if index is within bounds
    /// - [`RangeBoundsResult::Overflowed`] if index is beyond bounds
    fn check_index_is_within(&self, index: Self::IndexType) -> RangeBoundsResult;
}

/// Implementation of range operations for [`Range<IndexType>`].
///
/// This provides type-safe validation and clamping for ranges of any index type
/// ([`ColIndex`], [`RowIndex`], etc.) against their corresponding length types.
///
/// The implementation handles the critical distinction between:
/// - Array bounds checking for the start index (must be valid for content access)
/// - Cursor position bounds checking for the end index (can equal content length)
///
/// This dual approach ensures that ranges are valid for iteration while respecting
/// exclusive end semantics.
///
/// [`ColIndex`]: crate::ColIndex
/// [`RowIndex`]: crate::RowIndex
/// [`Range<IndexType>`]: std::ops::Range
impl<I> RangeBoundsExt for Range<I>
where
    I: IndexOps + ArrayBoundsCheck<I::LengthType> + Add<Output = I>,
{
    type IndexType = I;
    type LengthType = I::LengthType;

    fn check_range_is_valid_for_length(
        &self,
        buffer_length: impl Into<Self::LengthType>,
    ) -> RangeValidityStatus {
        let length = buffer_length.into();

        // Check for inverted ranges (start > end).
        if self.start > self.end {
            return RangeValidityStatus::Inverted;
        }

        // Start must be within bounds (standard index check).
        if self.start.overflows(length) != ArrayOverflowResult::Within {
            return RangeValidityStatus::StartOutOfBounds;
        }

        // End can be equal to length for exclusive ranges (special case).
        // Use CursorPositionBoundsStatus to handle this correctly.
        if length.check_cursor_position_bounds(self.end)
            == CursorPositionBoundsStatus::Beyond
        {
            return RangeValidityStatus::EndOutOfBounds;
        }

        RangeValidityStatus::Valid
    }

    fn clamp_range_to(self, buffer_length: Self::LengthType) -> Range<I> {
        // If start is beyond bounds, return empty range at start.
        if self.start.overflows(buffer_length) == ArrayOverflowResult::Overflowed {
            let zero = I::LengthType::from(0usize).convert_to_index();
            return zero..zero;
        }

        // Clamp start to valid bounds (already checked it's within bounds above).
        let clamped_start = self.start;

        // For end, we need to handle exclusive range semantics:
        // - End can equal content_length (exclusive ranges allow this)
        // - End beyond content_length should be clamped to content_length
        let clamped_end = if buffer_length.check_cursor_position_bounds(self.end)
            == CursorPositionBoundsStatus::Beyond
        {
            // For exclusive ranges, the end can equal the length (unlike regular index
            // bounds checking). Use CursorBoundsCheck to get the position where
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

    fn check_index_is_within(&self, index: I) -> RangeBoundsResult {
        if index < self.start {
            RangeBoundsResult::Underflowed
        } else if index >= self.end {
            RangeBoundsResult::Overflowed
        } else {
            RangeBoundsResult::Within
        }
    }
}

/// Implementation of range operations for [`RangeInclusive<IndexType>`].
///
/// This provides type-safe validation and clamping for inclusive ranges of any index type
/// ([`ColIndex`], [`RowIndex`], etc.) against their corresponding length types.
///
/// The implementation enforces that BOTH start and end must be valid array indices:
/// - Array bounds checking for the start index (must be < length)
/// - Array bounds checking for the end index (must be < length, NOT <= like exclusive
///   ranges)
///
/// This ensures that inclusive ranges can safely access all elements from start to end,
/// since both endpoints are included in the range.
///
/// [`ColIndex`]: crate::ColIndex
/// [`RowIndex`]: crate::RowIndex
/// [`RangeInclusive<IndexType>`]: std::ops::RangeInclusive
impl<I> RangeBoundsExt for RangeInclusive<I>
where
    I: IndexOps + ArrayBoundsCheck<I::LengthType> + Add<Output = I>,
{
    type IndexType = I;
    type LengthType = I::LengthType;

    fn check_range_is_valid_for_length(
        &self,
        buffer_length: impl Into<Self::LengthType>,
    ) -> RangeValidityStatus {
        let length = buffer_length.into();

        // Check for inverted ranges (start > end).
        if self.start() > self.end() {
            return RangeValidityStatus::Inverted;
        }

        // Start must be within bounds (standard index check).
        if self.start().overflows(length) != ArrayOverflowResult::Within {
            return RangeValidityStatus::StartOutOfBounds;
        }

        // IMPORTANT: For inclusive ranges, the end is INCLUDED, so it must also be
        // a valid array index (< length), not just a valid cursor position (<= length).
        // This is the key difference from exclusive Range validation.
        if self.end().overflows(length) != ArrayOverflowResult::Within {
            return RangeValidityStatus::EndOutOfBounds;
        }

        RangeValidityStatus::Valid
    }

    fn clamp_range_to(self, buffer_length: Self::LengthType) -> RangeInclusive<I> {
        // If start is beyond bounds, return range at first index.
        if self.start().overflows(buffer_length) == ArrayOverflowResult::Overflowed {
            let zero = I::LengthType::from(0usize).convert_to_index();
            return zero..=zero;
        }

        // Clamp start to valid bounds.
        let clamped_start = *self.start();

        // For inclusive ranges, end must be clamped to the LAST VALID INDEX (length - 1),
        // not to length itself (which would be invalid for array access).
        let clamped_end =
            if self.end().overflows(buffer_length) == ArrayOverflowResult::Overflowed {
                // Get the last valid index: length - 1
                buffer_length.convert_to_index()
            } else {
                *self.end()
            };

        // Ensure range is not inverted.
        // For RangeInclusive, if start == end, it's a valid single-element range.
        if clamped_start > clamped_end {
            // Return single-element range at start position (mimics empty range behavior)
            clamped_start..=clamped_start
        } else {
            clamped_start..=clamped_end
        }
    }

    fn check_index_is_within(&self, index: I) -> RangeBoundsResult {
        if index < *self.start() {
            RangeBoundsResult::Underflowed
        } else if index > *self.end() {
            RangeBoundsResult::Overflowed
        } else {
            RangeBoundsResult::Within
        }
    }
}

#[cfg(test)]
mod tests_range_validation {
    use super::*;
    use crate::{Index, idx, len};

    #[test]
    fn test_range_validation_valid_ranges() {
        let content_width = len(10); // Columns 0-9

        // Valid ranges
        let range1: Range<Index> = idx(0)..idx(5); // columns 0-4
        let range2: Range<Index> = idx(5)..idx(10); // columns 5-9 (end=10 is valid!)
        let range3: Range<Index> = idx(0)..idx(10); // entire content
        let range4: Range<Index> = idx(7)..idx(9); // middle range
        let empty_range: Range<Index> = idx(5)..idx(5); // empty range

        assert_eq!(
            range1.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 0..5 should be valid for width 10"
        );
        assert_eq!(
            range2.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 5..10 should be valid for width 10 (exclusive end)"
        );
        assert_eq!(
            range3.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 0..10 should be valid for width 10 (full content)"
        );
        assert_eq!(
            range4.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 7..9 should be valid for width 10"
        );
        assert_eq!(
            empty_range.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Empty range 5..5 should be valid"
        );
    }

    #[test]
    fn test_range_validation_invalid_ranges() {
        let content_width = len(10); // Columns 0-9

        // Invalid ranges - end beyond content
        let bad_range1: Range<Index> = idx(5)..idx(11); // end > len
        let bad_range2: Range<Index> = idx(0)..idx(11); // end > len
        let bad_range3: Range<Index> = idx(15)..idx(20); // start beyond content

        assert_eq!(
            bad_range1.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Range 5..11 should be invalid for width 10"
        );
        assert_eq!(
            bad_range2.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Range 0..11 should be invalid for width 10"
        );
        assert_eq!(
            bad_range3.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::StartOutOfBounds,
            "Range 15..20 should be invalid for width 10"
        );

        // Invalid ranges - inverted
        let inverted_range: Range<Index> = idx(8)..idx(3); // inverted range
        assert_eq!(
            inverted_range.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Inverted,
            "Inverted range 8..3 should be invalid"
        );
    }
}

#[cfg(test)]
mod tests_range_clamp {
    use super::*;
    use crate::{Index, idx, len};

    #[test]
    fn test_range_clamp_to_content_normal_cases() {
        let content_width = len(10); // Columns 0-9, length 10

        // Normal range within bounds - should remain unchanged
        let range1: Range<Index> = idx(2)..idx(7);
        let clamped1 = range1.clamp_range_to(content_width);
        assert_eq!(
            clamped1,
            idx(2)..idx(7),
            "Normal range should remain unchanged"
        );

        // Range that exactly fits content (0..length) - should remain unchanged
        let full_range: Range<Index> = idx(0)..idx(10);
        let clamped_full = full_range.clamp_range_to(content_width);
        assert_eq!(
            clamped_full,
            idx(0)..idx(10),
            "Full content range should remain unchanged"
        );

        // Range to end of content (start..length) - should remain unchanged
        let to_end_range: Range<Index> = idx(5)..idx(10);
        let clamped_to_end = to_end_range.clamp_range_to(content_width);
        assert_eq!(
            clamped_to_end,
            idx(5)..idx(10),
            "Range to end should remain unchanged"
        );
    }

    #[test]
    fn test_range_clamp_to_content_end_beyond_bounds() {
        let content_width = len(10); // Columns 0-9, length 10

        // Range with end beyond bounds - should clamp end to content length
        let range1: Range<Index> = idx(5)..idx(15);
        let clamped1 = range1.clamp_range_to(content_width);
        assert_eq!(
            clamped1,
            idx(5)..idx(10),
            "End should be clamped to content length"
        );

        // Range starting from 0 with end beyond bounds
        let range2: Range<Index> = idx(0)..idx(15);
        let clamped2 = range2.clamp_range_to(content_width);
        assert_eq!(
            clamped2,
            idx(0)..idx(10),
            "Full range with end beyond should clamp to content"
        );

        // Range with both start and end way beyond bounds
        let range3: Range<Index> = idx(20)..idx(30);
        let clamped3 = range3.clamp_range_to(content_width);
        assert_eq!(
            clamped3,
            idx(0)..idx(0),
            "Range beyond bounds should become empty at start"
        );
    }

    #[test]
    fn test_range_clamp_exclusive_end_semantics() {
        let content_width = len(10); // Columns 0-9, length 10

        // Test that exclusive end semantics are preserved
        // Range 5..10 should remain 5..10 (end == length is valid for exclusive ranges)
        let range_to_end: Range<Index> = idx(5)..idx(10);
        let clamped_to_end = range_to_end.clamp_range_to(content_width);
        assert_eq!(
            clamped_to_end,
            idx(5)..idx(10),
            "Range to content end should preserve exclusive end semantics"
        );

        // Range 0..10 should remain 0..10 (full content range)
        let full_range: Range<Index> = idx(0)..idx(10);
        let clamped_full = full_range.clamp_range_to(content_width);
        assert_eq!(
            clamped_full,
            idx(0)..idx(10),
            "Full content range should preserve exclusive end semantics"
        );

        // Range 9..10 should remain 9..10 (single element range at end)
        let last_element: Range<Index> = idx(9)..idx(10);
        let clamped_last = last_element.clamp_range_to(content_width);
        assert_eq!(
            clamped_last,
            idx(9)..idx(10),
            "Range for last element should preserve exclusive end semantics"
        );
    }

    #[test]
    fn test_inverted_range_clamping() {
        let content_width = len(10);

        // Inverted range within bounds should become empty at start
        let inverted: Range<Index> = idx(8)..idx(3);
        let clamped = inverted.clamp_range_to(content_width);
        assert_eq!(
            clamped,
            idx(8)..idx(8),
            "Inverted range should become empty at start"
        );

        // Inverted range with start at boundary
        let inverted_boundary: Range<Index> = idx(9)..idx(5);
        let clamped_boundary = inverted_boundary.clamp_range_to(content_width);
        assert_eq!(
            clamped_boundary,
            idx(9)..idx(9),
            "Inverted range at boundary should become empty at start"
        );
    }

    #[test]
    fn test_zero_length_content() {
        let zero_content = len(0);

        // Any range with zero-length content should become empty at 0
        let range1: Range<Index> = idx(0)..idx(5);
        let clamped1 = range1.clamp_range_to(zero_content);
        assert_eq!(
            clamped1,
            idx(0)..idx(0),
            "Range with zero-length content should be empty at 0"
        );

        // Range starting beyond zero should also become empty at 0
        let range2: Range<Index> = idx(5)..idx(10);
        let clamped2 = range2.clamp_range_to(zero_content);
        assert_eq!(
            clamped2,
            idx(0)..idx(0),
            "Range beyond zero-length content should be empty at 0"
        );
    }
}

#[cfg(test)]
mod tests_range_inclusive_validation {
    use super::*;
    use crate::{Index, idx, len};

    #[test]
    fn test_range_inclusive_validation_valid_ranges() {
        let content_width = len(10); // Columns 0-9

        // Valid inclusive ranges - end must be < length (not <= like exclusive)
        let range1: RangeInclusive<Index> = idx(0)..=idx(4); // columns 0-4
        let range2: RangeInclusive<Index> = idx(5)..=idx(9); // columns 5-9
        let range3: RangeInclusive<Index> = idx(0)..=idx(9); // entire content
        let range4: RangeInclusive<Index> = idx(7)..=idx(9); // end range
        let single: RangeInclusive<Index> = idx(5)..=idx(5); // single element

        assert_eq!(
            range1.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 0..=4 should be valid for width 10"
        );
        assert_eq!(
            range2.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 5..=9 should be valid for width 10"
        );
        assert_eq!(
            range3.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 0..=9 should be valid for width 10 (full content)"
        );
        assert_eq!(
            range4.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Range 7..=9 should be valid for width 10"
        );
        assert_eq!(
            single.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Single-element range 5..=5 should be valid"
        );
    }

    #[test]
    fn test_range_inclusive_validation_invalid_ranges() {
        let content_width = len(10); // Columns 0-9

        // CRITICAL: For inclusive ranges, end=10 is INVALID (index 10 is out of bounds)
        let bad_range1: RangeInclusive<Index> = idx(5)..=idx(10); // end == len is INVALID
        let bad_range2: RangeInclusive<Index> = idx(0)..=idx(10); // end == len is INVALID
        let bad_range3: RangeInclusive<Index> = idx(0)..=idx(11); // end > len
        let bad_range4: RangeInclusive<Index> = idx(15)..=idx(20); // start beyond content

        assert_eq!(
            bad_range1.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Range 5..=10 should be INVALID for width 10 (inclusive ranges: end must be < length)"
        );
        assert_eq!(
            bad_range2.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Range 0..=10 should be INVALID for width 10 (inclusive ranges: end must be < length)"
        );
        assert_eq!(
            bad_range3.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Range 0..=11 should be invalid for width 10"
        );
        assert_eq!(
            bad_range4.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::StartOutOfBounds,
            "Range 15..=20 should be invalid for width 10"
        );

        // Invalid ranges - inverted
        let inverted: RangeInclusive<Index> = idx(8)..=idx(3);
        assert_eq!(
            inverted.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Inverted,
            "Inverted range 8..=3 should be invalid"
        );
    }
}

#[cfg(test)]
mod tests_range_inclusive_clamp {
    use super::*;
    use crate::{Index, idx, len};

    #[test]
    fn test_range_inclusive_clamp_normal_cases() {
        let content_width = len(10); // Columns 0-9, length 10

        // Normal range within bounds - should remain unchanged
        let range1: RangeInclusive<Index> = idx(2)..=idx(7);
        let clamped1 = range1.clamp_range_to(content_width);
        assert_eq!(
            clamped1,
            idx(2)..=idx(7),
            "Normal range should remain unchanged"
        );

        // Range that covers entire valid content (0..=9) - should remain unchanged
        let full_range: RangeInclusive<Index> = idx(0)..=idx(9);
        let clamped_full = full_range.clamp_range_to(content_width);
        assert_eq!(
            clamped_full,
            idx(0)..=idx(9),
            "Full content range should remain unchanged"
        );

        // Single-element range at end - should remain unchanged
        let last_element: RangeInclusive<Index> = idx(9)..=idx(9);
        let clamped_last = last_element.clamp_range_to(content_width);
        assert_eq!(
            clamped_last,
            idx(9)..=idx(9),
            "Single-element range at last index should remain unchanged"
        );
    }

    #[test]
    fn test_range_inclusive_clamp_end_beyond_bounds() {
        let content_width = len(10); // Columns 0-9, length 10

        // CRITICAL TEST: Range 5..=10 should clamp to 5..=9
        // This demonstrates the key difference from exclusive ranges
        let range1: RangeInclusive<Index> = idx(5)..=idx(10);
        let clamped1 = range1.clamp_range_to(content_width);
        assert_eq!(
            clamped1,
            idx(5)..=idx(9),
            "Range 5..=10 should clamp end to 9 (last valid index)"
        );

        // Range 0..=10 should clamp to 0..=9
        let range2: RangeInclusive<Index> = idx(0)..=idx(10);
        let clamped2 = range2.clamp_range_to(content_width);
        assert_eq!(
            clamped2,
            idx(0)..=idx(9),
            "Range 0..=10 should clamp end to 9"
        );

        // Range with end way beyond bounds
        let range3: RangeInclusive<Index> = idx(5)..=idx(15);
        let clamped3 = range3.clamp_range_to(content_width);
        assert_eq!(
            clamped3,
            idx(5)..=idx(9),
            "Range 5..=15 should clamp end to 9"
        );

        // Range with both start and end way beyond bounds
        let range4: RangeInclusive<Index> = idx(20)..=idx(30);
        let clamped4 = range4.clamp_range_to(content_width);
        assert_eq!(
            clamped4,
            idx(0)..=idx(0),
            "Range beyond bounds should become single-element at start"
        );
    }

    #[test]
    fn test_range_inclusive_inverted_range_clamping() {
        let content_width = len(10);

        // Inverted range within bounds should become single-element at start
        let inverted: RangeInclusive<Index> = idx(8)..=idx(3);
        let clamped = inverted.clamp_range_to(content_width);
        assert_eq!(
            clamped,
            idx(8)..=idx(8),
            "Inverted range should become single-element at start"
        );

        // Inverted range at boundary
        let inverted_boundary: RangeInclusive<Index> = idx(9)..=idx(5);
        let clamped_boundary = inverted_boundary.clamp_range_to(content_width);
        assert_eq!(
            clamped_boundary,
            idx(9)..=idx(9),
            "Inverted range at boundary should become single-element at start"
        );
    }

    #[test]
    fn test_range_inclusive_zero_length_content() {
        let zero_content = len(0);

        // Any inclusive range with zero-length content should become single-element at 0
        let range1: RangeInclusive<Index> = idx(0)..=idx(5);
        let clamped1 = range1.clamp_range_to(zero_content);
        assert_eq!(
            clamped1,
            idx(0)..=idx(0),
            "Range with zero-length content should be single-element at 0"
        );

        // Range starting beyond zero
        let range2: RangeInclusive<Index> = idx(5)..=idx(10);
        let clamped2 = range2.clamp_range_to(zero_content);
        assert_eq!(
            clamped2,
            idx(0)..=idx(0),
            "Range beyond zero-length content should be single-element at 0"
        );
    }
}

#[cfg(test)]
mod tests_compare_exclusive_vs_inclusive {
    use super::*;
    use crate::{Index, idx, len};

    #[test]
    fn test_range_inclusive_semantic_difference() {
        let content_width = len(10); // Columns 0-9, length 10

        // CRITICAL: Demonstrate the key semantic difference

        // Exclusive range 5..10 is VALID (end=10 is OK for exclusive)
        let exclusive: Range<Index> = idx(5)..idx(10);
        assert_eq!(
            exclusive.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Exclusive range 5..10 should be VALID (end can equal length)"
        );

        // Inclusive range 5..=10 is INVALID (end=10 would access out-of-bounds index)
        let inclusive: RangeInclusive<Index> = idx(5)..=idx(10);
        assert_eq!(
            inclusive.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::EndOutOfBounds,
            "Inclusive range 5..=10 should be INVALID (end must be < length)"
        );

        // The correct inclusive range for columns 5-9 is 5..=9
        let correct_inclusive: RangeInclusive<Index> = idx(5)..=idx(9);
        assert_eq!(
            correct_inclusive.check_range_is_valid_for_length(content_width),
            RangeValidityStatus::Valid,
            "Inclusive range 5..=9 should be VALID (end < length)"
        );
    }
}

#[cfg(test)]
mod tests_range_membership {
    use super::*;
    use crate::{col, idx, row};

    #[test]
    fn test_check_index_is_within_exclusive() {
        let range = col(3)..col(8); // Exclusive: [3, 8)

        // Test underflowed
        assert_eq!(
            range.check_index_is_within(col(0)),
            RangeBoundsResult::Underflowed,
            "Index 0 should underflow [3, 8)"
        );
        assert_eq!(
            range.check_index_is_within(col(2)),
            RangeBoundsResult::Underflowed,
            "Index 2 should underflow [3, 8)"
        );

        // Test within bounds
        assert_eq!(
            range.check_index_is_within(col(3)),
            RangeBoundsResult::Within,
            "Index 3 should be within [3, 8) (start included)"
        );
        assert_eq!(
            range.check_index_is_within(col(5)),
            RangeBoundsResult::Within,
            "Index 5 should be within [3, 8)"
        );
        assert_eq!(
            range.check_index_is_within(col(7)),
            RangeBoundsResult::Within,
            "Index 7 should be within [3, 8)"
        );

        // Test overflowed - end is EXCLUDED
        assert_eq!(
            range.check_index_is_within(col(8)),
            RangeBoundsResult::Overflowed,
            "Index 8 should overflow [3, 8) (end excluded)"
        );
        assert_eq!(
            range.check_index_is_within(col(10)),
            RangeBoundsResult::Overflowed,
            "Index 10 should overflow [3, 8)"
        );
    }

    #[test]
    fn test_check_index_is_within_all_states() {
        // This test specifically verifies that all three states of the enum are used
        let range = idx(5)..idx(10); // Exclusive: [5, 10)

        let underflow_result = range.check_index_is_within(idx(3));
        let within_result = range.check_index_is_within(idx(7));
        let overflow_result = range.check_index_is_within(idx(12));

        // Verify we get all three different states
        assert_eq!(underflow_result, RangeBoundsResult::Underflowed);
        assert_eq!(within_result, RangeBoundsResult::Within);
        assert_eq!(overflow_result, RangeBoundsResult::Overflowed);

        // Ensure all three states are distinct
        assert_ne!(underflow_result, within_result);
        assert_ne!(within_result, overflow_result);
        assert_ne!(underflow_result, overflow_result);
    }

    #[test]
    fn test_check_index_is_within_edge_cases() {
        // Test single-element range [5, 6)
        let single_element_range = idx(5)..idx(6);

        assert_eq!(
            single_element_range.check_index_is_within(idx(4)),
            RangeBoundsResult::Underflowed,
            "Index 4 should underflow [5, 6)"
        );
        assert_eq!(
            single_element_range.check_index_is_within(idx(5)),
            RangeBoundsResult::Within,
            "Index 5 should be within [5, 6)"
        );
        assert_eq!(
            single_element_range.check_index_is_within(idx(6)),
            RangeBoundsResult::Overflowed,
            "Index 6 should overflow [5, 6)"
        );

        // Test zero-width range [5, 5) - empty range
        let empty_range = idx(5)..idx(5);
        assert_eq!(
            empty_range.check_index_is_within(idx(4)),
            RangeBoundsResult::Underflowed,
            "Index 4 should underflow empty range [5, 5)"
        );
        assert_eq!(
            empty_range.check_index_is_within(idx(5)),
            RangeBoundsResult::Overflowed,
            "Empty range [5, 5) should have no valid indices"
        );
    }

    #[test]
    fn test_check_index_is_within_inclusive_basic() {
        // VT-100 scroll region: rows 2,3,4,5 are all valid (inclusive)
        let scroll_region = row(2)..=row(5);

        assert_eq!(
            scroll_region.check_index_is_within(row(1)),
            RangeBoundsResult::Underflowed,
            "Row 1 should be before scroll region [2, 5]"
        );
        assert_eq!(
            scroll_region.check_index_is_within(row(2)),
            RangeBoundsResult::Within,
            "Row 2 should be within scroll region [2, 5]"
        );
        assert_eq!(
            scroll_region.check_index_is_within(row(3)),
            RangeBoundsResult::Within,
            "Row 3 should be within scroll region [2, 5]"
        );
        assert_eq!(
            scroll_region.check_index_is_within(row(5)),
            RangeBoundsResult::Within,
            "Row 5 should be within scroll region [2, 5]"
        );
        assert_eq!(
            scroll_region.check_index_is_within(row(6)),
            RangeBoundsResult::Overflowed,
            "Row 6 should be after scroll region [2, 5]"
        );
    }

    #[test]
    fn test_check_index_is_within_inclusive_edge_cases() {
        // Test single-element inclusive range [5, 5]
        let single_element_range = idx(5)..=idx(5);
        assert_eq!(
            single_element_range.check_index_is_within(idx(4)),
            RangeBoundsResult::Underflowed,
            "Index 4 should underflow [5, 5]"
        );
        assert_eq!(
            single_element_range.check_index_is_within(idx(5)),
            RangeBoundsResult::Within,
            "Index 5 should be within [5, 5]"
        );
        assert_eq!(
            single_element_range.check_index_is_within(idx(6)),
            RangeBoundsResult::Overflowed,
            "Index 6 should overflow [5, 5]"
        );

        // Test boundary values with larger range
        let range = idx(10)..=idx(20);
        assert_eq!(
            range.check_index_is_within(idx(9)),
            RangeBoundsResult::Underflowed,
            "Index 9 should underflow [10, 20]"
        );
        assert_eq!(
            range.check_index_is_within(idx(10)),
            RangeBoundsResult::Within,
            "Index 10 should be within [10, 20]"
        );
        assert_eq!(
            range.check_index_is_within(idx(20)),
            RangeBoundsResult::Within,
            "Index 20 should be within [10, 20]"
        );
        assert_eq!(
            range.check_index_is_within(idx(21)),
            RangeBoundsResult::Overflowed,
            "Index 21 should overflow [10, 20]"
        );
    }
}
