// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::RowIndex;
use crate::{ChUnit, generate_length_type_impl};
use std::hash::Hash;

/// [`RowHeight`] is row count, i.e., the number of rows that a UI component occupies.
///
/// This is one part of a [`Size`] and is different from the [`RowIndex`] (position).
/// You can use the [`height()`] to create a new instance.
///
/// # Working with row index
/// You cannot safely add or subtract a [`RowIndex`] from this [`RowHeight`]; since
/// without knowing your specific use case ahead of time, it is not possible to provide a
/// default implementation without leading to unintended consequences. You can do the
/// reverse safely.
///
/// To add or subtract a [`RowIndex`] from this [`RowHeight`], you can call
/// [`LengthOps::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
/// [`LengthOps::convert_to_index()`]: crate::LengthOps::convert_to_index
///
/// # Examples
/// ```
/// use r3bl_tui::{RowHeight, height};
/// let height = height(5);
/// let height = RowHeight::new(5);
/// ```
///
/// [`RowHeight`]: crate::RowHeight
/// [`RowIndex`]: crate::RowIndex
/// [`Size`]: crate::Size
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct RowHeight(pub ChUnit);
generate_length_type_impl!(RowHeight, RowIndex, height, row);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, ch, row};

    #[test]
    fn test_height_new() {
        let height = RowHeight::new(10);
        assert_eq!(height, RowHeight(10.into()));
        assert_eq!(*height, ch(10));
    }

    #[test]
    fn test_height_add() {
        let height1 = RowHeight(10.into());
        let height2 = RowHeight(4.into());
        let result = height1 + height2;
        assert_eq!(result, RowHeight(14.into()));
        assert_eq!(*result, ch(14));
    }

    #[test]
    fn test_height_sub() {
        // Sub. This returns a RowHeight as expected, and not a RowIndex.
        {
            let height1 = height(10);
            let height2 = height(4);
            let result = height1 - height2;
            assert_eq!(result, height(6));
            assert_eq!(*result, ch(6));
        }

        // SubAssign.
        {
            let mut height1 = height(10);
            let height2 = height(4);
            height1 -= height2;
            assert_eq!(height1, height(6));
            assert_eq!(*height1, ch(6));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut height = RowHeight(10.into());
        assert_eq!(*height, ch(10));
        *height = ch(20);
        assert_eq!(*height, ch(20));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(height(10) / ch(2), height(5));
    }

    #[test]
    fn test_div_row_height_returns_count() {
        // Dividing height by height yields a dimensionless count.
        assert_eq!(height(240) / height(80), 3_u16);
        assert_eq!(height(80) / height(80), 1_u16);
        assert_eq!(height(79) / height(80), 0_u16);
    }

    #[test]
    fn test_rem_row_height_returns_remainder() {
        // Remainder of height by height yields a dimensionless offset.
        assert_eq!(height(240) % height(80), 0_u16);
        assert_eq!(height(245) % height(80), 5_u16);
        assert_eq!(height(79) % height(80), 79_u16);
    }

    #[test]
    fn test_div_u16_scales_down() {
        // Dividing height by scalar scales down the height.
        assert_eq!(height(80) / 2_u16, height(40));
        assert_eq!(height(100) / 4_u16, height(25));
    }

    #[test]
    fn test_convert_to_index() {
        assert_eq!(height(10).convert_to_index(), row(9));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(RowHeight::from(10usize), height(10));
    }
}
