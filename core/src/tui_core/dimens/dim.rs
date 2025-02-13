/*
 *   Copyright (c) 2025 Nazmul Idris
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

//! [Dim] is a struct that holds the `width` and `height` of a text buffer.
//! [ColWidth] (aka [Width]) and [RowHeight] (aka [Height]) are the types of the
//! `width` and `height` respectively. This ensures that it isn't possible to use a
//! `width` when you intended to use a `height` and vice versa. Also [Size] is an alias
//! for [Dim].
//!
//! Here is a visual representation of how position and sizing works for the layout
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
//!   allow for easy construction of [Dim] by [ColWidth] with [RowHeight] in any
//!   order.
//! - You can use the [crate::dim()] to create a [Dim] struct. This function can take a
//!   sequence of [Add]ed [ColWidth] and [RowHeight] in any order, or tuples of
//!   them in any order.
//! - Just using the [Add] `+` operator ([RowHeight] and [ColWidth] can be in
//!   any order):
//!     - You can use [Add] to convert: [ColWidth] + [RowHeight], into: a [Dim].
//!
//! # Examples
//!
//! ```rust
//! use r3bl_core::{
//!     ch, Dim, ColWidth, RowHeight,
//!     width, height, Width, Height, dim
//! };
//!
//! // Note the order of the arguments don't matter below.
//! let size: Dim = dim( width(1) + height(2) );
//! assert_eq!(size.col_width, ch(1).into());
//! assert_eq!(*size.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_2: Dim = ( height(2), width(1) ).into();
//! assert_eq!(*size_2.col_width, ch(1));
//! assert_eq!(*size_2.row_height, ch(2));
//!
//! // Note the order of the arguments don't matter below.
//! let size_3 = Dim::new(
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

// REVIEW: [ ] drop aliases and rename the original structs to the alias names
pub type Size = Dim;
pub type Width = ColWidth;
pub type Height = RowHeight;

#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Dim {
    // REVIEW: [ ] rename field to col_width
    pub col_width: ColWidth,
    // REVIEW: [ ] rename field to row_height
    pub row_height: RowHeight,
}

pub fn dim(arg: impl Into<Dim>) -> Dim { arg.into() }

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Ord, Eq, Hash)]
pub enum SufficientSize {
    IsLargeEnough,
    IsTooSmall,
}

// TODO: [ ] impl constructor, debug, ops for Dim (equivalent to r3bl_core::Size)

mod constructor {
    use super::*;

    impl Dim {
        pub fn new(arg: impl Into<Dim>) -> Self { arg.into() }
    }

    impl From<(ColWidth, RowHeight)> for Dim {
        fn from((width, height): (ColWidth, RowHeight)) -> Self {
            Dim {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl From<(RowHeight, ColWidth)> for Dim {
        fn from((height, width): (RowHeight, ColWidth)) -> Self {
            Dim {
                col_width: width,
                row_height: height,
            }
        }
    }

    impl Add<RowHeight> for ColWidth {
        type Output = Dim;

        fn add(self, rhs: RowHeight) -> Self::Output {
            Dim {
                col_width: self,
                row_height: rhs,
            }
        }
    }

    impl Add<ColWidth> for RowHeight {
        type Output = Dim;

        fn add(self, rhs: ColWidth) -> Self::Output {
            Dim {
                col_width: rhs,
                row_height: self,
            }
        }
    }
}

mod api {
    use super::*;

    impl Dim {
        pub fn fits_min_size(&self, min_size: impl Into<Dim>) -> SufficientSize {
            let size: Dim = min_size.into();
            let min_width = size.col_width;
            let min_height = size.row_height;

            match self.col_width < min_width || self.row_height < min_height {
                false => SufficientSize::IsLargeEnough,
                true => SufficientSize::IsTooSmall,
            }
        }
    }
}

mod debug {
    use super::*;

    impl Debug for Dim {
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
    use super::*;

    impl Sub<Dim> for Dim {
        type Output = Dim;

        fn sub(self, rhs: Dim) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width -= *rhs.col_width;
            *self_copy.row_height -= *rhs.row_height;
            self_copy
        }
    }

    impl Add<Dim> for Dim {
        type Output = Dim;

        fn add(self, rhs: Dim) -> Self::Output {
            let mut self_copy = self;
            *self_copy.col_width += *rhs.col_width;
            *self_copy.row_height += *rhs.row_height;
            self_copy
        }
    }

    impl SubAssign<ChUnit> for Dim {
        fn sub_assign(&mut self, other: ChUnit) {
            *self.col_width -= other;
            *self.row_height -= other;
        }
    }

    impl Sub<ChUnit> for Dim {
        type Output = Dim;

        fn sub(self, other: ChUnit) -> Self::Output {
            let mut self_copy = self;
            self_copy -= other;
            self_copy
        }
    }

    impl AddAssign<ChUnit> for Dim {
        fn add_assign(&mut self, other: ChUnit) {
            *self.col_width += other;
            *self.row_height += other;
        }
    }

    impl Add<ChUnit> for Dim {
        type Output = Dim;

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
        let size = dim(width(5) + height(10));
        assert_eq!(size.col_width, ColWidth(ch(5)));
        assert_eq!(*size.col_width, ch(5));
        assert_eq!(size.row_height, RowHeight(ch(10)));
        assert_eq!(*size.row_height, ch(10));
        let size_2 = dim(height(10) + width(5));

        assert!(matches!(size_2.col_width, ColWidth(_)));
        assert!(matches!(size_2.row_height, RowHeight(_)));
    }

    #[test]
    fn test_size_new() {
        // Order does not matter.
        let size = Dim::new((ColWidth::new(5), RowHeight::new(10)));
        assert_eq!(size.col_width, ColWidth(ch(5)));
        assert_eq!(*size.col_width, 5.into());
        assert_eq!(size.row_height, RowHeight(10.into()));
        assert_eq!(*size.row_height, ch(10));

        // Order does not matter.
        let size_2 = Dim::new((width(5), height(10)));
        assert!(matches!(size_2.col_width, ColWidth(_)));
        assert!(matches!(size_2.row_height, RowHeight(_)));
    }

    #[test]
    fn test_size_from() {
        // Order does not matter!
        let size: Dim = (ColWidth(ch(5)), RowHeight(ch(10))).into();
        let size_2: Dim = (RowHeight(ch(10)), ColWidth(ch(5))).into();

        assert_eq!(size.col_width, ColWidth(ch(5)));
        assert_eq!(*size.col_width, ch(5));
        assert_eq!(size.row_height, RowHeight(ch(10)));
        assert_eq!(*size.row_height, ch(10));

        assert_eq!(size, size_2);
    }

    #[test]
    fn test_size_add() {
        let size1 = Dim::new((ColWidth(5.into()), RowHeight(10.into())));
        let size2 = Dim::new((ColWidth::from(ch(3)), RowHeight::from(ch(4))));
        let result = size1 + size2;
        assert_eq!(result.col_width, ColWidth(8.into()));
        assert_eq!(*result.col_width, ch(8));
        assert_eq!(result.row_height, RowHeight(14.into()));
        assert_eq!(*result.row_height, ch(14));
    }

    #[test]
    fn test_size_sub() {
        let size1 = Dim::new((ColWidth(5.into()), RowHeight(10.into())));
        let size2 = Dim::new((ColWidth(3.into()), RowHeight(4.into())));
        let result = size1 - size2;
        assert_eq!(result.col_width, ColWidth(ch(2)));
        assert_eq!(result.row_height, RowHeight(ch(6)));
    }

    #[test]
    fn test_fits_min_size() {
        let size = width(5) + height(10);
        assert_eq!(
            size.fits_min_size(Dim::new((width(3), height(4)))),
            SufficientSize::IsLargeEnough
        );
        assert_eq!(
            size.fits_min_size(Dim::new((width(100), height(100)))),
            SufficientSize::IsTooSmall
        );
    }

    #[test]
    fn test_debug_fmt() {
        let size = Dim::new((width(5), height(10)));
        assert_eq!(format!("{:?}", size), "[w: 5, h: 10]");
    }

    #[test]
    fn test_ch_unit_sub_and_sub_assign() {
        let mut size0 = Dim::new((width(5), height(10)));
        size0 -= ch(3);
        assert_eq!(size0.col_width, ColWidth(ch(2)));
        assert_eq!(size0.row_height, RowHeight(ch(7)));

        let size1 = size0 - ch(1);
        assert_eq!(size1.col_width, ColWidth(ch(1)));
        assert_eq!(size1.row_height, RowHeight(ch(6)));
    }

    #[test]
    fn test_ch_unit_add_and_add_assign() {
        let mut size0 = Dim::new((width(5), height(10)));
        size0 += ch(3);
        assert_eq!(size0.col_width, ColWidth(ch(8)));
        assert_eq!(size0.row_height, RowHeight(ch(13)));

        let size1 = size0 + ch(1);
        assert_eq!(size1.col_width, ColWidth(ch(9)));
        assert_eq!(size1.row_height, RowHeight(ch(14)));
    }
}
