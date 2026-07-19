// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`VPSize`] is a struct that holds the `width` and `height` of a text buffer.
//! [`VPWidth`] and [`VPHeight`] are the types of
//! the `width` and `height` respectively. This ensures that it isn't possible to use a
//! `width` when you intended to use a `height` and vice versa.
//!
//! Here is a visual representation of how position and sizing work for the layout
//! engine.
//!
//! ```text
//!     0   4    9    1    2    2
//!                   4    0    5
//!    ┌────┴────┴────┴────┴────┴── col
//!  0 ┤     ╭─────────────╮
//!  1 ┤     │ origin pos: │
//!  2 ┤     │ [5, 0]      │
//!  3 ┤     │ size:       │
//!  4 ┤     │ [16, 5]     │
//!  5 ┤     ╰─────────────╯
//!    │
//!   row
//! ```
//!
//! # The many ways to create one
//!
//! - This API uses the `impl Into<struct>` pattern and [Add] `+` operator overloading to
//!   allow for easy construction of [`VPSize`] by [`VPWidth`] with [`VPHeight`] in any
//!   order.
//! - You can use the [`vp_size()`] to create a [`VPSize`] struct. This function can take
//!   a sequence of [Add]ed [`VPWidth`] and [`VPHeight`] in any order, or tuples of them
//!   in any order.
//! - Just using the [Add] `+` operator ([`VPHeight`] and [`VPWidth`] can be in any
//!   order):
//!     - You can use [Add] to compose [`VPWidth`] + [`VPHeight`] into a [`VPSize`].
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{
//!     ch, VPSize, VPWidth, VPHeight,
//!     vp_width, vp_height, vp_size
//! };
//!
//! // Note the order of the arguments don't matter below.
//! let size: VPSize = (vp_width(1) + vp_height(2)).into();
//! assert_eq!(size.col_width, ch(1).into());
//! assert_eq!(*size.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_2: VPSize = ( vp_height(2), vp_width(1) ).into();
//! assert_eq!(*size_2.col_width, ch(1));
//! assert_eq!(*size_2.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_3 = VPSize::new(
//!     ( vp_height(2), vp_width(1) )
//! );
//! assert_eq!(*size_3.col_width, ch(1));
//! assert_eq!(*size_3.row_height, ch(2));
//! assert!(size_2 == size_3);
//!
//! let size_sum = size_2 + size_3;
//! assert_eq!(size_sum.col_width, ch(2).into());
//! assert_eq!(*size_sum.row_height, ch(4));
//!
//! let size_diff = size_2 - size_3;
//! assert_eq!(size_diff.col_width, ch(0).into());
//! assert_eq!(*size_diff.row_height, ch(0));
//! ```
//!
//! [`vp_size()`]: crate::vp_size()

use crate::{ChUnit, VPHeight, VPWidth, vp_height, vp_width};
use std::{fmt::Debug,
          ops::{Add, AddAssign, Sub, SubAssign}};

#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPSize {
    pub col_width: VPWidth,
    pub row_height: VPHeight,
}

#[inline]
pub fn vp_size(width_val: impl Into<VPWidth>, height_val: impl Into<VPHeight>) -> VPSize {
    VPSize {
        col_width: vp_width(width_val),
        row_height: vp_height(height_val),
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Ord, Eq, Hash)]
pub enum SufficientSize {
    IsLargeEnough,
    IsTooSmall,
}

mod constructor {
    use super::{Add, VPHeight, VPSize, VPWidth};

    impl VPSize {
        #[inline]
        pub fn new(arg_dim: impl Into<VPSize>) -> Self { arg_dim.into() }
    }

    impl From<(VPWidth, VPHeight)> for VPSize {
        #[inline]
        fn from((width, height): (VPWidth, VPHeight)) -> VPSize {
            VPSize {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl From<(VPHeight, VPWidth)> for VPSize {
        #[inline]
        fn from((height, width): (VPHeight, VPWidth)) -> VPSize {
            VPSize {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl Add<VPHeight> for VPWidth {
        type Output = VPSize;

        fn add(self, rhs: VPHeight) -> Self::Output {
            VPSize {
                col_width: self,
                row_height: rhs,
            }
        }
    }

    impl Add<VPWidth> for VPHeight {
        type Output = VPSize;

        fn add(self, rhs: VPWidth) -> Self::Output {
            VPSize {
                col_width: rhs,
                row_height: self,
            }
        }
    }
}

mod convert {
    use super::{VPHeight, VPSize, VPWidth};

    impl From<VPSize> for VPWidth {
        fn from(size: VPSize) -> VPWidth { size.col_width }
    }

    impl From<VPSize> for VPHeight {
        fn from(size: VPSize) -> VPHeight { size.row_height }
    }
}

mod api {
    use super::{SufficientSize, VPSize};

    impl VPSize {
        pub fn fits_min_size(&self, arg_min_size: impl Into<VPSize>) -> SufficientSize {
            let size: VPSize = arg_min_size.into();
            let min_width = size.col_width;
            let min_height = size.row_height;

            if self.col_width < min_width || self.row_height < min_height {
                SufficientSize::IsTooSmall
            } else {
                SufficientSize::IsLargeEnough
            }
        }
    }
}

mod debug {
    use super::{Debug, VPSize};

    impl Debug for VPSize {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "[w: {w:?}, h: {h:?}]",
                w = *self.col_width,
                h = *self.row_height
            )
        }
    }
}

mod dimension_arithmetic_operators {
    use super::{Add, Sub, VPSize};

    impl Sub<VPSize> for VPSize {
        type Output = VPSize;

        fn sub(self, rhs: VPSize) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width -= *rhs.col_width;
            *self_copy.row_height -= *rhs.row_height;
            self_copy
        }
    }

    impl Add<VPSize> for VPSize {
        type Output = VPSize;

        fn add(self, rhs: VPSize) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width += *rhs.col_width;
            *self_copy.row_height += *rhs.row_height;
            self_copy
        }
    }
}

mod numeric_arithmetic_operators {
    use super::{Add, AddAssign, ChUnit, Sub, SubAssign, VPSize};

