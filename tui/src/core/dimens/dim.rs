/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! [Size] is a struct that holds the `width` and `height` of a text buffer.
//! [`ColWidth`] (aka [Width]) and [`RowHeight`] (aka [Height]) are the types of the
//! `width` and `height` respectively. This ensures that it isn't possible to use a
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
//!   allow for easy construction of [Size] by [`ColWidth`] with [`RowHeight`] in any
//!   order.
//! - You can use the [`crate::size()`] to create a [Size] struct. This function can take
//!   a sequence of [Add]ed [`ColWidth`] and [`RowHeight`] in any order, or tuples of them
//!   in any order.
//! - Just using the [Add] `+` operator ([`RowHeight`] and [`ColWidth`] can be in any
//!   order):
//!     - You can use [Add] to convert: [`ColWidth`] + [`RowHeight`], into: a [Size].
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{
//!     ch, Size, ColWidth, RowHeight,
//!     width, height, Width, Height, size
//! };
//!
//! // Note the order of the arguments don't matter below.
//! let size: Size = size( width(1) + height(2) );
//! assert_eq!(size.col_width, ch(1).into());
//! assert_eq!(*size.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_2: Size = ( height(2), width(1) ).into();
//! assert_eq!(*size_2.col_width, ch(1));
//! assert_eq!(*size_2.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_3 = Size::new(
//!     ( height(2), width(1) )
//! );
//! assert!(matches!(size_3.col_width, ColWidth(_)));
//! assert!(matches!(size_3.row_height, RowHeight(_)));
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

use std::{fmt::Debug,
          ops::{Add, AddAssign, Sub, SubAssign}};

use crate::{ChUnit, ColWidth, RowHeight};

// Type aliases for better code readability.

pub type Width = ColWidth;
pub type Height = RowHeight;

#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Size {
    pub col_width: ColWidth,
    pub row_height: RowHeight,
}

pub fn size(arg_size: impl Into<Size>) -> Size { arg_size.into() }

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Ord, Eq, Hash)]
pub enum SufficientSize {
    IsLargeEnough,
    IsTooSmall,
}

mod constructor {
    use super::{Size, ColWidth, RowHeight, Add};

    impl Size {
        pub fn new(arg_dim: impl Into<Size>) -> Self { arg_dim.into() }
    }

