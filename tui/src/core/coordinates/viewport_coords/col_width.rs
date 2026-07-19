// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::VPCol;
use crate::{ChUnit, generate_length_type_impl};
use std::hash::Hash;

/// [`VPWidth`] is column count, i.e., the number of columns that a UI component
/// occupies.
///
/// This is one part of a [`VPSize`] and is different from the [`VPCol`] (position).
/// You can use the [`vp_width()`] to create a new instance.
///
/// # Working with col index
/// You cannot safely add or subtract a [`VPCol`] from this [`VPWidth`]; since
/// without knowing your specific use case ahead of time, it is not possible to provide a
/// default implementation without leading to unintended consequences. You can do the
/// reverse safely.
///
/// To add or subtract a [`VPCol`] from this [`VPWidth`], you can call
/// [`LengthOps::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
///
/// # Examples
/// ```
/// use r3bl_tui::{VPWidth, vp_width};
/// let width = vp_width(5);
/// let width = VPWidth::new(5u16);
/// ```
///
/// [`LengthOps::convert_to_index()`]: crate::LengthOps::convert_to_index
/// [`VPCol`]: crate::VPCol
/// [`VPSize`]: crate::VPSize
/// [`VPWidth`]: crate::VPWidth
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPWidth(ChUnit);
generate_length_type_impl!(VPWidth, VPCol, vp_width, vp_col);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, ch, vp_col};

    #[test]
    fn test_width_new() {
        let it = VPWidth::new(5u16);
        assert_eq!(it, vp_width(5));
        assert_eq!(*it, ch(5));
    }

    #[test]
    fn test_width_add() {
        // Add.
        {
            let width1 = VPWidth(5u16.into());
            let width2 = VPWidth(3u16.into());
            let result = width1 + width2;
            assert_eq!(result, VPWidth(8u16.into()));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut width1 = VPWidth(5u16.into());
            let width2 = VPWidth(3u16.into());
            width1 += width2;
            assert_eq!(width1, VPWidth(8u16.into()));
            assert_eq!(*width1, ch(8));
        }
    }

    #[test]
    fn test_width_sub() {
        // Sub. This returns a ColWidth as expected, and not a ColIndex.
        {
            let width1 = vp_width(5);
            let width2 = vp_width(3);
            let result = width1 - width2;
            assert_eq!(result, vp_width(2));
            assert_eq!(*result, ch(2));
        }
        // SubAssign.
        {
            let mut width1 = vp_width(5);
            let width2 = vp_width(3);
            width1 -= width2;
            assert_eq!(width1, vp_width(2));
            assert_eq!(*width1, ch(2));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut width = VPWidth(5u16.into());
        assert_eq!(*width, ch(5));
        *width = ch(10);
        assert_eq!(*width, ch(10));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(vp_width(10) / ch(2), vp_width(5));
    }

    #[test]
    fn test_div_col_width_returns_count() {
        // Dividing width by width yields a dimensionless count.
        assert_eq!(vp_width(240) / vp_width(80), 3_u16);
        assert_eq!(vp_width(80) / vp_width(80), 1_u16);
        assert_eq!(vp_width(79) / vp_width(80), 0_u16);
    }

    #[test]
    fn test_rem_col_width_returns_remainder() {
        // Remainder of width by width yields a dimensionless offset.
        assert_eq!(vp_width(240) % vp_width(80), 0_u16);
        assert_eq!(vp_width(245) % vp_width(80), 5_u16);
        assert_eq!(vp_width(79) % vp_width(80), 79_u16);
    }

    #[test]
    fn test_div_u16_scales_down() {
        // Dividing width by scalar scales down the width.
        assert_eq!(vp_width(80) / 2_u16, vp_width(40));
        assert_eq!(vp_width(100) / 4_u16, vp_width(25));
    }

    #[test]
    fn test_convert_to_index() {
        assert_eq!(vp_width(5).convert_to_index(), vp_col(4));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(VPWidth::from(5), vp_width(5));
    }
}
