// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! One-based character size measurements for terminal UI - see [`VPLength`] type.

use crate::{ChUnit, VPHeight, VPIndex, VPWidth, generate_length_type_impl};
use std::hash::Hash;

/// Represents a length measurement in character units within a viewport.
///
/// A `VPLength` is a 1-based measurement (as opposed to 0-based indices) that represents
/// the size or extent of something in the terminal UI, such as the width or height
/// of a component. It wraps a [`ChUnit`] value.
///
/// `VPLength` values can be created using the [`VPLength::new`] method, the [`vp_len`] /
/// [`vp_length`] functions, or by converting from various numeric types.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{VPLength, ch, vp_len};
///
/// // Create a VPLength using the new method
/// let length1 = VPLength::new(10u16);
///
/// // Create a VPLength using the vp_len function
/// let length2 = vp_len(10);
///
/// // Convert from a ChUnit
/// let length3 = VPLength::from(ch(10));
///
/// assert_eq!(length1, length2);
/// assert_eq!(length2, length3);
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPLength(ChUnit);
generate_length_type_impl!(VPLength, VPIndex, vp_len, vp_idx);

/// Helper constructor for [`VPLength`].
pub fn vp_length(val: impl Into<VPLength>) -> VPLength { val.into() }

impl From<VPLength> for VPWidth {
    fn from(val: VPLength) -> VPWidth { (*val).into() }
}

impl From<VPWidth> for VPLength {
    fn from(val: VPWidth) -> VPLength { (*val).into() }
}

impl From<VPLength> for VPHeight {
    fn from(val: VPLength) -> VPHeight { (*val).into() }
}

impl From<VPHeight> for VPLength {
    fn from(val: VPHeight) -> VPLength { (*val).into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, NarrowingCastToU16, ch, vp_idx};

    #[test]
    fn test_length_creation() {
        let length1 = VPLength::new(10u16);
        let length2 = VPLength::from(20);
        assert_eq!(*length1, ch(10));
        assert_eq!(*length2, ch(20));
    }

    #[test]
    fn test_length_conversion() {
        let length = VPLength::new(10u16);
        let index = length.convert_to_index();
        assert_eq!(*index, ch(9));
    }

    #[test]
    fn test_length_operators() {
        let length1 = VPLength::new(10u16);
        let length2 = VPLength::new(20u16);

        // Add
        let length3 = length1 + length2;
        assert_eq!(*length3, ch(30));

        // AddAssign
        let mut length4 = VPLength::new(10u16);
        length4 += length2;
        assert_eq!(*length4, ch(30));

        // Sub
        let length5 = length2 - length1;
        assert_eq!(*length5, ch(10));

        // SubAssign
        let mut length6 = VPLength::new(20u16);
        length6 -= length1;
        assert_eq!(*length6, ch(10));

        // Div (length / length -> u16 count)
        let count = length2 / length1;
        assert_eq!(count, 2_u16);

        // Rem (length % length -> u16 remainder)
        let remainder = VPLength::new(25u16) % length1;
        assert_eq!(remainder, 5_u16);

        // Div by scalar (length / u16 -> length)
        let half = length2 / 2_u16;
        assert_eq!(*half, ch(10));
    }

    #[test]
    fn test_length_deref() {
        let length = VPLength::new(10u16);
        let value = *length;
        assert_eq!(value, ch(10));
    }

    #[test]
    fn test_length_deref_mut() {
        let mut length = VPLength::new(10u16);
        *length = ch(20);
        assert_eq!(*length, ch(20));
    }

    #[test]
    fn test_length_from_various_types() {
        let length1 = VPLength::from(10);
        let length2 = VPLength::from(20);
        let length3 = VPLength::from(30_i32.as_u16_narrowing());

        assert_eq!(*length1, ch(10));
        assert_eq!(*length2, ch(20));
        assert_eq!(*length3, ch(30));
    }

    #[test]
    fn test_length_partial_eq() {
        let length1 = VPLength::new(10u16);
        let length2 = VPLength::new(10u16);
        let length3 = VPLength::new(20u16);

        assert_eq!(length1, length2);
        assert_ne!(length1, length3);
    }

    #[test]
    fn test_length_partial_ord() {
        let length1 = VPLength::new(10u16);
        let length2 = VPLength::new(20u16);

        assert!(length1 < length2);
        assert!(length2 > length1);
        assert!(length1 <= length2);
        assert!(length2 >= length1);
    }

    #[test]
    fn test_len_fn() {
        let length1 = vp_len(10);
        assert_eq!(*length1, ch(10));

        let length2 = vp_len(VPLength::new(20));
        assert_eq!(*length2, ch(20));
    }

    #[test]
    fn test_length_max_value() {
        // Test with maximum u16 value.
        let max_length = VPLength::new(u16::MAX);
        assert_eq!(max_length.as_u16(), u16::MAX);
    }

    #[test]
    fn test_length_zero() {
        // Test with zero
        let zero_length = VPLength::new(0u16);
        assert_eq!(*zero_length, ch(0));

        // Converting zero length to index.
        let index = zero_length.convert_to_index();
        assert_eq!(*index, ch(0)); // Should be 0 since we don't go below 0
    }

