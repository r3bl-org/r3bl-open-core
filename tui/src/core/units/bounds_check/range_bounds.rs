// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use std::ops::{Add, Range, RangeInclusive};

use super::{cursor_bounds::CursorBoundsCheck, index_marker::IndexMarker,
            length_marker::LengthMarker, result_enums::RangeBoundsResult};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorPositionBoundsStatus};

/// [`Range<Index>`] validation for iteration and algorithms.
///
/// This trait provides validation operations for Rust's [`Range<Index>`] objects,
/// ensuring that ranges are valid for use in iteration, algorithms, and other
/// operations that require well-formed ranges.
///
/// ## Core Purpose
///
/// Use case: "Is this [`Range<Index>`] valid for iteration/algorithms?"
///
/// Range validation serves a different purpose from bounds checking:
/// - Bounds checking: "Is this position safe/valid?"
/// - Range validation: "Is this Range object well-formed?"
///
/// # Key Distinction from Other Bounds Traits
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | [`ArrayBoundsCheck`]          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | [`CursorBoundsCheck`]         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | [`ViewportBoundsCheck`]       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | `RangeBoundsCheck`📍          | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// ## Why Range Validation Matters
///
/// Rust's [`Range<T>`] uses exclusive end semantics, meaning the end value is NOT
/// included in the range. This creates special validation requirements that differ
/// from simple bounds checking:
///
/// 1. Range structure: Start must be ≤ end
/// 2. Content boundaries: Range must not exceed content bounds
/// 3. Exclusive end semantics: End can equal content length (valid for iteration)
///
/// ## Interval Notation
///
/// See the [Interval Notation] section in the module documentation for a complete
/// reference on interval notation used throughout this codebase.
///
/// ## Range Type Semantics
///
/// This trait provides validation and clamping operations for both [`Range`] (exclusive)
/// and [`RangeInclusive`] (inclusive) range types, ensuring they are well-formed and
/// respect content boundaries with their respective end semantics:
///
/// - **[`Range`] (exclusive)**: End value NOT included - `5..10` contains `5,6,7,8,9`
/// - **[`RangeInclusive`] (inclusive)**: End value IS included - `5..=10` contains
///   `5,6,7,8,9,10`
///
/// ## Exclusive End Semantics
///
/// The key insight is that [`Range<Index>`] with exclusive end semantics `[start, end)`
/// allows the end value to equal the content length, even though that index would be
/// invalid for direct content access:
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
///         │ A │ B │ C │ D │ E │ X │ X │ X │ X │ X │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑                   ↑
///                             start=5            end=10 (exclusive)
/// ```
///
/// The end index 10 equals content length, which is valid for exclusive range
/// semantics but would be invalid for direct array access.
///
/// ## Exclusive vs Inclusive Range Comparison
///
/// Understanding the boundary treatment difference between [`Range`] (exclusive) and
/// [`RangeInclusive`] (inclusive) is crucial for choosing the right range type.
///
/// ### Exclusive Range (`Range<Index>`) - End NOT Included
///
/// ```text
/// Range 2..7 (exclusive end) - processes indices [2, 7)
///
///            min_index=2           max_index=7
///                  ↓                   ↓
/// Index:   0   1   2   3   4   5   6   7   8   9
///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///        │ U │ U │ W │ W │ W │ W │ W │ O │ O │ O │
///        └───┴───┼───┴───┴───┴───┴───┼───┴───┴───┘
///                ╰─── within range ──╯
///
/// U = Underflowed (index < min_index)
/// W = Within (min_index <= index < max_index)
/// O = Overflowed (index >= max_index)
///
/// Key point: Index 7 is NOT included (Overflowed)
/// ```
///
/// ### Inclusive Range (`RangeInclusive<Index>`) - Both Ends Included
///
/// ```text
/// RangeInclusive 2..=7 (inclusive end) - processes indices [2, 7]
///
///       min_index=2              max_index=7
///           ↓                          ↓
/// Index:    0   1   2   3   4   5   6   7   8   9
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │   │   │ ● │ ● │ ● │ ● │ ● │ ● │   │   │
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
/// ### When to Use Which Type
///
/// - **Use [`Range`] (exclusive)**: Iteration, slicing, viewport windows where end is a
///   limit
/// - **Use [`RangeInclusive`] (inclusive)**: VT-100 scroll regions, text selections, when
///   both endpoints are valid positions
///
/// ## Range Type Conversion
///
/// While Rust provides both [`Range`] (exclusive) and [`RangeInclusive`] (inclusive),
/// certain domains store ranges in one form but need to process them in another. The most
/// common example is **VT-100 terminal scroll regions**.
///
/// ### The VT-100 Conversion Challenge
///
/// VT-100 terminals define scroll regions using inclusive bounds where both the top and
/// bottom row numbers are valid, accessible positions. For example, a scroll region from
/// row 2 to row 5 means rows 2, 3, 4, and 5 are all in the scroll region.
///
/// However, Rust's iteration and slice operations require exclusive ranges where the end
/// is NOT included. To process the same set of rows (2, 3, 4, 5), you need an exclusive
/// range `2..6`.
///
/// ### The Problem with Manual Conversion
///
/// Without explicit conversion utilities, developers must manually add `+1` to the
/// inclusive end when converting to exclusive semantics:
///
/// ```text
/// // VT-100 scroll region (inclusive)
/// let scroll_top = row(2);
/// let scroll_bottom = row(5);  // Last valid row in region
///
/// // Manual conversion - ERROR-PRONE!
/// let range = scroll_top..(scroll_bottom + 1);  // Easy to forget the +1
/// ```
///
/// This pattern is error-prone because:
/// 1. It's easy to forget the `+1` conversion
/// 2. The semantic intent is unclear (why are we adding 1?)
/// 3. It requires understanding exclusive vs inclusive semantics at every call site
///
/// ### The Solution: Type-Safe Conversion
///
/// The [`RangeConversion`] trait provides explicit, type-safe conversion methods that
/// handle the semantic translation automatically:
///
/// ```rust
/// # use r3bl_tui::{row, RangeConversion};
/// // VT-100 scroll region (inclusive) - both endpoints valid
/// let scroll_region = row(2)..=row(5);
///
/// // Type-safe conversion - EXPLICIT INTENT!
/// let iteration_range = scroll_region.to_exclusive();  // row(2)..row(6)
/// ```
///
/// Benefits:
/// - **Explicit intent**: The method name clearly indicates what's happening
/// - **Type safety**: Conversion is handled by the trait, not manual arithmetic
/// - **Correctness**: No risk of forgetting the `+1` adjustment
/// - **Maintainability**: Semantic meaning is preserved in code
///
/// ### See Also
///
/// See [`RangeConversion`] trait documentation for detailed implementation, examples,
/// and important semantic differences between `(a..=b).to_exclusive()` and `(a..b)`.
///
/// ## Common Use Cases
///
/// - Iterator bounds validation: Ensuring `for i in range` is safe
/// - Algorithm parameter validation: Checking slice ranges, processing windows
/// - Buffer operation validation: Ensuring operations stay within allocated space
/// - Selection range validation: Validating text selection boundaries
/// - Viewport range validation: Ensuring rendering ranges are well-formed
///
/// ## Design Philosophy
///
/// This trait focuses on structural validation of range objects rather than
/// semantic validation of their use. It answers "Is this Range well-formed?" rather
/// than "Is it safe to use this Range for operation X?"
///
/// ## See Also
///
/// - [`IndexMarker`] - Index operations and comparisons
/// - [`ArrayBoundsCheck`] - Array access safety validation
/// - [`CursorBoundsCheck`] - Cursor positioning validation
/// - [`ViewportBoundsCheck`] - Viewport visibility checking
/// - [`RangeConversion`] - Converting between inclusive and exclusive range types
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [`IndexMarker`]: crate::IndexMarker
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`RangeConversion`]: crate::RangeConversion
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
/// [`Range<Index>`]: std::ops::Range
/// [`Range<T>`]: std::ops::Range
///
/// ## Why content boundary semantics matter for ranges
///
/// Different range types have different validation rules:
///
/// **[`Range<T>`] (exclusive end)**: The end value is NOT included in the range. This
/// creates a special case: a range like `0..10` is valid for content of length 10, even
/// though index 10 itself would be out of bounds for direct content access. The end can
/// equal the content length (cursor EOL semantics).
///
/// **[`RangeInclusive<T>`] (inclusive end)**: The end value IS included in the range.
/// For content of length 10 (indices 0-9), a valid inclusive range must have `end < 10`
/// (e.g., `0..=9`). If `end == 10`, it would try to access an invalid index. Both
/// endpoints must be valid array indices.
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
/// **Exclusive Range** - To process columns 5-9, use Range `5..10`:
/// ```text
///                               ╭─── Range 5..10 ───╮
///                               │    (exclusive)    │
///                               ▼                   ▼
/// Column:   0   1   2   3   4   5   6   7   8   9   10
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ X │ X │ X │ X │ X │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑                   ↑
///                            start=5            end=10 (NOT included)
///
/// end=10 equals length - VALID for exclusive ranges (cursor EOL)
/// ```
///
/// **Inclusive Range** - To process columns 5-9, use `RangeInclusive` `5..=9`:
/// ```text
///                               ╭─RangeInclusive╮
///                               │ 5..=9         │
///                               │ (inclusive)   │
///                               ▼               ▼
/// Column:   0   1   2   3   4   5   6   7   8   9   10
///         ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///         │ A │ B │ C │ D │ E │ X │ X │ X │ X │ X │ ! │
///         └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                               ↑               ↑
///                            start=5        end=9 (IS included)
///
/// end=9 < length - VALID (both endpoints are accessible indices)
/// end=10 would be INVALID (index 10 is out of bounds for array access)
/// ```
///
/// ## Core Methods
///
/// - [`is_valid()`] - Check if range is structurally valid and within content bounds
/// - [`clamp_range_to()`] - Ensure range fits within content bounds while preserving
///   semantics
///
/// These methods automatically handle the semantic differences:
/// - **Exclusive ([`Range`])**: End can equal content length (cursor EOL semantics)
/// - **Inclusive ([`RangeInclusive`])**: End must be < content length (array access
///   semantics)
///
/// [`is_valid()`]: RangeBoundsCheck::is_valid
/// [`clamp_range_to()`]: RangeBoundsCheck::clamp_range_to
pub trait RangeBoundsCheck {
    /// The index type contained in this range.
    type IndexType: PartialOrd + Copy;

