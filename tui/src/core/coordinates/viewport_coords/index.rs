// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Zero-based character position for terminal UI - see [`VPIndex`] type.

use super::{VPLength, vp_len};
use crate::{ChUnit, VPCol, VPRow, generate_index_type_impl};
use std::hash::Hash;

/// Represents an index position in character units within a viewport.
///
/// A `VPIndex` is a 0-based measurement that represents a position within a component
/// in the terminal UI, such as a row or column position. It wraps a [`ChUnit`] value.
///
/// `VPIndex` values can be created using the [`VPIndex::new`] method, the [`vp_idx`] /
/// [`vp_index`] functions, or by converting from various numeric types.
///
/// The relationship between [`VPIndex`] and [`VPLength`] is that:
/// - A `VPLength` is 1-based (starts from 1)
/// - A `VPIndex` is 0-based (starts from 0)
/// - The last valid index in a component with length L is L-1
///
/// # Examples
///
/// ```
/// use r3bl_tui::{VPIndex, ch, vp_idx};
///
/// // Create a VPIndex using the new method
/// let index1 = VPIndex::new(5);
///
/// // Create a VPIndex using the vp_idx function
/// let index2 = vp_idx(5u16);
///
/// // Convert from a ChUnit
/// let index3 = VPIndex::from(ch(5));
///
/// assert_eq!(index1, index2);
/// assert_eq!(index2, index3);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPIndex(ChUnit);
generate_index_type_impl!(
    VPIndex,  // Add impl for this type
    VPLength, // Use this associated type
    vp_idx,   // Make this constructor fn
    vp_len    // Use this constructor fn
);

/// Helper constructor for [`VPIndex`].
pub fn vp_index(val: impl Into<VPIndex>) -> VPIndex { val.into() }

impl From<VPRow> for VPIndex {
    fn from(row: VPRow) -> VPIndex { VPIndex(*row) }
}

impl From<VPCol> for VPIndex {
    fn from(col: VPCol) -> VPIndex { VPIndex(*col) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArrayBoundsCheck, ArrayOverflowResult, LengthOps, NarrowingCastToU16, ch};
    use std::hash::{DefaultHasher, Hasher};

    #[test]
    fn test_index_new() {
        let index = VPIndex::new(10);
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_add() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        let result = index1 + index2;
        assert_eq!(result, vp_idx(15u16));
    }

    #[test]
    fn test_index_add_assign() {
        let mut index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        index1 += index2;
        assert_eq!(index1, vp_idx(15u16));
    }