    #[test]
    fn test_length_interop_with_index() {
        // Test interoperability with VPIndex.
        let length = VPLength::new(10u16);
        let index = vp_idx(5u16);

        // Index + Length
        let new_index = index + length;
        assert_eq!(new_index, vp_idx(15u16));

        // Index - Length
        let new_index = vp_idx(20u16) - length;
        assert_eq!(new_index, vp_idx(10u16));
    }

    #[test]
    fn test_length_arithmetic_edge_cases() {
        // Test addition near maximum value.
        let max_length = VPLength::new(u16::MAX - 5);
        let small_length = VPLength::new(5u16);
        let result = max_length + small_length;
        assert_eq!(result, VPLength::new(u16::MAX));

        // Test subtraction with zero.
        let length = VPLength::new(5u16);
        let result = length - VPLength::new(5u16);
        assert_eq!(result, VPLength::new(0u16));

        // Test subtraction below zero (should clamp to zero due to unsigned type)
        let length = VPLength::new(5u16);
        let result = length - VPLength::new(10u16);
        assert_eq!(result, VPLength::new(0u16));
    }
}

#[cfg(test)]
mod tests_col_width_conversion {
    use super::*;
    use crate::{WideningCastToUsize, vp_width};

    #[test]
    fn test_length_to_col_width() {
        let length = VPLength::new(10u16);
        let col_width: VPWidth = length.into();
        assert_eq!(col_width, vp_width(10));
    }

    #[test]
    fn test_col_width_to_length() {
        let col_width = vp_width(15);
        let length: VPLength = col_width.into();
        assert_eq!(length, vp_len(15));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_length = vp_len(42);
        let col_width: VPWidth = original_length.into();
        let back_to_length: VPLength = col_width.into();
        assert_eq!(original_length, back_to_length);

        let original_width = vp_width(37);
        let length: VPLength = original_width.into();
        let back_to_width: VPWidth = length.into();
        assert_eq!(original_width, back_to_width);
    }

    #[test]
    fn test_zero_conversion() {
        let zero_length = vp_len(0);
        let zero_width: VPWidth = zero_length.into();
        assert_eq!(zero_width, vp_width(0));

        let zero_width = vp_width(0);
        let zero_length: VPLength = zero_width.into();
        assert_eq!(zero_length, vp_len(0));
    }

    #[test]
    fn test_max_value_conversion() {
        let max_length = vp_len(u16::MAX);
        let max_width: VPWidth = max_length.into();
        assert_eq!(max_width.as_usize(), u16::MAX.as_usize_widening());

        let max_width = vp_width(u16::MAX);
        let max_length: VPLength = max_width.into();
        assert_eq!(max_length.as_usize(), u16::MAX.as_usize_widening());
    }

    #[test]
    fn test_conversion_preserves_underlying_ch_unit() {
        let length = vp_len(25);
        let width: VPWidth = length.into();

        // Both should have the same underlying ChUnit value.
        assert_eq!(*length, *width);
    }
}

#[cfg(test)]
mod tests_row_height_conversion {
    use super::*;
    use crate::{WideningCastToUsize, vp_height};

    #[test]
    fn test_length_to_row_height() {
        let length = VPLength::new(8u16);
        let row_height: VPHeight = length.into();
        assert_eq!(row_height, vp_height(8));
    }

    #[test]
    fn test_row_height_to_length() {
        let row_height = vp_height(12);
        let length: VPLength = row_height.into();
        assert_eq!(length, vp_len(12));
    }

    #[test]
    fn test_round_trip_conversion() {
        let original_length = vp_len(33);
        let row_height: VPHeight = original_length.into();
        let back_to_length: VPLength = row_height.into();
        assert_eq!(original_length, back_to_length);

        let original_height = vp_height(29);
        let length: VPLength = original_height.into();
        let back_to_height: VPHeight = length.into();
        assert_eq!(original_height, back_to_height);
    }

    #[test]
    fn test_zero_conversion() {
        let zero_length = vp_len(0);
        let zero_height: VPHeight = zero_length.into();
        assert_eq!(zero_height, vp_height(0));

        let zero_height = vp_height(0);
        let zero_length: VPLength = zero_height.into();
        assert_eq!(zero_length, vp_len(0));
    }

    #[test]
    fn test_max_value_conversion() {
        let max_length = vp_len(u16::MAX);
        let max_height: VPHeight = max_length.into();
        assert_eq!(max_height.as_usize(), u16::MAX.as_usize_widening());

        let max_height = vp_height(u16::MAX);
        let max_length: VPLength = max_height.into();
        assert_eq!(max_length.as_usize(), u16::MAX.as_usize_widening());
    }

    #[test]
    fn test_conversion_preserves_underlying_chunit() {
        let length = vp_len(18);
        let height: VPHeight = length.into();

        // Both should have the same underlying ChUnit value.
        assert_eq!(*length, *height);
    }

    #[test]
    fn test_different_conversions_independence() {
        // Verify that ColWidth and RowHeight conversions work independently.
        let length = vp_len(50);

        let width: VPWidth = length.into();
        let height: VPHeight = length.into();

        // All three should have the same underlying value.
        assert_eq!(*length, *width);
        assert_eq!(*length, *height);
        assert_eq!(*width, *height);
    }
}