    /// The length type that this range can be validated against.
    type LengthType: CursorBoundsCheck;

    /// Check if this range is valid for the given buffer/line length.
    ///
    /// Returns `true` if:
    /// - The range is not inverted (start <= end) - empty ranges are valid
    /// - The start position is within buffer bounds (< length)
    /// - The end position is valid for EOL cursor placement (<= length)
    ///
    /// ## Validation Rules
    ///
    /// 1. Non-inverted: `start <= end` (empty ranges like `5..5` are valid)
    /// 2. Start within bounds: `start < content.length()` for content access
    /// 3. End allows exclusive semantics: `end <= content.length()` for iteration
    ///
    /// The key distinction is that while `start` must be a valid content index,
    /// `end` can equal the content length due to exclusive range semantics.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RangeBoundsCheck, ColIndex, ColWidth, col, width};
    ///
    /// let content_width = width(10);  // Content has 10 columns (0-9)
    ///
    /// // Valid ranges:
    /// let range1: Range<ColIndex> = col(0)..col(5);   // columns 0-4
    /// let range2: Range<ColIndex> = col(5)..col(10);  // columns 5-9 (end=10 is valid!)
    /// let range3: Range<ColIndex> = col(0)..col(10);  // entire content
    /// let empty: Range<ColIndex> = col(5)..col(5);    // empty range (valid)
    ///
    /// assert!(range1.is_valid(content_width));
    /// assert!(range2.is_valid(content_width));  // end=10 <= len=10 ✓
    /// assert!(range3.is_valid(content_width));
    /// assert!(empty.is_valid(content_width));
    ///
    /// // Invalid ranges:
    /// let bad_range1: Range<ColIndex> = col(5)..col(11);  // end > len
    /// let bad_range2: Range<ColIndex> = col(8)..col(3);   // start > end
    /// let bad_range3: Range<ColIndex> = col(15)..col(20); // start beyond bounds
    ///
    /// assert!(!bad_range1.is_valid(content_width));
    /// assert!(!bad_range2.is_valid(content_width));
    /// assert!(!bad_range3.is_valid(content_width));
    /// ```
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
    /// ## Clamping Behavior
    ///
    /// - Start clamping: Start is clamped to `[0, length)` for valid content access
    /// - End clamping: End is clamped to `[start, length]` (note: end can equal length)
    /// - Empty ranges: Preserved as empty when both start and end are clamped to same
    ///   value
    /// - Invalid ranges: Converted to valid empty ranges at the beginning
    ///
    /// ## Edge Case Handling
    ///
    /// - Start beyond bounds: Returns empty range `0..0`
    /// - Inverted ranges: Corrected to empty range at start position
    /// - End beyond bounds: Clamped to content length (exclusive end semantics)
    /// - Both beyond bounds: Returns empty range `0..0`
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use std::ops::Range;
    /// use r3bl_tui::{RangeBoundsCheck, ColIndex, ColWidth, col, width};
    ///
    /// let content_width = width(10); // Content has columns 0-9
    ///
    /// // Normal ranges within bounds - unchanged
    /// let normal: Range<ColIndex> = col(2)..col(7);
    /// assert_eq!(normal.clamp_range_to(content_width), col(2)..col(7));
    ///
    /// // Range to content end - unchanged (end=length is valid)
    /// let to_end: Range<ColIndex> = col(5)..col(10);
    /// assert_eq!(to_end.clamp_range_to(content_width), col(5)..col(10));
    ///
    /// // Range with end beyond bounds - end clamped
    /// let beyond_end: Range<ColIndex> = col(5)..col(15);
    /// assert_eq!(beyond_end.clamp_range_to(content_width), col(5)..col(10));
    ///
    /// // Range with start beyond bounds - becomes empty at start
    /// let beyond_start: Range<ColIndex> = col(15)..col(20);
    /// assert_eq!(beyond_start.clamp_range_to(content_width), col(0)..col(0));
    ///
    /// // Inverted range - becomes empty at start position
    /// let inverted: Range<ColIndex> = col(8)..col(3);
    /// assert_eq!(inverted.clamp_range_to(content_width), col(8)..col(8));
    /// ```
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

