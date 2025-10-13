// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{RowHeight, height};
use crate::{ChUnit, Index, generate_index_type_impl};
use std::hash::Hash;

/// The vertical index in a grid of characters, starting at 0, which is the first row.
///
/// This is one part of a [`Pos`] position and is different from [`RowHeight`], which is
/// one part of a [`Size`]. You can use the [`row()`] to create a new instance.
///
/// # Examples
/// ```
/// use r3bl_tui::{RowIndex, row};
/// let row = row(5);
/// let row = RowIndex::new(5);
/// ```
///
/// [`Pos`]: crate::Pos
/// [`RowHeight`]: crate::RowHeight
/// [`Size`]: crate::Size
/// [`row()`]: crate::row
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct RowIndex(pub ChUnit);
generate_index_type_impl!(
    /* Add impl for this type */ RowIndex,
    /* Use this associated type */ RowHeight,
    /* Make this constructor fn */ row, /* Use this constructor fn */ height
);

impl From<Index> for RowIndex {
    fn from(index: Index) -> Self { RowIndex(index.0) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ch;

    #[test]
    fn test_row_index_add() {
        let row1 = RowIndex::from(ch(5));
        let row2 = RowIndex::new(3);
        let result = row1 + row2;
        assert_eq!(result, RowIndex(ch(8)));
        assert_eq!(*result, ch(8));
    }

    #[test]
    fn test_row_index_sub() {
        let row1 = RowIndex::from(ch(5));
        let row2 = RowIndex::new(3);
        let result = row1 - row2;
        assert_eq!(result, RowIndex::new(2));
        assert_eq!(*result, ch(2));
    }

    #[test]
    fn test_row_index_sub_assign_add_assign() {
        let mut row0 = row(5);
        let row2 = row(3);

        row0 -= row2;
        assert_eq!(row0, row(2));
        assert_eq!(*row0, ch(2));

        row0 += row2;
        assert_eq!(row0, row(5));
        assert_eq!(*row0, ch(5));
    }

    #[test]
    fn test_deref_and_deref_mut() {
        let mut row = RowIndex::new(5);
        assert_eq!(*row, ch(5));
        *row = ch(10);
        assert_eq!(*row, ch(10));
    }

    #[test]
    fn test_height_mul() {
        let row = RowIndex::new(5);
        let height = RowHeight::new(3);
        let result = row * height;
        assert_eq!(result, RowIndex::new(15));
        assert_eq!(*result, ch(15));
    }

    #[test]
    fn test_height_add() {
        // Add.
        {
            let row = RowIndex::new(5);
            let height = RowHeight::new(3);
            let result = row + height;
            assert_eq!(result, RowIndex::new(8));
            assert_eq!(*result, ch(8));
        }
        // AddAssign.
        {
            let mut row = RowIndex::new(5);
            let height = RowHeight::new(3);
            row += height;
            assert_eq!(row, RowIndex::new(8));
            assert_eq!(*row, ch(8));
        }
    }

    #[test]
    fn test_height_sub() {
        // Sub.
        {
            let row_idx = RowIndex::new(5);
            let ht = RowHeight::new(3);
            let res = row_idx - ht;
            assert_eq!(res, row(2));
            assert_eq!(*res, ch(2));
        }
        // SubAssign.
        {
            let mut row = RowIndex::new(5);
            let height = RowHeight::new(3);
            row -= height;
            assert_eq!(row, RowIndex::new(2));
            assert_eq!(*row, ch(2));
        }
    }

    #[test]
    fn test_as_usize() {
        let row = RowIndex::new(5);
        assert_eq!(row.as_usize(), 5);
    }

    #[test]
    fn test_convert_to_length() {
        let row = RowIndex::new(5);
        let ht = row.convert_to_length();
        assert_eq!(ht, height(6));
        assert_eq!(*ht, ch(6));
    }

    #[test]
    fn test_as_u16() {
        let row = RowIndex::new(5);
        assert_eq!(row.as_u16(), 5);
    }

    #[test]
    fn test_from_usize() {
        assert_eq!(RowIndex::from(5usize), row(5));
    }

    #[test]
    fn test_row_index_add_i32() {
        // Add positive i32.
        {
            let row_idx = row(5);
            let result = row_idx + 3i32;
            assert_eq!(result, row(8));
        }
        // Add negative i32 (should be treated as 0).
        {
            let row_idx = row(5);
            let result = row_idx + -3i32;
            assert_eq!(result, row(5)); // -3 becomes 0
        }
        // Add zero.
        {
            let row_idx = row(5);
            let result = row_idx + 0i32;
            assert_eq!(result, row(5));
        }
    }

    #[test]
    fn test_row_index_sub_i32() {
        // Subtract positive i32.
        {
            let row_idx = row(10);
            let result = row_idx - 3i32;
            assert_eq!(result, row(7));
        }
        // Subtract larger value (should saturate to 0).
        {
            let row_idx = row(5);
            let result = row_idx - 10i32;
            assert_eq!(result, row(0));
        }
        // Subtract negative i32 (should be treated as 0, no change).
        {
            let row_idx = row(5);
            let result = row_idx - -3i32;
            assert_eq!(result, row(5)); // -3 becomes 0
        }
        // Subtract zero.
        {
            let row_idx = row(5);
            let result = row_idx - 0i32;
            assert_eq!(result, row(5));
        }
    }

    #[test]
    fn test_row_index_add_assign_i32() {
        // AddAssign positive i32.
        {
            let mut row_idx = row(5);
            row_idx += 3i32;
            assert_eq!(row_idx, row(8));
        }
        // AddAssign negative i32 (should be treated as 0).
        {
            let mut row_idx = row(5);
            row_idx += -3i32;
            assert_eq!(row_idx, row(5)); // -3 becomes 0
        }
    }

    #[test]
    fn test_row_index_sub_assign_i32() {
        // SubAssign positive i32.
        {
            let mut row_idx = row(10);
            row_idx -= 3i32;
            assert_eq!(row_idx, row(7));
        }
        // SubAssign larger value (should saturate to 0).
        {
            let mut row_idx = row(5);
            row_idx -= 10i32;
            assert_eq!(row_idx, row(0));
        }
        // SubAssign negative i32 (should be treated as 0, no change).
        {
            let mut row_idx = row(5);
            row_idx -= -3i32;
            assert_eq!(row_idx, row(5)); // -3 becomes 0
        }
    }
}
