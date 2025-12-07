// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::ColIndex;
use crate::{ChUnit, generate_length_type_impl};
use std::hash::Hash;

/// [`ColWidth`] is column count, i.e., the number of columns that a UI component
/// occupies.
///
/// This is one part of a [`Size`] and is different from the [`ColIndex`] (position).
/// You can use the [`width()`] to create a new instance.
///
/// # Working with col index
/// You cannot safely add or subtract a [`ColIndex`] from this [`ColWidth`]; since without
/// knowing your specific use case ahead of time, it is not possible to provide a default
/// implementation without leading to unintended consequences. You can do the reverse
/// safely.
///
/// To add or subtract a [`ColIndex`] from this [`ColWidth`], you can call
/// [`LengthOps::convert_to_index()`] and apply whatever logic makes sense for your use
/// case.
///
/// [`LengthOps::convert_to_index()`]: crate::LengthOps::convert_to_index
///
/// # Examples
/// ```
/// use r3bl_tui::{ColWidth, width};
/// let width = width(5);
/// let width = ColWidth::new(5);
/// ```
///
/// [`ColWidth`]: crate::ColWidth
/// [`ColIndex`]: crate::ColIndex
/// [`Size`]: crate::Size
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct ColWidth(pub ChUnit);
generate_length_type_impl!(ColWidth, ColIndex, width, col);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LengthOps, ch, col};

    #[test]
    fn test_width_new() {
        let it = ColWidth::new(5);
        assert_eq!(it, width(5));
        assert_eq!(*it, ch(5));
    }

    #[test]
    fn test_width_add() {
        // Add.
        {
            let width1 = ColWidth(5.into());
            let width2 = ColWidth(3.into());
            let result = width1 + width2;
            assert_eq!(result, ColWidth(8.into()));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut width1 = ColWidth(5.into());
            let width2 = ColWidth(3.into());
            width1 += width2;
            assert_eq!(width1, ColWidth(8.into()));
            assert_eq!(*width1, ch(8));
        }
    }

    #[test]
    fn test_width_sub() {
        // Sub. This returns a ColWidth as expected, and not a ColIndex.
        {
            let width1 = width(5);
            let width2 = width(3);
            let result = width1 - width2;
            assert_eq!(result, width(2));
            assert_eq!(*result, ch(2));
        }
        // SubAssign.
        {
            let mut width1 = width(5);
            let width2 = width(3);
            width1 -= width2;
            assert_eq!(width1, width(2));
            assert_eq!(*width1, ch(2));
        }
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut width = ColWidth(5.into());
        assert_eq!(*width, ch(5));
        *width = ch(10);
        assert_eq!(*width, ch(10));
    }

    #[test]
    fn test_div_ch_unit() {
        assert_eq!(width(10) / ch(2), width(5));
    }

    #[test]
    fn test_div_col_width_returns_count() {
        // Dividing width by width yields a dimensionless count.
        assert_eq!(width(240) / width(80), 3_u16);
        assert_eq!(width(80) / width(80), 1_u16);
        assert_eq!(width(79) / width(80), 0_u16);
    }

    #[test]
    fn test_rem_col_width_returns_remainder() {
        // Remainder of width by width yields a dimensionless offset.
        assert_eq!(width(240) % width(80), 0_u16);
        assert_eq!(width(245) % width(80), 5_u16);
        assert_eq!(width(79) % width(80), 79_u16);
    }

    #[test]
    fn test_div_u16_scales_down() {
        // Dividing width by scalar scales down the width.
        assert_eq!(width(80) / 2_u16, width(40));
        assert_eq!(width(100) / 4_u16, width(25));
    }

    #[test]
    fn test_convert_to_index() {
        assert_eq!(width(5).convert_to_index(), col(4));
    }

    #[test]
    fn test_convert_from_usize() {
        assert_eq!(ColWidth::from(5usize), width(5));
    }
}
