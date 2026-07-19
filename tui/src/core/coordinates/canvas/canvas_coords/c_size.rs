// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CHeight, CWidth, c_height, c_width};
use crate::{NarrowingCastToU16, VPSize};
use std::ops::{Add, Mul, Sub};

/// Helper constructor for [`CSize`].
pub fn c_size(col_w: impl Into<CWidth>, row_h: impl Into<CHeight>) -> CSize {
    CSize {
        col_width: c_width(col_w),
        row_height: c_height(row_h),
    }
}

/// 2D extent (column width and row height) in the continuous storage buffer space (64-bit
/// [`Canvas`] domain).
///
/// Combines [`CWidth`] and [`CHeight`] to represent 2D dimensions in
/// document space.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CSize {
    pub col_width: CWidth,
    pub row_height: CHeight,
}

mod impl_canvas_size {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CSize {
        #[inline]
        #[must_use]
        pub fn new((col_width, row_height): (CWidth, CHeight)) -> Self {
            Self {
                col_width,
                row_height,
            }
        }
    }

    impl TryFrom<CSize> for VPSize {
        type Error = std::num::TryFromIntError;

        fn try_from(val: CSize) -> Result<Self, Self::Error> {
            let width = val.col_width.as_usize().as_u16_narrowing();
            let height = val.row_height.as_usize().as_u16_narrowing();
            Ok(VPSize {
                col_width: width.into(),
                row_height: height.into(),
            })
        }
    }

    impl From<VPSize> for CSize {
        fn from(size: VPSize) -> CSize {
            CSize {
                col_width: CWidth(size.col_width.as_usize()),
                row_height: CHeight(size.row_height.as_usize()),
            }
        }
    }

    impl From<(CWidth, CHeight)> for CSize {
        fn from(val: (CWidth, CHeight)) -> CSize {
            CSize {
                col_width: val.0,
                row_height: val.1,
            }
        }
    }

    impl Add for CSize {
        type Output = Self;

        fn add(self, rhs: Self) -> Self {
            CSize {
                col_width: self.col_width + rhs.col_width,
                row_height: self.row_height + rhs.row_height,
            }
        }
    }

    impl Sub for CSize {
        type Output = Self;

        fn sub(self, rhs: Self) -> Self {
            CSize {
                col_width: self.col_width - rhs.col_width,
                row_height: self.row_height - rhs.row_height,
            }
        }
    }

    impl Mul<usize> for CSize {
        type Output = Self;

        fn mul(self, rhs: usize) -> Self {
            CSize {
                col_width: self.col_width * rhs,
                row_height: self.row_height * rhs,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VPSize, vp_height, vp_width};

    #[test]
    fn test_canvas_size_and_math_ops() {
        let sz = c_size(80, 24);

        assert_eq!(c_size(80, 24), sz);

        let size: VPSize = sz.try_into().expect("conversion error");
        assert_eq!(size, VPSize::new((vp_width(80), vp_height(24))));

        let converted_back = CSize::from(size);
        assert_eq!(converted_back.col_width, c_width(80));
        assert_eq!(converted_back.row_height, c_height(24));

        // CSize::new and tuple conversions.
        let sz_tuple = CSize::from((c_width(80), c_height(24)));
        assert_eq!(sz_tuple, sz);
        let sz_new = CSize::new((c_width(80), c_height(24)));
        assert_eq!(sz_new, sz);

        // CSize addition, subtraction, and multiplication.
        let sz1 = c_size(10usize, 20usize);
        let sz2 = c_size(5usize, 4usize);
        assert_eq!(sz1 + sz2, c_size(15usize, 24usize));
        assert_eq!(sz1 - c_size(30usize, 30usize), c_size(0usize, 0usize));
        assert_eq!(sz1 * 3usize, c_size(30usize, 60usize));
    }
}