    /// Checks if an index is within this range's bounds.
    ///
    /// Returns a three-state result indicating whether the index is below, within, or
    /// above the range boundaries. The boundary semantics (inclusive vs exclusive)
    /// are determined by the range type:
    /// - [`Range<I>`] uses exclusive end: `[start, end)`
    /// - [`RangeInclusive<I>`] uses inclusive end: `[start, end]`
    ///
    /// # Returns
    /// - [`RangeBoundsResult::Underflowed`] if index < range.start
    /// - [`RangeBoundsResult::Within`] if index is within range bounds
    /// - [`RangeBoundsResult::Overflowed`] if index is beyond range bounds
    ///
    /// # Examples
    ///
    /// **Exclusive Range** (`Range<I>`) - end NOT included:
    /// ```rust
    /// use r3bl_tui::{col, RangeBoundsCheck, RangeBoundsResult};
    ///
    /// let range = col(3)..col(8);  // [3, 8) - exclusive end
    ///
    /// assert_eq!(range.check_index_is_within(col(2)), RangeBoundsResult::Underflowed);
    /// assert_eq!(range.check_index_is_within(col(3)), RangeBoundsResult::Within);
    /// assert_eq!(range.check_index_is_within(col(7)), RangeBoundsResult::Within);
    /// assert_eq!(range.check_index_is_within(col(8)), RangeBoundsResult::Overflowed);  // 8 excluded!
    /// ```
    ///
    /// **Inclusive Range** (`RangeInclusive<I>`) - both ends included:
    /// ```rust
    /// use r3bl_tui::{row, RangeBoundsCheck, RangeBoundsResult};
    ///
    /// let scroll_region = row(2)..=row(5);  // [2, 5] - inclusive end
    ///
    /// assert_eq!(scroll_region.check_index_is_within(row(1)), RangeBoundsResult::Underflowed);
    /// assert_eq!(scroll_region.check_index_is_within(row(2)), RangeBoundsResult::Within);
    /// assert_eq!(scroll_region.check_index_is_within(row(5)), RangeBoundsResult::Within);  // 5 included!
    /// assert_eq!(scroll_region.check_index_is_within(row(6)), RangeBoundsResult::Overflowed);
    /// ```
    ///
    /// [`Range<I>`]: std::ops::Range
    fn check_index_is_within(&self, index: Self::IndexType) -> RangeBoundsResult;
}

