// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{length_marker::LengthMarker,
            result_enums::{ArrayOverflowResult, ArrayUnderflowResult}};
use crate::UnitMarker;

/// Array-style bounds checking for safe element access.
///
/// Provides bounds checking specifically for array access patterns where an index
/// must be less than the container length to be valid for correct element access.
///
/// # Core Purpose
/// Use case: "Can I access `array[index]` correctly?"
///
/// This is the traditional bounds checking pattern used throughout programming:
/// - Array element access: `buffer[index]` requires `index < buffer.len()`
/// - Vector operations: `vec[index]` requires `index < vec.len()`
/// - Buffer operations: Index bounds validation for correct access
///
/// # Key Distinction from Other Bounds Traits
///
/// | Trait                         | Rule                          | Use Case      | Example                                              |
/// |-------------------------------|-------------------------------|---------------|------------------------------------------------------|
/// | `ArrayBoundsCheck`📍          | `index < length`              | Index safety  | `buffer[5]` needs `5 < buffer.len()`                 |
/// | [`CursorBoundsCheck`]         | `index <= length`             | Text editing  | Cursor can be at position `length` (after last char) |
/// | [`ViewportBoundsCheck`]       | `start <= index < start+size` | Rendering     | Content visibility in windows                        |
/// | [`RangeBoundsCheck`]          | `start <= end <= length`      | Iteration     | Range object structural validation                   |
///
/// # Array Access Semantics
/// Array bounds checking uses strict inequality because array elements are indexed
/// from 0 to length-1:
///
/// ```text
/// Array with length=10:
///                                                   ╭─ boundary
///                                                   │ ╭─ ERR ─╮
/// Index:     0   1   2   3   4   5   6   7   8   9  │ 10  11  12
/// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
/// Access:   │ W │ W │ W │ W │ W │ W │ W │ W │ W │ W │ O │ O │ O │
///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
///           ├────────── valid indices ──────────────┼─ overflow ┘
///           └─────────── length=10 ─────────────────┘
///                        (1-based)
///
/// W: within bounds (valid access)
/// O: out of bounds (invalid access)
///
/// Valid: array[0] through array[9]
/// Invalid: array[10] and beyond (index >= length)
/// ```
///
/// # Relationship to Index Operations
/// This trait provides the core bounds checking methods used by index types.
/// Index types (from [`IndexMarker`]) call these methods to validate their positions
/// against container boundaries.
///
/// # Safety Guarantees
/// The bounds checking in this trait prevents:
/// - Buffer overruns: Accessing memory beyond allocated boundaries
/// - Segmentation faults: Invalid memory access in unsafe code
/// - Array index panics: Out-of-bounds access in safe Rust code
/// - Data corruption: Unintended writes to invalid memory locations
///
/// # Type Safety
/// This trait is generic over length types that implement [`LengthMarker`],
/// and can only be implemented by index types that implement [`IndexMarker`].
/// This ensures type safety and prevents incorrect comparisons between incompatible
/// types.
///
/// # Examples
/// ```
/// use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, RowIndex, RowHeight};
///
/// let row_index = RowIndex::new(5);
/// let height = RowHeight::new(5);
/// assert_eq!(row_index.overflows(height), ArrayOverflowResult::Overflowed);
/// ```
///
/// # See Also
/// - [`IndexMarker`] - Index types that use these bounds checking methods
/// - [`CursorBoundsCheck`] - Cursor positioning with different boundary rules
/// - [`ViewportBoundsCheck`] - Viewport visibility with exclusive upper bounds
/// - [`RangeBoundsCheck`] - Range validation for iteration and algorithms
/// - [Module documentation] - Overview of the complete bounds checking architecture
///
/// [Module documentation]: mod@crate::core::units::bounds_check
/// [`IndexMarker`]: crate::IndexMarker
/// [`LengthMarker`]: crate::LengthMarker
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`RangeBoundsCheck`]: crate::RangeBoundsCheck
/// [`ArrayBoundsCheck`]: crate::ArrayBoundsCheck
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
pub trait ArrayBoundsCheck<LengthType: LengthMarker>
where
    Self: UnitMarker,
{
    /// Performs comprehensive array bounds checking `[0, length)`.
    ///
    /// > <div class="warning">
    /// >
    /// > Read [Interval Notation] in [Module documentation] for explanation of
    /// > mathematical interval notation used in this method to describe exclusive bounds.
    /// >
    /// > </div>
    ///
    /// ```text
    /// Array-style bounds checking:
    ///
    ///                           index=5 (0-based)   index=10 (0-based)
    ///                                 ↓                   ↓
    /// Index:      0   1   2   3   4   5   6   7   8   9 │ 10  11  12
    /// (0-based) ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┼───┬───┬───┐
    ///           │ W │ W │ W │ W │ W │ W │ W │ W │ W │ W │ O │ O │ O │
    ///           ├───┴───┴───┴───┴───┴───┴───┴───┴───┴───┼───┴───┴───┤
    ///           ├─────────── within bounds ─────────────┼─ overflow ┘
    ///           └────────── length=10 (1-based) ────────┘
    ///
    /// W: within bounds (valid access)
    /// O: out of bounds (invalid access)
    ///
    /// overflows(length=10) for index=5  = Within
    /// overflows(length=10) for index=10 = Overflowed
    /// ```
    ///
    /// # Returns
    /// - [`ArrayOverflowResult::Within`] if the index can safely access content,
    /// - [`ArrayOverflowResult::Overflowed`] if the index would exceed array bounds.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, col, width};
    ///
    /// let index = col(10);
    /// let buffer_width = width(10);
    ///
    /// // Guard clause pattern
    /// if index.overflows(buffer_width) == ArrayOverflowResult::Overflowed {
    ///     return;
    /// }
    ///
    /// // Pattern matching
    /// match index.overflows(buffer_width) {
    ///     ArrayOverflowResult::Within => { /* safe */ }
    ///     ArrayOverflowResult::Overflowed => { /* overflow */ }
    /// }
    ///
    /// // Safe array access
    /// let chars = vec!['a'; 5];
    /// let idx = col(3);
    /// if idx.overflows(width(chars.len())) == ArrayOverflowResult::Within {
    ///     let ch = chars[idx.as_usize()];
    /// }
    /// ```
    ///
    /// [Interval Notation]: mod@crate::core::units::bounds_check#interval-notation
    /// [Module documentation]: mod@crate::core::units::bounds_check
    fn overflows(&self, arg_length: impl Into<LengthType>) -> ArrayOverflowResult
    where
        LengthType: LengthMarker<IndexType = Self>,
    {
        let length: LengthType = arg_length.into();
        // Special case: empty collection (length 0) has no valid indices.
        if length.is_zero() {
            return ArrayOverflowResult::Overflowed;
        }
        if *self > length.convert_to_index() {
            ArrayOverflowResult::Overflowed
        } else {
            ArrayOverflowResult::Within
        }
    }

    /// Checks if this index underflows (goes below) the given minimum bound.
    ///
    /// ```text
    /// Underflow bounds checking:
    ///
    ///                  min=3 (inclusive)
    ///                      ↓
    /// Index:   0   1   2   3   4   5   6   7   8   9   10  11  12
    ///        ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
    ///        │ U │ U │ U │ W │ W │ W │ W │ W │ W │ W │ W │ W │ W │
    ///        ├───┴───┴───┼───┴───┴───┴───┴───┴───┴───┴───┴───┴───┤
    ///        ├─underflow─┼────────── within bounds ──────────────┤
    ///        └───────────┴───────────────────────────────────────┘
    ///
    /// U: underflowed (index < min)
    /// W: within bounds (index ≥ min)  ← Minimum is INCLUDED
    ///
    /// underflows(min=3) for index=0 = Underflowed
    /// underflows(min=3) for index=2 = Underflowed
    /// underflows(min=3) for index=3 = Within  ← Boundary included!
    /// underflows(min=3) for index=5 = Within
    /// ```
    ///
    /// # Returns
    /// - [`ArrayUnderflowResult::Within`] if the index is at or above the minimum bound,
    /// - [`ArrayUnderflowResult::Underflowed`] if the index is below the minimum bound.
    ///
    /// # Examples
    /// ```
    /// use r3bl_tui::{ArrayBoundsCheck, ArrayUnderflowResult, col};
    ///
    /// let min_col = col(3);
    ///
    /// // Boundary checking
    /// assert_eq!(col(0).underflows(min_col), ArrayUnderflowResult::Underflowed);  // below minimum
    /// assert_eq!(col(2).underflows(min_col), ArrayUnderflowResult::Underflowed);  // still below
    /// assert_eq!(col(3).underflows(min_col), ArrayUnderflowResult::Within); // at boundary (valid)
    /// assert_eq!(col(5).underflows(min_col), ArrayUnderflowResult::Within); // above minimum
    /// ```
    fn underflows(&self, min_bound: impl Into<Self>) -> ArrayUnderflowResult {
        let min: Self = min_bound.into();
        if *self < min {
            ArrayUnderflowResult::Underflowed
        } else {
            ArrayUnderflowResult::Within
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColIndex, ColWidth, RowHeight, RowIndex, idx, len};

    /// Comprehensive tests to ensure consistency between all bounds checking methods:
    /// - `overflows()`
    /// - `is_overflowed_by()`
    #[test]
    fn test_bounds_checking_consistency() {
        // Test critical boundary cases with generic Index/Length.
        let test_cases = [
            // (index, length, expected_overflows).
            (0, 1, false), // First valid index
            (0, 5, false), // First valid index in larger array
            (4, 5, false), // Last valid index (length-1)
            (5, 5, true),  // First invalid index (length)
            (6, 5, true),  // Beyond bounds
            (0, 0, true),  // Empty collection edge case
            (1, 0, true),  // Index in empty collection
        ];

        for (index_val, length_val, expected_overflows) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);

            let expected_status = if expected_overflows {
                ArrayOverflowResult::Overflowed
            } else {
                ArrayOverflowResult::Within
            };

            // Test overflows() method.
            let overflows_result = index.overflows(length);
            assert_eq!(
                overflows_result, expected_status,
                "overflows mismatch for idx({index_val}).overflows(len({length_val}))"
            );

            // Test is_overflowed_by() method (same operation from length perspective).
            let is_overflowed_result = length.is_overflowed_by(index);
            assert_eq!(
                is_overflowed_result, overflows_result,
                "is_overflowed_by mismatch for len({length_val}).is_overflowed_by(idx({index_val}))"
            );

            // Test overflows() consistency (same as check_array_access_bounds).
            let bounds_status = index.overflows(length);
            assert_eq!(
                bounds_status, expected_status,
                "overflows mismatch for idx({index_val}).overflows(len({length_val}))"
            );
        }
    }

    #[test]
    fn test_typed_bounds_checking_consistency() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Test with ColIndex/ColWidth
        let col_cases = [
            (0, 3, false), // First valid
            (2, 3, false), // Last valid
            (3, 3, true),  // First invalid
            (0, 0, true),  // Empty
        ];

        for (index_val, width_val, expected_overflows) in col_cases {
            let col_index = ColIndex::new(index_val);
            let col_width = ColWidth::new(width_val);

            let expected_status = if expected_overflows {
                ArrayOverflowResult::Overflowed
            } else {
                ArrayOverflowResult::Within
            };

            let overflows_result = col_index.overflows(col_width);
            let is_overflowed_result = col_width.is_overflowed_by(col_index);

            assert_eq!(
                overflows_result, expected_status,
                "ColIndex overflows mismatch for {index_val}:{width_val}"
            );
            assert_eq!(
                is_overflowed_result, expected_status,
                "ColWidth is_overflowed_by mismatch for {width_val}:{index_val}"
            );
        }

        // Test with RowIndex/RowHeight
        let row_cases = [
            (0, 2, false), // First valid
            (1, 2, false), // Last valid
            (2, 2, true),  // First invalid
        ];

        for (index_val, height_val, expected_overflows) in row_cases {
            let row_index = RowIndex::new(index_val);
            let row_height = RowHeight::new(height_val);

            let expected_status = if expected_overflows {
                ArrayOverflowResult::Overflowed
            } else {
                ArrayOverflowResult::Within
            };

            let overflows_result = row_index.overflows(row_height);
            let is_overflowed_result = row_height.is_overflowed_by(row_index);

            assert_eq!(
                overflows_result, expected_status,
                "RowIndex overflows mismatch for {index_val}:{height_val}"
            );
            assert_eq!(
                is_overflowed_result, expected_status,
                "RowHeight is_overflowed_by mismatch for {height_val}:{index_val}"
            );
        }
    }

    #[test]
    fn test_extreme_values_u16_max() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Test u16::MAX values for bounds checking
        let max_u16 = u16::MAX;

        // Test ColIndex with max values
        let col_index_max = ColIndex::new(max_u16);
        let col_width_max = ColWidth::new(max_u16);

        // Index at u16::MAX should overflow any length
        assert_eq!(
            col_index_max.overflows(col_width_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            col_width_max.is_overflowed_by(col_index_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            col_index_max.overflows(ColWidth::new(100)),
            ArrayOverflowResult::Overflowed
        );

        // Test RowIndex with max values
        let row_index_max = RowIndex::new(max_u16);
        let row_height_max = RowHeight::new(max_u16);

        assert_eq!(
            row_index_max.overflows(row_height_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            row_height_max.is_overflowed_by(row_index_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            row_index_max.overflows(RowHeight::new(100)),
            ArrayOverflowResult::Overflowed
        );

        // Test near-max values
        let col_index_near_max = ColIndex::new(max_u16 - 1);
        assert_eq!(
            col_index_near_max.overflows(ColWidth::new(max_u16 - 1)),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            col_index_near_max.overflows(ColWidth::new(max_u16)),
            ArrayOverflowResult::Within
        );
    }

    #[test]
    fn test_extreme_values_usize() {
        // Test with generic Index/Length using large usize values
        // Using safer values that avoid potential overflow in comparisons
        let large_val = 1_000_000;
        let large_index = idx(large_val);
        let larger_length = len(large_val + 1);
        let equal_length = len(large_val);

        // Index should not overflow when length is larger
        assert_eq!(
            large_index.overflows(larger_length),
            ArrayOverflowResult::Within
        );
        assert_eq!(
            larger_length.is_overflowed_by(large_index),
            ArrayOverflowResult::Within
        );
        assert_eq!(
            large_index.overflows(larger_length),
            ArrayOverflowResult::Within
        );

        // Index should overflow when length equals index (since valid indices are
        // 0..length-1)
        assert_eq!(
            large_index.overflows(equal_length),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            equal_length.is_overflowed_by(large_index),
            ArrayOverflowResult::Overflowed
        );

        // Should definitely overflow smaller length
        let small_length = len(100);
        assert_eq!(
            large_index.overflows(small_length),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            small_length.is_overflowed_by(large_index),
            ArrayOverflowResult::Overflowed
        );
    }

    #[test]
    fn test_zero_length_edge_cases_comprehensive() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Zero-length arrays should reject all indices
        let zero_width = ColWidth::new(0);
        let zero_height = RowHeight::new(0);

        // Test various indices against zero-length
        for i in [0, 1, 10, 100, u16::MAX] {
            let col_idx = ColIndex::new(i);
            let row_idx = RowIndex::new(i);

            assert_eq!(
                col_idx.overflows(zero_width),
                ArrayOverflowResult::Overflowed,
                "Index {i} should overflow zero width"
            );
            assert_eq!(
                zero_width.is_overflowed_by(col_idx),
                ArrayOverflowResult::Overflowed,
                "Zero width should be overflowed by index {i}"
            );
            assert_eq!(
                col_idx.overflows(zero_width),
                ArrayOverflowResult::Overflowed,
                "Index {i} bounds check should fail for zero width"
            );

            assert_eq!(
                row_idx.overflows(zero_height),
                ArrayOverflowResult::Overflowed,
                "Index {i} should overflow zero height"
            );
            assert_eq!(
                zero_height.is_overflowed_by(row_idx),
                ArrayOverflowResult::Overflowed,
                "Zero height should be overflowed by index {i}"
            );
            assert_eq!(
                row_idx.overflows(zero_height),
                ArrayOverflowResult::Overflowed,
                "Index {i} bounds check should fail for zero height"
            );
        }
    }

    #[test]
    fn test_unit_length_edge_cases() {
        use crate::{ColIndex, ColWidth, RowHeight, RowIndex};

        // Unit-length arrays should only accept index 0
        let unit_width = ColWidth::new(1);
        let unit_height = RowHeight::new(1);

        // Index 0 should be valid
        let col_zero = ColIndex::new(0);
        let row_zero = RowIndex::new(0);

        assert_eq!(col_zero.overflows(unit_width), ArrayOverflowResult::Within);
        assert_eq!(
            unit_width.is_overflowed_by(col_zero),
            ArrayOverflowResult::Within
        );

        assert_eq!(row_zero.overflows(unit_height), ArrayOverflowResult::Within);
        assert_eq!(
            unit_height.is_overflowed_by(row_zero),
            ArrayOverflowResult::Within
        );

        // Any index >= 1 should be invalid
        for i in [1, 2, 10, 100] {
            let col_idx = ColIndex::new(i);
            let row_idx = RowIndex::new(i);

            assert_eq!(
                col_idx.overflows(unit_width),
                ArrayOverflowResult::Overflowed,
                "Index {i} should overflow unit width"
            );
            assert_eq!(
                unit_width.is_overflowed_by(col_idx),
                ArrayOverflowResult::Overflowed
            );

            assert_eq!(
                row_idx.overflows(unit_height),
                ArrayOverflowResult::Overflowed,
                "Index {i} should overflow unit height"
            );
            assert_eq!(
                unit_height.is_overflowed_by(row_idx),
                ArrayOverflowResult::Overflowed
            );
        }
    }

    #[test]
    fn test_overflows() {
        // Test basic cases with Index/Length - now returns ArrayAccessResult
        assert_eq!(
            idx(1).overflows(len(3)),
            ArrayOverflowResult::Within,
            "Within bounds"
        );
        assert_eq!(
            idx(3).overflows(len(3)),
            ArrayOverflowResult::Overflowed,
            "At boundary"
        );
        assert_eq!(
            idx(5).overflows(len(3)),
            ArrayOverflowResult::Overflowed,
            "Beyond bounds"
        );
        assert_eq!(
            idx(0).overflows(len(0)),
            ArrayOverflowResult::Overflowed,
            "Empty collection edge case"
        );

        // Test with typed dimensions.
        assert_eq!(
            ColIndex::new(5).overflows(ColWidth::new(10)),
            ArrayOverflowResult::Within,
            "Typed indices within bounds"
        );
        assert_eq!(
            ColIndex::new(10).overflows(ColWidth::new(10)),
            ArrayOverflowResult::Overflowed,
            "Typed indices at boundary"
        );
        assert_eq!(
            RowIndex::new(3).overflows(RowHeight::new(5)),
            ArrayOverflowResult::Within,
            "Row indices within bounds"
        );
        assert_eq!(
            RowIndex::new(5).overflows(RowHeight::new(5)),
            ArrayOverflowResult::Overflowed,
            "Row indices at boundary"
        );

        // Verify method matches is_overflowed_by behavior (converts enum to bool for
        // comparison)
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = idx(index_val);
            let length = len(length_val);
            let overflows_result = index.overflows(length);
            assert_eq!(
                overflows_result,
                length.is_overflowed_by(index),
                "overflows() should match is_overflowed_by() for index {index_val} and length {length_val}"
            );
        }

        // Test with specific typed combinations.
        let col_cases = [(0, 5), (4, 5), (5, 5), (6, 5)];
        for (index_val, width_val) in col_cases {
            let col_index = ColIndex::new(index_val);
            let col_width = ColWidth::new(width_val);
            let overflows_result = col_index.overflows(col_width);
            assert_eq!(
                overflows_result,
                col_width.is_overflowed_by(col_index),
                "ColIndex::overflows should match ColWidth::is_overflowed_by for index {index_val} and width {width_val}"
            );
        }

        let row_cases = [(0, 3), (2, 3), (3, 3), (4, 3)];
        for (index_val, height_val) in row_cases {
            let row_index = RowIndex::new(index_val);
            let row_height = RowHeight::new(height_val);
            let overflows_result = row_index.overflows(row_height);
            assert_eq!(
                overflows_result,
                row_height.is_overflowed_by(row_index),
                "RowIndex::overflows should match RowHeight::is_overflowed_by for index {index_val} and height {height_val}"
            );
        }
    }

    #[test]
    fn test_underflows_method() {
        use crate::RowIndex;

        let min_row = RowIndex::new(3);

        // Test underflow cases
        assert_eq!(
            RowIndex::new(0).underflows(min_row),
            ArrayUnderflowResult::Underflowed,
            "Row 0 should underflow min 3"
        );
        assert_eq!(
            RowIndex::new(2).underflows(min_row),
            ArrayUnderflowResult::Underflowed,
            "Row 2 should underflow min 3"
        );

        // Test at boundary
        assert_eq!(
            RowIndex::new(3).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 3 should not underflow min 3"
        );

        // Test above boundary
        assert_eq!(
            RowIndex::new(5).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 5 should not underflow min 3"
        );
        assert_eq!(
            RowIndex::new(10).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 10 should not underflow min 3"
        );
    }
}
