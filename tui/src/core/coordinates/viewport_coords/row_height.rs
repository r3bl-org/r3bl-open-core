// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::VPRow;
use crate::{ChUnit, generate_length_type_impl};
use std::hash::Hash;

/// [`VPHeight`] is row count, i.e., the number of rows that a UI component occupies.
///
/// This is one part of a [`VPSize`] and is different from the [`VPRow`] (position).
/// You can use the [`vp_height()`] to create a new instance.
///
/// # Working with row index
/// You cannot safely add or subtract a [`VPRow`] from this [`VPHeight`]; since
/// without knowing your specific use case ahead of time, it is not possible to provide a
/// default implementation without leading to unintended consequences. You can do the
/// reverse safely.
///
/// To add or subtract a [`VPRow`] from this [`VPHeight`], you can call
/// [`LengthOps::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
///
/// # Examples
/// ```
/// use r3bl_tui::{VPHeight, vp_height};
/// let height = vp_height(5);
/// let height = VPHeight::new(5u16);
/// ```
///
/// [`LengthOps::convert_to_index()`]: crate::LengthOps::convert_to_index
/// [`VPHeight`]: crate::VPHeight
/// [`VPRow`]: crate::VPRow
/// [`VPSize`]: crate::VPSize
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPHeight(ChUnit);
generate_length_type_impl!(VPHeight, VPRow, vp_height, vp_row);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, ch, vp_row};

    #[test]
    fn test_height_new() {
        let height = VPHeight::new(10u16);
        assert_eq!(height, VPHeight(10u16.into()));
        assert_eq!(*height, ch(10));
    }

    #[test]
    fn test_height_add() {
        let height1 = VPHeight(10u16.into());
        let height2 = VPHeight(4u16.into());
        let result = height1 + height2;
        assert_eq!(result, VPHeight(14u16.into()));
        assert_eq!(*result, ch(14));
    }

    #[test]
    fn test_height_sub() {
        // Sub. This returns a RowHeight as expected, and not a RowIndex.
        {
            let height1 = vp_height(10);
            let height2 = vp_height(4);
            let result = height1 - height2;
            assert_eq!(result, vp_height(6));
            assert_eq!(*result, ch(6));
        }

        // SubAssign.
        {
            let mut height1 = vp_height(10);
            let height2 = vp_height(4);
            height1 -= height2;
            assert_eq!(height1, vp_height(6));
            assert_eq!(*height1, ch(6));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut height = VPHeight(10u16.into());
        assert_eq!(*height, ch(10));
        *height = ch(20);
        assert_eq!(*height, ch(20));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(vp_height(10) / ch(2), vp_height(5));
    }

    #[test]
    fn test_div_row_height_returns_count() {
        // Dividing height by height yields a dimensionless count.
        assert_eq!(vp_height(240) / vp_height(80), 3_u16);
        assert_eq!(vp_height(80) / vp_height(80), 1_u16);
        assert_eq!(vp_height(79) / vp_height(80), 0_u16);
    }

    #[test]
    fn test_rem_row_height_returns_remainder() {
        // Remainder of height by height yields a dimensionless offset.
        assert_eq!(vp_height(240) % vp_height(80), 0_u16);
        assert_eq!(vp_height(245) % vp_height(80), 5_u16);
        assert_eq!(vp_height(79) % vp_height(80), 79_u16);
    }

    #[test]
    fn test_div_u16_scales_down() {
        // Dividing height by scalar scales down the height.
        assert_eq!(vp_height(80) / 2_u16, vp_height(40));
        assert_eq!(vp_height(100) / 4_u16, vp_height(25));
    }

    #[test]
    fn test_convert_to_index() {
        assert_eq!(vp_height(10).convert_to_index(), vp_row(9));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(VPHeight::from(10), vp_height(10));
    }
}