    #[test]
    fn test_index_sub() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        let result = index1 - index2;
        assert_eq!(result, vp_idx(5u16));
    }

    #[test]
    fn test_index_sub_assign() {
        let mut index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        index1 -= index2;
        assert_eq!(index1, vp_idx(5u16));
    }

    #[test]
    fn test_index_from_ch_unit() {
        let ch_unit = ch(10);
        let index = VPIndex::from(ch_unit);
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_from_usize() {
        let val = 10_usize;
        let index = VPIndex::from(val.as_u16_narrowing());
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_from_u16() {
        let val = 10_u16;
        let index = VPIndex::from(val);
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_from_i32() {
        let val = 10_i32;
        let index = VPIndex::from(val.as_u16_narrowing());
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_as_usize() {
        let index = vp_idx(10u16);
        let val = index.as_usize();
        assert_eq!(val, 10_usize);
    }

    #[test]
    fn test_index_as_u16() {
        let index = vp_idx(10u16);
        let val = index.as_u16();
        assert_eq!(val, 10_u16);
    }

    #[test]
    fn test_index_convert_to_length() {
        let index = vp_idx(9u16); // 0 based.
        let value = index.convert_to_length(); // 1 based.
        assert_eq!(value, vp_len(10));
    }

    #[test]
    fn test_index_deref() {
        let index = vp_idx(10u16);
        let value = *index;
        assert_eq!(value, ch(10));
    }

    #[test]
    fn test_index_deref_mut() {
        let mut index = vp_idx(10u16);
        *index = ch(20);
        assert_eq!(index, vp_idx(20u16));
    }

    #[test]
    fn test_index_sub_length() {
        let index = vp_idx(10u16);
        let length = vp_len(3);
        let result = index - length;
        assert_eq!(result, vp_idx(7u16));
    }

    #[test]
    fn test_index_sub_assign_length() {
        let mut index = vp_idx(10u16);
        let length = vp_len(3);
        index -= length;
        assert_eq!(index, vp_idx(7u16));
    }

    #[test]
    fn test_index_add_length() {
        let index = vp_idx(10u16);
        let length = vp_len(3);
        let result = index + length;
        assert_eq!(result, vp_idx(13u16));
    }

    #[test]
    fn test_index_add_assign_length() {
        let mut index = vp_idx(10u16);
        let length = vp_len(3);
        index += length;
        assert_eq!(index, vp_idx(13u16));
    }

    #[test]
    fn test_index_mul_length() {
        let index = vp_idx(10u16);
        let length = vp_len(3);
        let result = index * length;
        assert_eq!(result, vp_idx(30u16));
    }

    #[test]
    fn test_index_into_usize() {
        let index = vp_idx(10u16);
        let result: usize = index.into();
        assert_eq!(result, 10);
    }

    #[test]
    fn test_index_partial_ord() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        assert!(index1 > index2);
        assert!(index2 < index1);
        assert!(index1 >= index2);
        assert!(index2 <= index1);
    }

    #[test]
    fn test_index_ord() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        assert!(index1 > index2);
        assert!(index2 < index1);
    }

    #[test]
    fn test_index_eq() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(10u16);
        assert_eq!(index1, index2);
    }

    #[test]
    fn test_index_ne() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(5u16);
        assert_ne!(index1, index2);
    }

    #[test]
    fn test_index_hash() {
        let index1 = vp_idx(10u16);
        let index2 = vp_idx(10u16);

        let mut hasher1 = DefaultHasher::new();
        index1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        index2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_idx_fn() {
        let index = VPIndex(ch(10));
        assert_eq!(index, vp_idx(10u16));
    }

    #[test]
    fn test_index_max_value() {
        // Test with maximum u16 value.
        let max_index = vp_idx(u16::MAX);
        assert_eq!(max_index.as_u16(), u16::MAX);
    }

    #[test]
    fn test_index_convert_to_length_edge_cases() {
        // Test with 0
        let index = vp_idx(0u16);
        let length = index.convert_to_length();
        assert_eq!(length, vp_len(1));

        // Test with max value.
        let max_index = vp_idx(u16::MAX - 1); // Subtract 1 to avoid overflow when adding 1
        let length = max_index.convert_to_length();
        assert_eq!(length, vp_len(u16::MAX));
    }

    #[test]
    fn test_index_arithmetic_edge_cases() {
        // Test addition near maximum value.
        let max_index = vp_idx(u16::MAX - 5);
        let small_index = vp_idx(5u16);
        let result = max_index + small_index;
        assert_eq!(result, vp_idx(u16::MAX));

        // Test subtraction with zero.
        let index = vp_idx(5u16);
        let result = index - vp_idx(5u16);
        assert_eq!(result, vp_idx(0u16));

        // Test subtraction below zero (should clamp to zero due to unsigned type)
        let index = vp_idx(5u16);
        let result = index - vp_idx(10u16);
        assert_eq!(result, vp_idx(0u16));
    }

    #[test]
    fn test_index_with_length_operations_edge_cases() {
        // Test addition with length near maximum.
        let max_index = vp_idx(u16::MAX - 5);
        let length = vp_len(5);
        let result = max_index + length;
        assert_eq!(result, vp_idx(u16::MAX));

        // Test subtraction with length.
        let index = vp_idx(10u16);
        let length = vp_len(5);
        let result = index - length;
        assert_eq!(result, vp_idx(5u16));

        // Test subtraction with length below zero.
        let index = vp_idx(5u16);
        let length = vp_len(10);
        let result = index - length;
        assert_eq!(result, vp_idx(0u16));

        // Test multiplication with length.
        let index = vp_idx(u16::MAX / 2);
        let length = vp_len(2);
        let result = index * length;
        assert_eq!(result, vp_idx(u16::MAX - 1)); // Due to how multiplication works with u16
    }

    #[test]
    fn test_index_bounds_check_with_length() {
        // Test index within bounds.
        let index = vp_idx(5u16);
        let length = vp_len(10);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Within);

        // Test index at boundary.
        let index = vp_idx(9u16);
        let length = vp_len(10);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Within);

        // Test index overflowing.
        let index = vp_idx(10u16);
        let length = vp_len(10);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Overflowed);

        // Test index far beyond bounds.
        let index = vp_idx(20u16);
        let length = vp_len(10);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Overflowed);
    }

    #[test]
    fn test_index_bounds_check_edge_cases() {
        // Test with zero length - empty collections have no valid indices
        let index = vp_idx(0u16);
        let length = vp_len(0);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Overflowed);

        // Test with non-zero index against zero length.
        let index = vp_idx(1u16);
        let length = vp_len(0);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Overflowed);

        // Test with maximum values.
        let index = vp_idx(u16::MAX);
        let length = vp_len(u16::MAX);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Overflowed);

        // Test with maximum index against maximum length.
        let index = vp_idx(u16::MAX - 1);
        let length = vp_len(u16::MAX);
        assert_eq!(index.overflows(length), ArrayOverflowResult::Within);
    }

    #[test]
    fn test_full_interoperability() {
        // Create an index and length.
        let index = vp_idx(5u16);
        let length = vp_len(10);

        // Check if index is within bounds.
        assert_eq!(index.overflows(length), ArrayOverflowResult::Within);

        // Convert index to length.
        let new_length = index.convert_to_length();
        assert_eq!(new_length, vp_len(6));

        // Convert length to index.
        let new_index = length.convert_to_index();
        assert_eq!(new_index, vp_idx(9u16));

        // Perform arithmetic with index and length.
        let result_index = index + length;
        assert_eq!(result_index, vp_idx(15u16));

        // Check if the new index is within bounds.
        assert_eq!(
            result_index.overflows(length),
            ArrayOverflowResult::Overflowed
        );

        // Subtract length from index.
        let result_index = result_index - length;
        assert_eq!(result_index, vp_idx(5u16));

        // Check if the new index is within bounds.
        assert_eq!(result_index.overflows(length), ArrayOverflowResult::Within);
    }
}
