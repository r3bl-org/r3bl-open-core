// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{VPHeight, VPIndex, vp_height};
use crate::{ChUnit, generate_index_type_impl};
use std::hash::Hash;

/// The vertical index in a grid of characters, starting at 0, which is the first row.
///
/// This is one part of a [`VPPos`] position and is different from [`VPHeight`], which
/// is one part of a [`VPSize`]. You can use the [`vp_row()`] to create a new instance.
///
/// # Examples
/// ```
/// use r3bl_tui::{VPRow, vp_row};
/// let row = vp_row(5);
/// let row = VPRow::new(5);
/// ```
///
/// [`vp_row()`]: crate::vp_row
/// [`VPHeight`]: crate::VPHeight
/// [`VPPos`]: crate::core::VPPos
/// [`VPSize`]: crate::VPSize
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPRow(ChUnit);
generate_index_type_impl!(
    VPRow,     // Add impl for this type
    VPHeight,  // Use this associated type
    vp_row,    // Make this constructor fn
    vp_height  // Use this constructor fn
);

impl From<VPIndex> for VPRow {
    fn from(index: VPIndex) -> VPRow { VPRow(*index) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    #[test]
    fn test_row_index_add() {
        let row1 = VPRow::from(ch(5));
        let row2 = VPRow::new(3);
        let result = row1 + row2;
        assert_eq!(result, VPRow(ch(8)));
        assert_eq!(*result, ch(8));
    }

    #[test]
    fn test_row_index_sub() {
        let row1 = VPRow::from(ch(5));
        let row2 = VPRow::new(3);
        let result = row1 - row2;
        assert_eq!(result, VPRow::new(2));
        assert_eq!(*result, ch(2));
    }

    #[test]
    fn test_row_index_sub_assign_add_assign() {
        let mut row0 = vp_row(5);
        let row2 = vp_row(3);

        row0 -= row2;
        assert_eq!(row0, vp_row(2));
        assert_eq!(*row0, ch(2));

        row0 += row2;
        assert_eq!(row0, vp_row(5));
        assert_eq!(*row0, ch(5));
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut row = VPRow::new(5);
        assert_eq!(*row, ch(5));
        *row = ch(10);
        assert_eq!(*row, ch(10));
    }

    #[test]
    fn test_height_mul() {
        let row = VPRow::new(5);
        let height = VPHeight::new(3u16);
        let result = row * height;
        assert_eq!(result, VPRow::new(15));
        assert_eq!(*result, ch(15));
    }

    #[test]
    fn test_height_add() {
        // Add.
        {
            let row = VPRow::new(5);
            let height = VPHeight::new(3u16);
            let result = row + height;
            assert_eq!(result, VPRow::new(8));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut row = VPRow::new(5);
            let height = VPHeight::new(3u16);
            row += height;
            assert_eq!(row, VPRow::new(8));
            assert_eq!(*row, ch(8));
        }
    }

    #[test]
    fn test_height_sub() {
        // Sub.
        {
            let row_idx = VPRow::new(5);
            let ht = VPHeight::new(3u16);
            let res = row_idx - ht;
            assert_eq!(res, vp_row(2));
            assert_eq!(*res, ch(2));
        }
        // SubAssign.
        {
            let mut row = VPRow::new(5);
            let height = VPHeight::new(3u16);
            row -= height;
            assert_eq!(row, VPRow::new(2));
            assert_eq!(*row, ch(2));
        }
    }

    #[test]
    fn test_as_usize() {
        let row = VPRow::new(5);
        assert_eq!(row.as_usize(), 5);
    }

    #[test]
    fn test_convert_to_length() {
        let row = VPRow::new(5);
        let ht = row.convert_to_length();
        assert_eq!(ht, vp_height(6));
        assert_eq!(*ht, ch(6));
    }

    #[test]
    fn test_as_u16() {
        let row = VPRow::new(5);
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_from_usize() {
        assert_eq!(VPRow::from(5), vp_row(5));
    }

    #[test]
    fn test_row_index_add_i32() {
        // Add positive i32.
        {
            let row_idx = vp_row(5);
            let result = row_idx + 3i32;
            assert_eq!(result, vp_row(8));
        }
        // Add negative i32 (should be treated as 0).
        {
            let row_idx = vp_row(5);
            let result = row_idx + -3i32;
            assert_eq!(result, vp_row(5)); // -3 becomes 0
        }
        // Add zero.
        {
            let row_idx = vp_row(5);
            let result = row_idx + 0i32;
            assert_eq!(result, vp_row(5));
        }
    }

    #[test]
    fn test_row_index_sub_i32() {
        // Subtract positive i32.
        {
            let row_idx = vp_row(10);
            let result = row_idx - 3i32;
            assert_eq!(result, vp_row(7));
        }
        // Subtract larger value (should saturate to 0).
        {
            let row_idx = vp_row(5);
            let result = row_idx - 10i32;
            assert_eq!(result, vp_row(0));
        }
        // Subtract negative i32 (should be treated as 0, no change).
        {
            let row_idx = vp_row(5);
            let result = row_idx - -3i32;
            assert_eq!(result, vp_row(5)); // -3 becomes 0
        }
        // Subtract zero.
        {
            let row_idx = vp_row(5);
            let result = row_idx - 0i32;
            assert_eq!(result, vp_row(5));
        }
    }

    #[test]
    fn test_row_index_add_assign_i32() {
        // AddAssign positive i32.
        {
            let mut row_idx = vp_row(5);
            row_idx += 3i32;
            assert_eq!(row_idx, vp_row(8));
        }
        // AddAssign negative i32 (should be treated as 0).
        {
            let mut row_idx = vp_row(5);
            row_idx += -3i32;
            assert_eq!(row_idx, vp_row(5)); // -3 becomes 0
        }
    }

    #[test]
    fn test_row_index_sub_assign_i32() {
        // SubAssign positive i32.
        {
            let mut row_idx = vp_row(10);
            row_idx -= 3i32;
            assert_eq!(row_idx, vp_row(7));
        }
        // SubAssign larger value (should saturate to 0).
        {
            let mut row_idx = vp_row(5);
            row_idx -= 10i32;
            assert_eq!(row_idx, vp_row(0));
        }
        // SubAssign negative i32 (should be treated as 0, no change).
        {
            let mut row_idx = vp_row(5);
            row_idx -= -3i32;
            assert_eq!(row_idx, vp_row(5)); // -3 becomes 0
        }
    }
}