/// Implementation of range operations for `Range<IndexType>`.
///
/// This provides type-safe validation and clamping for ranges of any index type
/// (`ColIndex`, `RowIndex`, etc.) against their corresponding length types.
///
/// The implementation handles the critical distinction between:
/// - Array bounds checking for the start index (must be valid for content access)
/// - Cursor position bounds checking for the end index (can equal content length)
///
/// This dual approach ensures that ranges are valid for iteration while respecting
/// exclusive end semantics.
impl<I> RangeBoundsCheck for Range<I>
where
    I: IndexMarker + ArrayBoundsCheck<I::LengthType> + Add<Output = I>,
{
    type IndexType = I;
    type LengthType = I::LengthType;

    fn is_valid(&self, buffer_length: impl Into<Self::LengthType>) -> bool {
        let length = buffer_length.into();

        // Check for inverted ranges (start > end).
        if self.start > self.end {
            return false;
        }

        // Start must be within bounds (standard index check).
        if self.start.overflows(length) != ArrayOverflowResult::Within {
            return false;
        }

        // End can be equal to length for exclusive ranges (special case).
        // Use CursorPositionBoundsStatus to handle this correctly.
        length.check_cursor_position_bounds(self.end)
            != CursorPositionBoundsStatus::Beyond
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

/// Implementation of range operations for `RangeInclusive<IndexType>`.
///
/// This provides type-safe validation and clamping for inclusive ranges of any index type
/// (`ColIndex`, `RowIndex`, etc.) against their corresponding length types.
///
/// The implementation enforces that BOTH start and end must be valid array indices:
/// - Array bounds checking for the start index (must be < length)
/// - Array bounds checking for the end index (must be < length, NOT <= like exclusive
///   ranges)
///
/// This ensures that inclusive ranges can safely access all elements from start to end,
/// since both endpoints are included in the range.
impl<I> RangeBoundsCheck for RangeInclusive<I>
where
    I: IndexMarker + ArrayBoundsCheck<I::LengthType> + Add<Output = I>,
{
    type IndexType = I;
    type LengthType = I::LengthType;

    fn is_valid(&self, buffer_length: impl Into<Self::LengthType>) -> bool {
        let length = buffer_length.into();

        // Check for inverted ranges (start > end).
        if self.start() > self.end() {
            return false;
        }

        // Start must be within bounds (standard index check).
        if self.start().overflows(length) != ArrayOverflowResult::Within {
            return false;
        }

        // IMPORTANT: For inclusive ranges, the end is INCLUDED, so it must also be
        // a valid array index (< length), not just a valid cursor position (<= length).
        // This is the key difference from exclusive Range validation.
        self.end().overflows(length) == ArrayOverflowResult::Within
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

/// Extension trait for converting between range types.
///
/// This trait provides conversion methods for range types used in bounds checking,
/// particularly for converting between VT-100's inclusive scroll region semantics
/// and Rust's exclusive iteration semantics.
///
/// # Motivation
///
/// VT-100 terminal scroll regions use inclusive ranges where both endpoints are
/// valid row positions (e.g., `2..=5` means rows 2,3,4,5 are all in the region).
/// However, Rust's slice operations and iteration use exclusive ranges where the
/// end is NOT included (e.g., `2..6` processes indices 2,3,4,5).
///
/// This trait provides explicit, type-safe conversion between these two semantics.
///
/// # Why Extension Trait?
///
/// We cannot add inherent methods to `std::ops::RangeInclusive` (orphan rule),
/// so we use an extension trait that can be implemented on foreign types.
///
/// ## Usage
///
/// To use the conversion methods, you must import this trait:
///
/// ```rust
/// use r3bl_tui::{row, RangeConversion};
///
/// let scroll_region = row(2)..=row(5);
/// let iter_range = scroll_region.to_exclusive();
/// ```
pub trait RangeConversion {
    /// The index type contained in this range.
    type IndexType: IndexMarker;

    /// Convert an inclusive range to an exclusive range for iteration.
    ///
    /// This is useful when you have an inclusive range (e.g., VT-100 scroll regions
    /// where both endpoints are valid positions) but need an exclusive range for
    /// Rust iteration or slice operations.
    ///
    /// # Semantic Translation
    ///
    /// - **Input** (inclusive): `start..=end` - both start and end are included
    /// - **Output** (exclusive): `start..(end+1)` - end+1 is NOT included
    /// - **Effect**: Both represent the same set of indices for iteration
    ///
    /// # Common Mistake: NOT the Same as Direct Exclusive Range
    ///
    /// **IMPORTANT**: `(a..=b).to_exclusive()` is **NOT** the same as `(a..b)`!
    ///
    /// ```text
    /// Given: row_index = row(2), scroll_bottom = row(5)
    ///
    /// ✓ CORRECT:
    /// (row(2)..=row(5)).to_exclusive()
    ///     → Inclusive: row(2), row(3), row(4), row(5)
    ///     → Exclusive: row(2)..row(6)
    ///     → Processes: row(2), row(3), row(4), row(5) ✓
    ///
    /// ✗ WRONG:
    /// (row(2)..row(5))
    ///     → Exclusive: row(2)..row(5)
    ///     → Processes: row(2), row(3), row(4) only!
    ///     → MISSING: row(5) - BUG! ✗
    /// ```
    ///
    /// The key difference:
    /// - **`(2..=5).to_exclusive()`** converts by adding 1 to end: `2..6`
    /// - **`(2..5)`** directly uses 5 as the exclusive end (5 is NOT included)
    /// - Direct exclusive `(2..5)` would **exclude the last element** (row 5)
    ///
    /// In VT-100 context, `scroll_bottom` represents the **last valid row** in the
    /// scroll region (inclusive), so you need `to_exclusive()` to convert it properly
    /// for Rust's iteration semantics. Using `(row_index..scroll_bottom)` would
    /// incorrectly exclude the `scroll_bottom` row from the operation.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use r3bl_tui::{row, RangeConversion};
    ///
    /// // VT-100 scroll region: rows 2,3,4,5 (inclusive)
    /// let scroll_region = row(2)..=row(5);
    ///
    /// // Convert for Rust iteration: row(2)..row(6) (exclusive)
    /// let iteration_range = scroll_region.to_exclusive();
    ///
    /// // Verify conversion
    /// assert_eq!(iteration_range.start, row(2));
    /// assert_eq!(iteration_range.end, row(6));
    /// ```
    ///
    /// # VT-100 Scroll Region Example
    ///
    /// ```rust
    /// use r3bl_tui::{row, len, RangeConversion};
    ///
    /// // VT-100 scroll region from row 1 to row 4 (both inclusive)
    /// let scroll_region = row(1)..=row(4);
    ///
    /// // Need to shift lines within this region
    /// // buffer.shift_lines_up(scroll_region.to_exclusive(), len(1));
    /// ```
    #[must_use]
    fn to_exclusive(self) -> Range<Self::IndexType>;
}

/// Implementation for `RangeInclusive<I>` - the primary use case.
///
/// This implementation converts VT-100 style inclusive ranges (where both endpoints
/// are valid positions) to Rust's exclusive ranges (where the end is not included)
/// for use with iteration and slice operations.
impl<I> RangeConversion for RangeInclusive<I>
where
    I: IndexMarker + Add<Output = I>,
{
    type IndexType = I;

    fn to_exclusive(self) -> Range<I> {
        let start = *self.start();
        let end = *self.end() + I::from(1u16);
        start..end
    }
}

#[cfg(test)]
mod tests {
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

        assert!(
            range1.is_valid(content_width),
            "Range 0..5 should be valid for width 10"
        );
        assert!(
            range2.is_valid(content_width),
            "Range 5..10 should be valid for width 10 (exclusive end)"
        );
        assert!(
            range3.is_valid(content_width),
            "Range 0..10 should be valid for width 10 (full content)"
        );
        assert!(
            range4.is_valid(content_width),
            "Range 7..9 should be valid for width 10"
        );
        assert!(
            empty_range.is_valid(content_width),
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

        assert!(
            !bad_range1.is_valid(content_width),
            "Range 5..11 should be invalid for width 10"
        );
        assert!(
            !bad_range2.is_valid(content_width),
            "Range 0..11 should be invalid for width 10"
        );
        assert!(
            !bad_range3.is_valid(content_width),
            "Range 15..20 should be invalid for width 10"
        );

        // Invalid ranges - inverted
        let inverted_range: Range<Index> = idx(8)..idx(3); // inverted range
        assert!(
            !inverted_range.is_valid(content_width),
            "Inverted range 8..3 should be invalid"
        );
    }

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

    // ========================================================================
    // Tests for RangeInclusive<I> - Inclusive end semantics
    // ========================================================================

    #[test]
    fn test_range_inclusive_validation_valid_ranges() {
        let content_width = len(10); // Columns 0-9

        // Valid inclusive ranges - end must be < length (not <= like exclusive)
        let range1: RangeInclusive<Index> = idx(0)..=idx(4); // columns 0-4
        let range2: RangeInclusive<Index> = idx(5)..=idx(9); // columns 5-9
        let range3: RangeInclusive<Index> = idx(0)..=idx(9); // entire content
        let range4: RangeInclusive<Index> = idx(7)..=idx(9); // end range
        let single: RangeInclusive<Index> = idx(5)..=idx(5); // single element

        assert!(
            range1.is_valid(content_width),
            "Range 0..=4 should be valid for width 10"
        );
        assert!(
            range2.is_valid(content_width),
            "Range 5..=9 should be valid for width 10"
        );
        assert!(
            range3.is_valid(content_width),
            "Range 0..=9 should be valid for width 10 (full content)"
        );
        assert!(
            range4.is_valid(content_width),
            "Range 7..=9 should be valid for width 10"
        );
        assert!(
            single.is_valid(content_width),
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

        assert!(
            !bad_range1.is_valid(content_width),
            "Range 5..=10 should be INVALID for width 10 (inclusive ranges: end must be < length)"
        );
        assert!(
            !bad_range2.is_valid(content_width),
            "Range 0..=10 should be INVALID for width 10 (inclusive ranges: end must be < length)"
        );
        assert!(
            !bad_range3.is_valid(content_width),
            "Range 0..=11 should be invalid for width 10"
        );
        assert!(
            !bad_range4.is_valid(content_width),
            "Range 15..=20 should be invalid for width 10"
        );

        // Invalid ranges - inverted
        let inverted: RangeInclusive<Index> = idx(8)..=idx(3);
        assert!(
            !inverted.is_valid(content_width),
            "Inverted range 8..=3 should be invalid"
        );
    }

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
    fn test_range_inclusive_semantic_difference() {
        let content_width = len(10); // Columns 0-9, length 10

        // CRITICAL: Demonstrate the key semantic difference

        // Exclusive range 5..10 is VALID (end=10 is OK for exclusive)
        let exclusive: Range<Index> = idx(5)..idx(10);
        assert!(
            exclusive.is_valid(content_width),
            "Exclusive range 5..10 should be VALID (end can equal length)"
        );

        // Inclusive range 5..=10 is INVALID (end=10 would access out-of-bounds index)
        let inclusive: RangeInclusive<Index> = idx(5)..=idx(10);
        assert!(
            !inclusive.is_valid(content_width),
            "Inclusive range 5..=10 should be INVALID (end must be < length)"
        );

        // The correct inclusive range for columns 5-9 is 5..=9
        let correct_inclusive: RangeInclusive<Index> = idx(5)..=idx(9);
        assert!(
            correct_inclusive.is_valid(content_width),
            "Inclusive range 5..=9 should be VALID (end < length)"
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
mod range_conversion_tests {
    #[test]
    fn test_range_conversion_inclusive_to_exclusive() {
        use crate::{RangeConversion, row};

        // VT-100 scroll region: rows 2,3,4,5 (inclusive)
        let inclusive = row(2)..=row(5);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(2));
        assert_eq!(exclusive.end, row(6)); // end+1 for exclusive semantics
    }

    #[test]
    fn test_range_conversion_single_element() {
        use crate::{RangeConversion, row};

        // Single row region
        let inclusive = row(3)..=row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(3));
        assert_eq!(exclusive.end, row(4));
    }

    #[test]
    fn test_range_conversion_vt100_scroll_region() {
        use crate::{RangeConversion, row};

        // Simulate VT-100 scroll region lines 1-4 (both inclusive)
        let scroll_region = row(1)..=row(4);
        let iter_range = scroll_region.to_exclusive();

        // Should convert to 1..5 for iteration
        assert_eq!(iter_range.start, row(1));
        assert_eq!(iter_range.end, row(5));
    }

    #[test]
    fn test_range_conversion_zero_based() {
        use crate::{RangeConversion, row};

        // Range starting from 0
        let inclusive = row(0)..=row(3);
        let exclusive = inclusive.to_exclusive();

        assert_eq!(exclusive.start, row(0));
        assert_eq!(exclusive.end, row(4));
    }
}

#[cfg(test)]
mod range_membership_tests {
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
