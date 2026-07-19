// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Safe array element access validation - see [`ArrayBoundsCheck`] trait.

use super::{length_ops::LengthOps,
            result_enums::{ArrayOverflowResult, ArrayUnderflowResult}};
use crate::NumericValue;

/// Trait for 0-based position/index types, providing operations to validate safe array
/// element access.
///
/// This trait provides bounds checking operations for any index type that needs to
/// validate safe array element access. It ensures that indices are within valid bounds
/// for accessing array/buffer elements where the rule is `index < length`.
///
/// # Purpose
///
/// This trait answers the fundamental question: **"Can I access `array[index]` safely?"**
///
/// This is the traditional bounds checking pattern used throughout programming for
/// preventing buffer overruns, segmentation faults, and out-of-bounds panics.
///
/// # Key Trait Capabilities
///
/// - **Overflow checking**: Validate that an index is less than a container's length via
///   [`overflows()`]
/// - **Underflow checking**: Validate that an index is at or above a minimum bound via
///   [`underflows()`]
/// - **Type-safe validation**: Generic over length types that implement [`LengthOps`]
/// - **Dual perspective**: Check from index view ([`index.overflows()`]) or length view
///   ([`length.is_overflowed_by()`])
///
/// # Implementing Types
///
/// The following index types in this codebase implement `ArrayBoundsCheck`:
///
/// - [`VPIndex`] - Generic implementation via blanket impls
/// - [`VPRow`] - Implements `ArrayBoundsCheck<VPHeight>` for vertical bounds checking
/// - [`VPCol`] - Implements `ArrayBoundsCheck<VPWidth>` for horizontal bounds checking
/// - [`SegIndex`] - Implements `ArrayBoundsCheck<SegLength>` for grapheme segment bounds
/// - [`ByteIndex`] - Implements `ArrayBoundsCheck<ByteLength>` for byte-level bounds
///
/// # Array Access Semantics
///
/// Array bounds checking uses strict inequality because array elements are indexed
/// from 0 to length-1:
///
/// ## Overflow Checking (`overflows()` method)
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
/// ## Underflow Checking (`underflows()` method)
/// ```text
/// Checking against minimum bound:
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
/// underflows(min=3) for index=3 = Within  ← Boundary included!
/// ```
///
/// # Key Distinction from Other Bounds Traits
///
/// | Trait                           | Rule                            | Use Case        | Example                                                |
/// | ------------------------------- | ------------------------------- | --------------- | ------------------------------------------------------ |
/// | ➤ `ArrayBoundsCheck`            | `index < length`                | Index safety    | `buffer[5]` needs `5 < buffer.len()`                   |
/// | [`CursorBoundsCheck`]           | `index <= length`               | Text editing    | Cursor can be at position `length` (after last char)   |
/// | [`ViewportBoundsCheck`]         | `start <= index < start+size`   | Rendering       | Content visibility in windows                          |
/// | [`RangeBoundsExt`]              | `start <= end <= length`        | Iteration       | Range object structural validation                     |
///
/// # Safety Guarantees
///
/// The bounds checking in this trait prevents:
/// - Buffer overruns: Accessing memory beyond allocated boundaries
/// - Segmentation faults: Invalid memory access in unsafe code
/// - Array index panics: Out-of-bounds access in safe Rust code
/// - Data corruption: Unintended writes to invalid memory locations
///
/// # Examples
///
/// The `ArrayBoundsCheck` trait provides comprehensive array access validation:
///
/// ```rust
/// use r3bl_tui::{ArrayBoundsCheck, ArrayOverflowResult, vp_col, vp_width};
///
/// let index = vp_col(5);
/// let buffer_width = vp_width(10);
///
/// // Simple equality check - most common pattern
/// if index.overflows(buffer_width) == ArrayOverflowResult::Within {
///     // Safe to access buffer[index]
/// }
///
/// // Pattern matching for detailed handling
/// match index.overflows(buffer_width) {
///     ArrayOverflowResult::Within => { /* safe access */ }
///     ArrayOverflowResult::Overflowed => { /* handle error */ }
/// }
///
/// // Guard clause pattern
/// if index.overflows(buffer_width) == ArrayOverflowResult::Overflowed {
///     return; // Early exit on overflow
/// }
/// // Safe to proceed with buffer[index]
/// ```
///
/// # See Also
///
/// - [`IndexOps`] - Index types that use these bounds checking methods
/// - [`LengthOps`] - Length types used in bounds comparisons
/// - [`NumericValue`] - Base trait that index and length types build upon
/// - [`CursorBoundsCheck`] - Cursor positioning with different boundary rules (`index <=
///   length`)
/// - [`ViewportBoundsCheck`] - Viewport visibility with window-based checking
/// - [`RangeBoundsExt`] - Range validation for iteration and algorithms
///
/// [`ByteIndex`]: crate::ByteIndex
/// [`CursorBoundsCheck`]: crate::CursorBoundsCheck
/// [`index.overflows()`]: ArrayBoundsCheck::overflows
/// [`IndexOps`]: crate::IndexOps
/// [`length.is_overflowed_by()`]: crate::length_ops::LengthOps::is_overflowed_by
/// [`LengthOps`]: crate::LengthOps
/// [`NumericValue`]: crate::NumericValue
/// [`overflows()`]: ArrayBoundsCheck::overflows
/// [`RangeBoundsExt`]: crate::RangeBoundsExt
/// [`SegIndex`]: crate::SegIndex
/// [`underflows()`]: ArrayBoundsCheck::underflows
/// [`ViewportBoundsCheck`]: crate::ViewportBoundsCheck
/// [`VPCol`]: crate::VPCol
/// [`VPIndex`]: crate::VPIndex
/// [`VPRow`]: crate::VPRow
pub trait ArrayBoundsCheck<LengthType: LengthOps>
where
    Self: NumericValue,
{
    /// Checks if this index would overflow when accessing an array of the given length.
    ///
    /// See the [trait documentation][Self] for detailed visual diagrams and array access
    /// semantics.
    ///
    /// # Returns
    /// - [`ArrayOverflowResult::Within`] if `index < length`
    /// - [`ArrayOverflowResult::Overflowed`] if `index >= length`
    fn overflows(&self, arg_length: impl Into<LengthType>) -> ArrayOverflowResult
    where
        LengthType: LengthOps<IndexType = Self>,
    {
        let length: LengthType = arg_length.into();
        // Special case: empty collection (length 0) has no valid indices.
        if length.is_empty() {
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
    /// See the [trait documentation][Self] for underflow checking semantics and visual
    /// diagrams.
    ///
    /// # Returns
    /// - [`ArrayUnderflowResult::Within`] if `index >= min_bound` (inclusive)
    /// - [`ArrayUnderflowResult::Underflowed`] if `index < min_bound`
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
    use crate::{NarrowingCastToU16, VPCol, VPHeight, VPRow, VPWidth, vp_idx, vp_len};

    /// Comprehensive tests to ensure consistency between all bounds checking methods:
    /// - `overflows()`
    /// - `is_overflowed_by()`
    #[test]
    fn test_bounds_checking_consistency() {
        // Test critical boundary cases with generic VPIndex/VPLength.
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
            let index = vp_idx(index_val);
            let length = vp_len(length_val);

            let expected_status = if expected_overflows {
                ArrayOverflowResult::Overflowed
            } else {
                ArrayOverflowResult::Within
            };

            // Test overflows() method.
            let overflows_result = index.overflows(length);
            assert_eq!(
                overflows_result, expected_status,
                "overflows mismatch for vp_idx({index_val}).overflows(vp_len({length_val}))"
            );

            // Test is_overflowed_by() method (same operation from length perspective).
            let is_overflowed_result = length.is_overflowed_by(index);
            assert_eq!(
                is_overflowed_result, overflows_result,
                "is_overflowed_by mismatch for vp_len({length_val}).is_overflowed_by(vp_idx({index_val}))"
            );

            // Test overflows() consistency (same as check_array_access_bounds).
            let bounds_status = index.overflows(length);
            assert_eq!(
                bounds_status, expected_status,
                "overflows mismatch for vp_idx({index_val}).overflows(vp_len({length_val}))"
            );
        }
    }

    #[test]
    fn test_typed_bounds_checking_consistency() {
        use crate::{VPCol, VPHeight, VPRow, VPWidth};

        // Test with ColIndex/ColWidth
        let col_cases = [
            (0, 3, false), // First valid
            (2, 3, false), // Last valid
            (3, 3, true),  // First invalid
            (0, 0, true),  // Empty
        ];

        for (index_val, width_val, expected_overflows) in col_cases {
            let col_index = VPCol::new(index_val);
            let col_width = VPWidth::new(width_val.as_u16_narrowing());

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
            let row_index = VPRow::new(index_val);
            let row_height = VPHeight::new(height_val.as_u16_narrowing());

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
        use crate::{VPCol, VPHeight, VPRow, VPWidth};

        // Test u16::MAX values for bounds checking
        let max_u16 = u16::MAX;

        // Test ColIndex with max values
        let col_index_max = VPCol::new(max_u16);
        let col_width_max = VPWidth::new(max_u16);

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
            col_index_max.overflows(VPWidth::new(100u16)),
            ArrayOverflowResult::Overflowed
        );

        // Test RowIndex with max values
        let row_index_max = VPRow::new(max_u16);
        let row_height_max = VPHeight::new(max_u16);

        assert_eq!(
            row_index_max.overflows(row_height_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            row_height_max.is_overflowed_by(row_index_max),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            row_index_max.overflows(VPHeight::new(100u16)),
            ArrayOverflowResult::Overflowed
        );

        // Test near-max values
        let col_index_near_max = VPCol::new(max_u16 - 1);
        assert_eq!(
            col_index_near_max.overflows(VPWidth::new(max_u16 - 1)),
            ArrayOverflowResult::Overflowed
        );
        assert_eq!(
            col_index_near_max.overflows(VPWidth::new(max_u16)),
            ArrayOverflowResult::Within
        );
    }

    #[test]
    fn test_extreme_values_usize() {
        // Test with generic VPIndex/VPLength using large values
        // Using safer values that avoid potential overflow in comparisons
        let large_val = 60_000u16;
        let large_index = vp_idx(large_val);
        let larger_length = vp_len(large_val + 1);
        let equal_length = vp_len(large_val);

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
        let small_length = vp_len(100);
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
        use crate::{VPCol, VPHeight, VPRow, VPWidth};

        // Zero-length arrays should reject all indices
        let zero_width = VPWidth::new(0u16);
        let zero_height = VPHeight::new(0u16);

        // Test various indices against zero-length
        for i in [0, 1, 10, 100, u16::MAX] {
            let col_idx = VPCol::new(i);
            let row_idx = VPRow::new(i);

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
        use crate::{VPCol, VPHeight, VPRow, VPWidth};

        // Unit-length arrays should only accept index 0
        let unit_width = VPWidth::new(1u16);
        let unit_height = VPHeight::new(1u16);

        // Index 0 should be valid
        let col_zero = VPCol::new(0);
        let row_zero = VPRow::new(0);

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
            let col_idx = VPCol::new(i);
            let row_idx = VPRow::new(i);

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
        // Test basic cases with VPIndex/VPLength - now returns ArrayAccessResult
        assert_eq!(
            vp_idx(1u16).overflows(vp_len(3)),
            ArrayOverflowResult::Within,
            "Within bounds"
        );
        assert_eq!(
            vp_idx(3u16).overflows(vp_len(3)),
            ArrayOverflowResult::Overflowed,
            "At boundary"
        );
        assert_eq!(
            vp_idx(5u16).overflows(vp_len(3)),
            ArrayOverflowResult::Overflowed,
            "Beyond bounds"
        );
        assert_eq!(
            vp_idx(0u16).overflows(vp_len(0)),
            ArrayOverflowResult::Overflowed,
            "Empty collection edge case"
        );

        // Test with typed dimensions.
        assert_eq!(
            VPCol::new(5).overflows(VPWidth::new(10u16)),
            ArrayOverflowResult::Within,
            "Typed indices within bounds"
        );
        assert_eq!(
            VPCol::new(10).overflows(VPWidth::new(10u16)),
            ArrayOverflowResult::Overflowed,
            "Typed indices at boundary"
        );
        assert_eq!(
            VPRow::new(3).overflows(VPHeight::new(5u16)),
            ArrayOverflowResult::Within,
            "Row indices within bounds"
        );
        assert_eq!(
            VPRow::new(5).overflows(VPHeight::new(5u16)),
            ArrayOverflowResult::Overflowed,
            "Row indices at boundary"
        );

        // Verify method matches is_overflowed_by behavior (converts enum to bool for
        // comparison)
        let test_cases = [(0, 1), (1, 1), (5, 10), (10, 10)];
        for (index_val, length_val) in test_cases {
            let index = vp_idx(index_val);
            let length = vp_len(length_val);
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
            let col_index = VPCol::new(index_val);
            let col_width = VPWidth::new(width_val.as_u16_narrowing());
            let overflows_result = col_index.overflows(col_width);
            assert_eq!(
                overflows_result,
                col_width.is_overflowed_by(col_index),
                "ColIndex::overflows should match ColWidth::is_overflowed_by for index {index_val} and width {width_val}"
            );
        }

        let row_cases = [(0, 3), (2, 3), (3, 3), (4, 3)];
        for (index_val, height_val) in row_cases {
            let row_index = VPRow::new(index_val);
            let row_height = VPHeight::new(height_val.as_u16_narrowing());
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
        use crate::VPRow;

        let min_row = VPRow::new(3);

        // Test underflow cases
        assert_eq!(
            VPRow::new(0).underflows(min_row),
            ArrayUnderflowResult::Underflowed,
            "Row 0 should underflow min 3"
        );
        assert_eq!(
            VPRow::new(2).underflows(min_row),
            ArrayUnderflowResult::Underflowed,
            "Row 2 should underflow min 3"
        );

        // Test at boundary
        assert_eq!(
            VPRow::new(3).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 3 should not underflow min 3"
        );

        // Test above boundary
        assert_eq!(
            VPRow::new(5).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 5 should not underflow min 3"
        );
        assert_eq!(
            VPRow::new(10).underflows(min_row),
            ArrayUnderflowResult::Within,
            "Row 10 should not underflow min 3"
        );
    }
}
