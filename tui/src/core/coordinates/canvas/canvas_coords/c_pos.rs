// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{CCol, CHeight, CRow, CSize, CWidth, c_col, c_row, c_size};
use crate::{ArrayBoundsCheck, ArrayOverflowResult, CursorBoundsCheck};
use std::ops::{Add, Sub};

/// Helper constructor for [`CPos`].
pub fn c_pos(col_val: impl Into<CCol>, row_val: impl Into<CRow>) -> CPos {
    CPos {
        col_index: c_col(col_val),
        row_index: c_row(row_val),
    }
}

/// Absolute 2D position (column index and row index) in the continuous storage buffer
/// space (64-bit [`Canvas`] domain).
///
/// Combines [`CCol`] and [`CRow`] to address an exact coordinate
/// location in document space.
///
/// See the [Canvas and Viewport concept] for details.
///
/// [`Canvas`]: mod@crate::core::coordinates::canvas
/// [Canvas and Viewport concept]: mod@crate::core::coordinates::canvas#canvas-and-viewport-concept
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default, Debug)]
pub struct CPos {
    pub col_index: CCol,
    pub row_index: CRow,
}

mod impl_canvas_pos {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl CPos {
        /// Mutates this [`CPos`] in place with the given value.
        pub fn set(&mut self, value: impl Into<Self>) { *self = value.into(); }

        /// Returns the underlying [`CPos`] value.
        #[must_use]
        pub fn get(&self) -> Self { *self }

        pub fn add_row_with_bounds(
            &mut self,
            arg_row_height: impl Into<CHeight>,
            arg_max_row_height: impl Into<CHeight>,
        ) {
            let value: CHeight = arg_row_height.into();
            let max: CHeight = arg_max_row_height.into();
            let new_row_index = self.row_index + value;
            self.row_index =
                if new_row_index.overflows(max) == ArrayOverflowResult::Overflowed {
                    // Handle zero height edge case: clamp to position 0
                    if max.as_usize() == 0 {
                        c_row(0)
                    } else {
                        max.eol_cursor_position() // Allow "after last row" position
                    }
                } else {
                    new_row_index
                };
        }
    }

    impl From<(CCol, CRow)> for CPos {
        fn from(val: (CCol, CRow)) -> CPos {
            CPos {
                col_index: val.0,
                row_index: val.1,
            }
        }
    }

    impl From<(CRow, CCol)> for CPos {
        fn from(val: (CRow, CCol)) -> CPos {
            CPos {
                row_index: val.0,
                col_index: val.1,
            }
        }
    }

    impl Add for CPos {
        type Output = Self;

        fn add(self, rhs: Self) -> Self {
            CPos {
                col_index: self.col_index + rhs.col_index,
                row_index: self.row_index + rhs.row_index,
            }
        }
    }

    impl Sub for CPos {
        type Output = CSize;

        fn sub(self, rhs: Self) -> Self::Output {
            c_size(
                self.col_index - rhs.col_index,
                self.row_index - rhs.row_index,
            )
        }
    }

    impl Add<CSize> for CPos {
        type Output = Self;

        fn add(self, rhs: CSize) -> Self {
            CPos {
                col_index: self.col_index + rhs.col_width,
                row_index: self.row_index + rhs.row_height,
            }
        }
    }

    impl Sub<CSize> for CPos {
        type Output = Self;

        fn sub(self, rhs: CSize) -> Self {
            CPos {
                col_index: self.col_index - rhs.col_width,
                row_index: self.row_index - rhs.row_height,
            }
        }
    }

    impl Add<CWidth> for CPos {
        type Output = Self;

        fn add(self, rhs: CWidth) -> Self {
            CPos {
                col_index: self.col_index + rhs,
                row_index: self.row_index,
            }
        }
    }

    impl Add<CHeight> for CPos {
        type Output = Self;

        fn add(self, rhs: CHeight) -> Self {
            CPos {
                col_index: self.col_index,
                row_index: self.row_index + rhs,
            }
        }
    }

    impl Add<(CWidth, CHeight)> for CPos {
        type Output = Self;

        fn add(self, rhs: (CWidth, CHeight)) -> Self {
            CPos {
                col_index: self.col_index + rhs.0,
                row_index: self.row_index + rhs.1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{c_height, c_width};

    #[test]
    fn test_get_and_set_methods() {
        let mut p = c_pos(8, 5);
        assert_eq!(p.get(), c_pos(8, 5));
        p.set(c_pos(20, 10));
        assert_eq!(p.get(), c_pos(20, 10));
    }

    #[test]
    fn test_canvas_pos_constructors_and_conversions() {
        let p1 = c_pos(10usize, 5usize);
        assert_eq!(p1.col_index.0, 10);
        assert_eq!(p1.row_index.0, 5);

        assert_eq!(CPos::from((c_col(10usize), c_row(5usize))), p1);
        assert_eq!(CPos::from((c_row(5usize), c_col(10usize))), p1);
    }

    #[test]
    fn test_canvas_pos_math_ops() {
        let p1 = c_pos(10usize, 5usize);
        let p2 = c_pos(3usize, 2usize);

        assert_eq!(p1 + p2, c_pos(13usize, 7usize));
        assert_eq!(p1 - p2, CSize::from((c_width(7usize), c_height(3usize))));

        // Underflow saturates to 0.
        let p_large = c_pos(20usize, 10usize);
        assert_eq!(
            p1 - p_large,
            CSize::from((c_width(0usize), c_height(0usize)))
        );

        let sz = c_size(80, 24);
        let offset_pos = p1 + sz;
        assert_eq!(offset_pos, c_pos(90usize, 29usize));

        let sub_pos = offset_pos - sz;
        assert_eq!(sub_pos, p1);

        // Subtraction saturating at zero.
        let underflow_pos = p1 - c_size(20, 20);
        assert_eq!(underflow_pos, c_pos(0usize, 0usize));

        let w = c_width(80);
        let h = c_height(24);
        assert_eq!(p1 + w, c_pos(90usize, 5usize));
        assert_eq!(p1 + h, c_pos(10usize, 29usize));
        assert_eq!(p1 + (w, h), c_pos(90usize, 29usize));
    }

    #[test]
    fn test_add_row_with_bounds() {
        // Within bounds
        let mut p = c_pos(5usize, 5usize);
        p.add_row_with_bounds(c_height(10usize), c_height(20usize));
        assert_eq!(p.row_index, c_row(15usize));

        // Overflow clamped to max eol_cursor_position
        let mut p_over = c_pos(5usize, 15usize);
        p_over.add_row_with_bounds(c_height(10usize), c_height(20usize));
        assert_eq!(p_over.row_index, c_row(20usize));

        // Zero height clamped to row 0
        let mut p_zero = c_pos(5usize, 5usize);
        p_zero.add_row_with_bounds(c_height(10usize), c_height(0usize));
        assert_eq!(p_zero.row_index, c_row(0usize));
    }
}