    impl SubAssign<ChUnit> for VPSize {
        fn sub_assign(&mut self, other: ChUnit) {
            *self.col_width -= other;
            *self.row_height -= other;
        }
    }

    impl Sub<ChUnit> for VPSize {
        type Output = VPSize;

        fn sub(self, other: ChUnit) -> Self::Output {
            let mut self_copy = self;
            self_copy -= other;
            self_copy
        }
    }

    impl AddAssign<ChUnit> for VPSize {
        fn add_assign(&mut self, other: ChUnit) {
            *self.col_width += other;
            *self.row_height += other;
        }
    }

    impl Add<ChUnit> for VPSize {
        type Output = VPSize;

        fn add(self, other: ChUnit) -> Self::Output {
            let mut self_copy = self;
            self_copy += other;
            self_copy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ch, vp_height, vp_width};

    #[test]
    fn test_dim() {
        let size_1 = vp_width(5) + vp_height(10);
        assert_eq!(size_1.col_width, vp_width(5));
        assert_eq!(*size_1.col_width, ch(5));
        assert_eq!(size_1.row_height, vp_height(10));
        assert_eq!(*size_1.row_height, ch(10));
        let size_2 = vp_height(10) + vp_width(5);

        assert_eq!(size_2.col_width, vp_width(5));
        assert_eq!(size_2.row_height, vp_height(10));
    }

    #[test]
    fn test_size_new() {
        // Order does not matter.
        let size = VPSize::new((VPWidth::new(5u16), VPHeight::new(10u16)));
        assert_eq!(size.col_width, vp_width(5));
        assert_eq!(*size.col_width, 5u16.into());
        assert_eq!(size.row_height, vp_height(10));
        assert_eq!(*size.row_height, ch(10));

        // Order does not matter.
        let size_2 = VPSize::new((vp_width(5), vp_height(10)));
        assert_eq!(size_2.col_width, vp_width(5));
        assert_eq!(size_2.row_height, vp_height(10));
    }

    #[test]
    fn test_size_from() {
        // Order does not matter!
        let size: VPSize = (vp_width(5), vp_height(10)).into();
        let size_2: VPSize = (vp_height(10), vp_width(5)).into();

        assert_eq!(size.col_width, vp_width(5));
        assert_eq!(*size.col_width, ch(5));
        assert_eq!(size.row_height, vp_height(10));
        assert_eq!(*size.row_height, ch(10));

        assert_eq!(size, size_2);
    }

    #[test]
    fn test_size_add() {
        let size1 = VPSize::new((VPWidth::from(5u16), VPHeight::from(10u16)));
        let size2 = VPSize::new((VPWidth::from(ch(3)), VPHeight::from(ch(4))));
        let result = size1 + size2;
        assert_eq!(result.col_width, vp_width(8));
        assert_eq!(*result.col_width, ch(8));
        assert_eq!(result.row_height, vp_height(14));
        assert_eq!(*result.row_height, ch(14));
    }

    #[test]
    fn test_size_sub() {
        let size1 = VPSize::new((VPWidth::from(5u16), VPHeight::from(10u16)));
        let size2 = VPSize::new((VPWidth::from(3u16), VPHeight::from(4u16)));
        let result = size1 - size2;
        assert_eq!(result.col_width, vp_width(2));
        assert_eq!(result.row_height, vp_height(6));
    }

    #[test]
    fn test_fits_min_size() {
        let size = vp_width(5) + vp_height(10);
        assert_eq!(
            size.fits_min_size(VPSize::new((vp_width(3), vp_height(4)))),
            SufficientSize::IsLargeEnough
        );
        assert_eq!(
            size.fits_min_size(VPSize::new((vp_width(100), vp_height(100)))),
            SufficientSize::IsTooSmall
        );
    }

    #[test]
    fn test_debug_fmt() {
        let size = VPSize::new((vp_width(5), vp_height(10)));
        assert_eq!(format!("{size:?}"), "[w: 5, h: 10]");
    }

    #[test]
    fn test_ch_unit_sub_and_sub_assign() {
        let mut size0 = VPSize::new((vp_width(5), vp_height(10)));
        size0 -= ch(3);
        assert_eq!(size0.col_width, vp_width(2));
        assert_eq!(size0.row_height, vp_height(7));

        let size1 = size0 - ch(1);
        assert_eq!(size1.col_width, vp_width(1));
        assert_eq!(size1.row_height, vp_height(6));
    }

    #[test]
    fn test_ch_unit_add_and_add_assign() {
        let mut size0 = VPSize::new((vp_width(5), vp_height(10)));
        size0 += ch(3);
        assert_eq!(size0.col_width, vp_width(8));
        assert_eq!(size0.row_height, vp_height(13));

        let size1 = size0 + ch(1);
        assert_eq!(size1.col_width, vp_width(9));
        assert_eq!(size1.row_height, vp_height(14));
    }

    #[test]
    fn test_convert_dim_to_width_or_height() {
        let size = vp_width(5) + vp_height(10);
        let w: VPWidth = size.into();
        let h: VPHeight = size.into();
        assert_eq!(h, vp_height(10));
        assert_eq!(w, vp_width(5));
    }
}