    impl From<(ColWidth, RowHeight)> for Size {
        fn from((width, height): (ColWidth, RowHeight)) -> Self {
            Size {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl From<(RowHeight, ColWidth)> for Size {
        fn from((height, width): (RowHeight, ColWidth)) -> Self {
            Size {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl Add<RowHeight> for ColWidth {
        type Output = Size;

        fn add(self, rhs: RowHeight) -> Self::Output {
            Size {
                col_width: self,
                row_height: rhs,
            }
        }
    }

    impl Add<ColWidth> for RowHeight {
        type Output = Size;

        fn add(self, rhs: ColWidth) -> Self::Output {
            Size {
                col_width: rhs,
                row_height: self,
            }
        }
    }
}

mod convert {
    use super::{Size, ColWidth, RowHeight};

    impl From<Size> for ColWidth {
        fn from(size: Size) -> Self { size.col_width }
    }

    impl From<Size> for RowHeight {
        fn from(size: Size) -> Self { size.row_height }
    }
}

mod api {
    use super::{Size, SufficientSize};

    impl Size {
        pub fn fits_min_size(&self, arg_min_size: impl Into<Size>) -> SufficientSize {
            let size: Size = arg_min_size.into();
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
    use super::{Debug, Size};

    impl Debug for Size {
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

mod ops {
    use super::{Sub, Size, Add, SubAssign, ChUnit, AddAssign};

    impl Sub<Size> for Size {
        type Output = Size;

        fn sub(self, rhs: Size) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width -= *rhs.col_width;
            *self_copy.row_height -= *rhs.row_height;
            self_copy
        }
    }

    impl Add<Size> for Size {
        type Output = Size;

        fn add(self, rhs: Size) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width += *rhs.col_width;
            *self_copy.row_height += *rhs.row_height;
            self_copy
        }
    }

    impl SubAssign<ChUnit> for Size {
        fn sub_assign(&mut self, other: ChUnit) {
            *self.col_width -= other;
            *self.row_height -= other;
        }
    }

    impl Sub<ChUnit> for Size {
        type Output = Size;

        fn sub(self, other: ChUnit) -> Self::Output {
            let mut self_copy = self;
            self_copy -= other;
            self_copy
        }
    }

    impl AddAssign<ChUnit> for Size {
        fn add_assign(&mut self, other: ChUnit) {
            *self.col_width += other;
            *self.row_height += other;
        }
    }

    impl Add<ChUnit> for Size {
        type Output = Size;

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
    use crate::{ch, height, width};

    #[test]
    fn test_dim() {
        let size_1 = size(width(5) + height(10));
        assert_eq!(size_1.col_width, ColWidth(ch(5)));
        assert_eq!(*size_1.col_width, ch(5));
        assert_eq!(size_1.row_height, RowHeight(ch(10)));
        assert_eq!(*size_1.row_height, ch(10));
        let size_2 = size(height(10) + width(5));

        assert!(matches!(size_2.col_width, ColWidth(_)));
        assert!(matches!(size_2.row_height, RowHeight(_)));
    }

    #[test]
    fn test_size_new() {
        // Order does not matter.
        let size = Size::new((ColWidth::new(5), RowHeight::new(10)));
        assert_eq!(size.col_width, ColWidth(ch(5)));
        assert_eq!(*size.col_width, 5.into());
        assert_eq!(size.row_height, RowHeight(10.into()));
        assert_eq!(*size.row_height, ch(10));

        // Order does not matter.
        let size_2 = Size::new((width(5), height(10)));
        assert!(matches!(size_2.col_width, ColWidth(_)));
        assert!(matches!(size_2.row_height, RowHeight(_)));
    }

    #[test]
    fn test_size_from() {
        // Order does not matter!
        let size: Size = (ColWidth(ch(5)), RowHeight(ch(10))).into();
        let size_2: Size = (RowHeight(ch(10)), ColWidth(ch(5))).into();

        assert_eq!(size.col_width, ColWidth(ch(5)));
        assert_eq!(*size.col_width, ch(5));
        assert_eq!(size.row_height, RowHeight(ch(10)));
        assert_eq!(*size.row_height, ch(10));

        assert_eq!(size, size_2);
    }

    #[test]
    fn test_size_add() {
        let size1 = Size::new((ColWidth(5.into()), RowHeight(10.into())));
        let size2 = Size::new((ColWidth::from(ch(3)), RowHeight::from(ch(4))));
        let result = size1 + size2;
        assert_eq!(result.col_width, ColWidth(8.into()));
        assert_eq!(*result.col_width, ch(8));
        assert_eq!(result.row_height, RowHeight(14.into()));
        assert_eq!(*result.row_height, ch(14));
    }

    #[test]
    fn test_size_sub() {
        let size1 = Size::new((ColWidth(5.into()), RowHeight(10.into())));
        let size2 = Size::new((ColWidth(3.into()), RowHeight(4.into())));
        let result = size1 - size2;
        assert_eq!(result.col_width, ColWidth(ch(2)));
        assert_eq!(result.row_height, RowHeight(ch(6)));
    }

    #[test]
    fn test_fits_min_size() {
        let size = width(5) + height(10);
        assert_eq!(
            size.fits_min_size(Size::new((width(3), height(4)))),
            SufficientSize::IsLargeEnough
        );
        assert_eq!(
            size.fits_min_size(Size::new((width(100), height(100)))),
            SufficientSize::IsTooSmall
        );
    }

    #[test]
    fn test_debug_fmt() {
        let size = Size::new((width(5), height(10)));
        assert_eq!(format!("{size:?}"), "[w: 5, h: 10]");
    }

    #[test]
    fn test_ch_unit_sub_and_sub_assign() {
        let mut size0 = Size::new((width(5), height(10)));
        size0 -= ch(3);
        assert_eq!(size0.col_width, ColWidth(ch(2)));
        assert_eq!(size0.row_height, RowHeight(ch(7)));

        let size1 = size0 - ch(1);
        assert_eq!(size1.col_width, ColWidth(ch(1)));
        assert_eq!(size1.row_height, RowHeight(ch(6)));
    }

    #[test]
    fn test_ch_unit_add_and_add_assign() {
        let mut size0 = Size::new((width(5), height(10)));
        size0 += ch(3);
        assert_eq!(size0.col_width, ColWidth(ch(8)));
        assert_eq!(size0.row_height, RowHeight(ch(13)));

        let size1 = size0 + ch(1);
        assert_eq!(size1.col_width, ColWidth(ch(9)));
        assert_eq!(size1.row_height, RowHeight(ch(14)));
    }

    #[test]
    fn test_convert_dim_to_width_or_height() {
        let size = width(5) + height(10);
        let w: Width = size.into();
        let h: Height = size.into();
        assert_eq!(h, RowHeight(ch(10)));
        assert_eq!(w, ColWidth(ch(5)));
    }
}
