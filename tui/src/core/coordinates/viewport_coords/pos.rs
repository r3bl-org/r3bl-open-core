// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ArrayBoundsCheck, ArrayOverflowResult, ChUnit, CursorBoundsCheck, VPCol,
            VPHeight, VPRow, VPSize, VPWidth, ch, vp_col, vp_row};
use std::{fmt::{Debug, Formatter},
          ops::{Add, AddAssign, Mul, Sub, SubAssign}};

/// [`VPPos`] is a struct that holds the `row` and `col` indices of a character in a text
/// buffer. [`VPRow`] and [`VPCol`] are the types of the `row` and `col` indices
/// respectively. This ensures that it isn't possible to use a `col` when you intended to
/// use a `row` and vice versa.
///
/// > 💡 **See also**: For complete workflows showing [`VPPos`] used with other coordinate
/// > types (e.g., [`VT-100`] conversions, bounds checking), see the [coordinates module
/// > documentation](crate::coordinates).
///
/// Here is a visual representation of how position and sizing work for the layout engine.
///
/// ```text
///     0   4    9    1    2    2
///                   4    0    5
///    ┌────┴────┴────┴────┴────┴── col
///  0 ┤     ╭─────────────╮
///  1 ┤     │ origin pos: │
///  2 ┤     │ [5, 0]      │
///  3 ┤     │ size:       │
///  4 ┤     │ [16, 5]     │
///  5 ┤     ╰─────────────╯
///    │
///   row
/// ```
///
/// # The many ways to create one
///
/// This API uses the `impl Into<struct>` pattern and [Add] `+` operator overloading to
/// allow for easy conversion between [`ChUnit`] and [`VPRow`]/[`VPCol`].
/// - You can use [`vp_pos()`] function and pass it a [`VPRow`] and [`VPCol`] tuple, or
///   pass a sequence of them with the [Add] `+` operator.
/// - Just using the [Add] `+` operator:
///     - You can use [Add] to compose: [`VPRow`] + [`VPCol`], into: a `VPPos`.
///     - You can use [Add] to compose: [`VPCol`] + [`VPRow`], into: a `VPPos`.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{
///     ch,
///     VPPos, VPRow, VPCol,
///     vp_row, vp_col, vp_pos
/// };
///
/// // So many different ways to create a VPPos.
/// let pos_1: VPPos = (vp_row(2) + vp_col(3)).into();
/// let pos_1: VPPos = (vp_row(2), vp_col(3)).into();
/// let pos_1: VPPos = (vp_col(3), vp_row(2)).into();
///
/// // Create a VPPos from a VPPos.
/// let pos_2: VPPos = (vp_row(2) + vp_col(3)).into();
/// let vp_pos_1: VPPos = pos_2.into();
///
/// assert_eq!(*pos_1.row_index, ch(2));
/// assert_eq!(*pos_1.col_index, ch(3));
///
/// let pos_a: VPPos = (vp_row(4) + vp_col(10)).into();
/// let pos_b: VPPos = (vp_row(2) + vp_col(6)).into();
///
/// let pos_sum = pos_a + pos_b;
/// assert_eq!(*pos_sum.row_index, ch(6));
/// assert_eq!(*pos_sum.col_index, ch(16));
///
/// let pos_diff = pos_a - pos_b;
/// assert_eq!(*pos_diff.row_index, ch(2));
/// assert_eq!(*pos_diff.col_index, ch(4));
/// ```
///
/// [`ChUnit`]: ChUnit
/// [`vp_pos()`]: crate::vp_pos()
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct VPPos {
    /// Row index, 0-based.
    pub row_index: VPRow,
    /// Column index, 0-based.
    pub col_index: VPCol,
}

#[inline]
pub fn vp_pos(col_val: impl Into<VPCol>, row_val: impl Into<VPRow>) -> VPPos {
    VPPos {
        col_index: vp_col(col_val),
        row_index: vp_row(row_val),
    }
}

mod constructor {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl VPPos {
        #[inline]
        pub fn new(arg_pos: impl Into<VPPos>) -> Self { arg_pos.into() }
    }

    impl From<(VPRow, VPCol)> for VPPos {
        #[inline]
        fn from((row, col): (VPRow, VPCol)) -> VPPos {
            VPPos {
                row_index: row,
                col_index: col,
            }
        }
    }

    impl From<(VPCol, VPRow)> for VPPos {
        #[inline]
        fn from((col, row): (VPCol, VPRow)) -> VPPos {
            VPPos {
                row_index: row,
                col_index: col,
            }
        }
    }

    impl Add<VPCol> for VPRow {
        type Output = VPPos;

        fn add(self, rhs: VPCol) -> Self::Output {
            VPPos {
                row_index: self,
                col_index: rhs,
            }
        }
    }

    impl Add<VPRow> for VPCol {
        type Output = VPPos;

        fn add(self, rhs: VPRow) -> Self::Output {
            VPPos {
                row_index: rhs,
                col_index: self,
            }
        }
    }
}

mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<VPPos> for VPRow {
        fn from(pos: VPPos) -> VPRow { pos.row_index }
    }

    impl From<VPPos> for VPCol {
        fn from(pos: VPPos) -> VPCol { pos.col_index }
    }
}

mod dimension_arithmetic_operators {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // Dim is equivalent to (ColWidthCount, RowHeightCount).
    impl Mul<VPSize> for VPPos {
        type Output = VPPos;

        fn mul(self, rhs: VPSize) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.row_height;
            self_copy.col_index = self.col_index * rhs.col_width;
            self_copy
        }
    }

    // (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount) is equivalent to
    // Dim.
    impl Mul<(VPWidth, VPHeight)> for VPPos {
        type Output = VPPos;

        fn mul(self, rhs: (VPWidth, VPHeight)) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.1;
            self_copy.col_index = self.col_index * rhs.0;
            self_copy
        }
    }

    // (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount) is equivalent to
    // Dim.
    impl Mul<(VPHeight, VPWidth)> for VPPos {
        type Output = VPPos;

        fn mul(self, rhs: (VPHeight, VPWidth)) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.0;
            self_copy.col_index = self.col_index * rhs.1;
            self_copy
        }
    }

    impl Add<VPSize> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPSize) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index + rhs.row_height;
            self_copy.col_index = self.col_index + rhs.col_width;
            self_copy
        }
    }

    impl Sub<VPSize> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPSize) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = {
                let it = self.row_index - rhs.row_height;
                vp_row(*it)
            };
            self_copy.col_index = {
                let it = self.col_index - rhs.col_width;
                vp_col(*it)
            };
            self_copy
        }
    }

    impl AddAssign<VPSize> for VPPos {
        fn add_assign(&mut self, rhs: VPSize) { *self = *self + rhs; }
    }

    impl SubAssign<VPSize> for VPPos {
        fn sub_assign(&mut self, rhs: VPSize) { *self = *self - rhs; }
    }

    impl Add<VPPos> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPPos) -> Self::Output {
            let mut self_copy = self;
            *self_copy.row_index += *rhs.row_index;
            *self_copy.col_index += *rhs.col_index;
            self_copy
        }
    }

    impl Sub<VPPos> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPPos) -> Self::Output {
            let mut self_copy = self;
            *self_copy.row_index -= *rhs.row_index;
            *self_copy.col_index -= *rhs.col_index;
            self_copy
        }
    }

    impl AddAssign<VPPos> for VPPos {
        fn add_assign(&mut self, rhs: VPPos) { *self = *self + rhs; }
    }

    impl SubAssign<VPPos> for VPPos {
        fn sub_assign(&mut self, rhs: VPPos) { *self = *self - rhs; }
    }

    impl Add<VPWidth> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPWidth) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index = self.col_index + rhs;
            self_copy
        }
    }

    impl AddAssign<VPWidth> for VPPos {
        fn add_assign(&mut self, rhs: VPWidth) { *self = *self + rhs; }
    }

    impl Sub<VPWidth> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPWidth) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index -= rhs;
            self_copy
        }
    }

    impl SubAssign<VPWidth> for VPPos {
        fn sub_assign(&mut self, rhs: VPWidth) { *self = *self - rhs; }
    }

    impl Add<VPCol> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPCol) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index = self.col_index + rhs;
            self_copy
        }
    }

    impl Add<VPRow> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPRow) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index + rhs;
            self_copy
        }
    }

    impl Sub<VPCol> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPCol) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index = self.col_index - rhs;
            self_copy
        }
    }

    impl Sub<VPRow> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPRow) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index - rhs;
            self_copy
        }
    }

    impl Add<VPHeight> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: VPHeight) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index + rhs;
            self_copy
        }
    }

    impl Sub<VPHeight> for VPPos {
        type Output = VPPos;

        fn sub(self, rhs: VPHeight) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index -= rhs;
            self_copy
        }
    }

    impl AddAssign<VPHeight> for VPPos {
        fn add_assign(&mut self, rhs: VPHeight) { *self = *self + rhs; }
    }

    impl SubAssign<VPHeight> for VPPos {
        fn sub_assign(&mut self, rhs: VPHeight) { *self = *self - rhs; }
    }
}

mod numeric_arithmetic_operators {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl AddAssign<ChUnit> for VPPos {
        fn add_assign(&mut self, rhs: ChUnit) {
            *self.row_index += rhs;
            *self.col_index += rhs;
        }
    }

    impl Add<ChUnit> for VPPos {
        type Output = VPPos;

        fn add(self, rhs: ChUnit) -> Self {
            let mut self_copy = self;
            self_copy += rhs;
            self_copy
        }
    }
}

mod api {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // Reset API.
    impl VPPos {
        /// Reset col and row index to `0`.
        #[inline]
        pub fn reset(&mut self) {
            *self.col_index = ch(0);
            *self.row_index = ch(0);
        }

        /// Reset row index to `0`.
        #[inline]
        pub fn reset_row(&mut self) { *self.row_index = ch(0); }

        /// Reset col index to `0`.
        #[inline]
        pub fn reset_col(&mut self) { *self.col_index = ch(0); }
    }

    // Row index API.
    impl VPPos {
        /// Sets row index to `value`.
        #[inline]
        pub fn set_row(&mut self, arg_row_index: impl Into<VPRow>) {
            self.row_index = arg_row_index.into();
        }

        /// Increment row index by `value`.
        #[inline]
        pub fn add_row(&mut self, arg_row_index: impl Into<VPHeight>) {
            let value: VPHeight = arg_row_index.into();
            *self.row_index += *value;
        }

        /// Increment row index by `value`, while making sure it will never exceed
        /// `max_row`.
        #[allow(clippy::return_self_not_must_use)]
        pub fn add_row_with_bounds(
            &mut self,
            arg_row_height: impl Into<VPHeight>,
            arg_max_row_height: impl Into<VPHeight>,
        ) {
            let value: VPHeight = arg_row_height.into();
            let max: VPHeight = arg_max_row_height.into();
            let new_row_index = self.row_index + value;
            self.row_index =
                if new_row_index.overflows(max) == ArrayOverflowResult::Overflowed {
                    // Handle zero height edge case: clamp to position 0
                    if max.is_empty() {
                        vp_row(0)
                    } else {
                        max.eol_cursor_position() // Allow "after last row" position
                    }
                } else {
                    new_row_index
                };
        }

        /// Decrement row index by `value`.
        #[inline]
        pub fn sub_row(&mut self, arg_row_height: impl Into<VPHeight>) {
            let value: VPHeight = arg_row_height.into();
            *self.row_index -= *value;
        }
    }

    // Col index API.
    impl VPPos {
        /// Sets col index to `value`.
        #[inline]
        pub fn set_col(&mut self, arg_col_index: impl Into<VPCol>) {
            let value: VPCol = arg_col_index.into();
            self.col_index = value;
        }

        /// Increment col index by `value`. Returns a copy of `Pos`.
        #[allow(clippy::return_self_not_must_use)]
        #[inline]
        pub fn add_col(&mut self, arg_col_width: impl Into<VPWidth>) -> Self {
            let width: VPWidth = arg_col_width.into();
            *self.col_index += *width;
            *self
        }

        /// Increment col index by `col_amt`, while making sure it will never exceed
        /// `max_col_amt`. This function is not concerned with scrolling.
        ///
        /// Note that a caret is allowed to "go past" the end of the max index, so max
        /// index + 1 is a valid position.
        ///
        /// - Let's assume the caret is represented by "░".
        /// - Think about typing "hello", and you expected the caret "░" to go past the
        ///   end of the string "hello░".
        /// - So the caret's col index is 5 in this case.
        ///
        /// ```text
        /// R ┌──────────┐
        /// 0 ▸hello░    │
        ///   └─────▴────┘
        ///   C0123456789
        /// ```
        ///
        /// Keep in mind these dynamics change when introducing scrolling, but this struct
        /// does not take scrolling into account. See
        /// [`scroll_editor_content`] for that.
        ///
        /// [`scroll_editor_content`]: crate::editor_engine::scroll_editor_content
        pub fn add_col_with_bounds(
            &mut self,
            arg_col_width: impl Into<VPWidth>,
            arg_max_col_width: impl Into<VPWidth>,
        ) {
            let value: VPWidth = arg_col_width.into();
            let max: VPWidth = arg_max_col_width.into();
            let new_col_index = self.col_index + value;
            self.col_index =
                if new_col_index.overflows(max) == ArrayOverflowResult::Overflowed {
                    // Handle zero width edge case: clamp to position 0
                    if max.is_empty() {
                        vp_col(0)
                    } else {
                        max.eol_cursor_position() // Allow "after last character" position
                    }
                } else {
                    new_col_index
                };
        }

        /// Clip col index to `max_col` if it exceeds it.
        pub fn clip_col_to_bounds(&mut self, arg_max_col_width: impl Into<VPWidth>) {
            let max: VPWidth = arg_max_col_width.into();
            if self.col_index.overflows(max) == ArrayOverflowResult::Overflowed {
                // Handle zero width edge case: clamp to position 0
                if max.is_empty() {
                    self.col_index = vp_col(0);
                } else {
                    self.col_index = max.eol_cursor_position(); // Allow "after last character" position
                }
            }
        }

        /// Decrement col index by `value`.
        pub fn sub_col(&mut self, arg_col_width: impl Into<VPWidth>) {
            let value: VPWidth = arg_col_width.into();
            *self.col_index -= *value;
        }
    }
}

mod debug {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for VPPos {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Pos [c: {a:?}, r: {b:?}]",
                a = *self.col_index,
                b = *self.row_index
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vp_height, vp_width};
    use std::fmt::Write;

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_api() {
        // Constructor.
        {
            let pos_0 = vp_row(1) + vp_col(2);
            assert_eq!(*pos_0.row_index, ch(1));
            assert_eq!(*pos_0.col_index, ch(2));

            let pos_1 = vp_row(1) + vp_col(2);
            assert_eq!(*pos_1.row_index, ch(1));
            assert_eq!(*pos_1.col_index, ch(2));

            let pos_2 = vp_col(2) + vp_row(1);
            assert_eq!(*pos_2.row_index, ch(1));
            assert_eq!(*pos_2.col_index, ch(2));
        }

        // Methods.
        {
            let row_idx = VPRow::new(ch(1));
            let col_idx = VPCol::new(ch(2));
            let wid = VPWidth::new(ch(3));

            let mut pos: VPPos = (col_idx, row_idx).into();
            assert_eq!(*pos.row_index, ch(1));
            assert_eq!(*pos.col_index, ch(2));

            pos.reset();
            assert_eq!(*pos.row_index, ch(0));
            assert_eq!(*pos.col_index, ch(0));

            *pos.row_index = ch(1);
            *pos.col_index = ch(2);
            assert_eq!(*pos.row_index, ch(1));
            assert_eq!(*pos.col_index, ch(2));

            pos.reset_col();
            assert_eq!(*pos.col_index, ch(0));

            pos.set_col(col_idx);
            assert_eq!(*pos.col_index, ch(2));

            pos.add_col(wid);
            assert_eq!(*pos.col_index, ch(5));

            pos.add_col_with_bounds(wid, wid);
            assert_eq!(*pos.col_index, ch(3));

            pos.clip_col_to_bounds({
                let col_idx = wid - vp_width(1);
                vp_width(*col_idx)
            });
            assert_eq!(*pos.col_index, ch(2));

            pos.sub_col(vp_width(1));
            assert_eq!(*pos.col_index, ch(1));

            pos.sub_col(vp_width(10));
            assert_eq!(*pos.col_index, ch(0));

            pos.reset_row();
            assert_eq!(*pos.row_index, ch(0));

            pos.set_row(row_idx);
            assert_eq!(*pos.row_index, ch(1));

            pos.add_row(vp_height(ch(3)));
            assert_eq!(*pos.row_index, ch(4));

            pos.add_row_with_bounds(vp_height(ch(10)), vp_height(ch(5)));
            assert_eq!(*pos.row_index, ch(5));

            pos.sub_row(vp_height(ch(2)));
            assert_eq!(*pos.row_index, ch(3));

            pos.sub_row(vp_height(ch(10)));
            assert_eq!(*pos.row_index, ch(0));
        }

        // Debug Pos.
        {
            let pos = VPPos::new((VPCol::new(ch(2)), VPRow::new(ch(1))));
            let mut acc = String::new();
            // We don't care about the result of this operation.
            write!(acc, "{pos:?}").ok();
            assert_eq!(acc, "Pos [c: 2, r: 1]");
        }

        // Mul (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount).
        {
            let pos = VPPos::new((vp_row(1), vp_col(2)));

            let pos_1 = pos * (vp_height(ch(2)), vp_width(ch(2)));
            assert_eq!(*pos_1.row_index, ch(2));
            assert_eq!(*pos_1.col_index, ch(4));

            let pos_2 = pos * (vp_width(ch(2)), vp_height(ch(2)));
            assert_eq!(*pos_2.row_index, ch(2));
            assert_eq!(*pos_2.col_index, ch(4));
        }

        // Add, Sub Dim.
        {
            let pos = VPPos::new((vp_row(1), vp_col(2)));
            let dim: VPSize = (vp_width(ch(2)), vp_height(ch(2))).into();

            let pos_1 = pos + dim;
            assert_eq!(*pos_1.row_index, ch(3));
            assert_eq!(*pos_1.col_index, ch(4));

            let pos_2 = pos_1 - dim;
            assert_eq!(*pos_2.row_index, ch(1));
            assert_eq!(*pos_2.col_index, ch(2));
        }

        // AddAssign, SubAssign Dim.
        {
            let mut pos = VPPos::new((VPRow::new(ch(1)), VPCol::new(ch(2))));
            pos += VPSize::new((vp_width(ch(2)), vp_height(ch(2))));
            assert_eq!(*pos.row_index, ch(3));
            assert_eq!(*pos.col_index, ch(4));

            pos -= VPSize::new((vp_width(ch(2)), vp_height(ch(2))));
            assert_eq!(*pos.row_index, ch(1));
            assert_eq!(*pos.col_index, ch(2));
        }

        // Add, Sub Pos.
        {
            let pos = VPPos::new((vp_row(2), vp_col(2)));
            let pos_1 = pos - VPPos::new((vp_row(1), vp_col(1)));
            assert_eq!(*pos_1.row_index, ch(1));
            assert_eq!(*pos_1.col_index, ch(1));

            let pos_2 = pos + VPPos::new((vp_row(1), vp_col(1)));
            assert_eq!(*pos_2.row_index, ch(3));
            assert_eq!(*pos_2.col_index, ch(3));
        }

        // AddAssign, SubAssign Pos.
        {
            let mut pos_1 = VPPos::new((vp_row(1), vp_col(2)));
            pos_1 += VPPos::new((vp_row(3), vp_col(4)));
            assert_eq!(*pos_1.row_index, ch(4));
            assert_eq!(*pos_1.col_index, ch(6));

            let mut pos_2 = VPPos::new((vp_row(5), vp_col(7)));
            pos_2 -= VPPos::new((vp_row(2), vp_col(3)));
            assert_eq!(*pos_2.row_index, ch(3));
            assert_eq!(*pos_2.col_index, ch(4));
        }

        // Add, Sub ColWidthCount.
        {
            let pos = VPPos::new((vp_col(ch(5)), vp_row(ch(7))));

            let pos_1 = pos + VPWidth::new(ch(2));
            assert_eq!(*pos_1.col_index, ch(7));
            assert_eq!(*pos_1.row_index, ch(7));

            let pos_2 = pos - VPWidth::new(ch(2));
            assert_eq!(*pos_2.col_index, ch(3));
            assert_eq!(*pos_2.row_index, ch(7));
        }

        // AddAssign, SubAssign ColWidthCount.
        {
            let mut pos_1 = VPPos::new((vp_row(5), vp_col(7)));
            pos_1 += VPWidth::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(5));

            let mut pos_2 = VPPos::new((vp_row(5), vp_col(7)));
            pos_2 -= VPWidth::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(5));
        }

        // Add, Sub RowWidthCount.
        {
            let pos = VPPos::new((vp_row(ch(5)), vp_col(ch(7))));
            let pos_1 = pos + VPHeight::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(7));

            let pos_2 = pos - VPHeight::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(3));
        }

        // AddAssign, SubAssign RowWidthCount.
        {
            let mut pos_1 = VPPos::new((vp_row(ch(5)), vp_col(ch(7))));
            pos_1 += VPHeight::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(7));

            let mut pos_2 = VPPos::new((vp_row(ch(5)), vp_col(ch(7))));
            pos_2 -= VPHeight::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(3));
        }
    }

    #[test]
    fn test_pos_new() {
        // Order matters.
        let pos = VPPos::new((vp_row(1), vp_col(2)));
        assert_eq!(pos.row_index, ch(1).into());
        assert_eq!(pos.col_index, ch(2).into());
        assert_eq!(*pos.row_index, ch(1));
        assert_eq!(*pos.col_index, ch(2));

        let pos_2 = VPPos {
            row_index: ch(1).into(),
            col_index: ch(2).into(),
        };
        assert_eq!(pos, pos_2);
    }

    #[test]
    fn test_pos_from() {
        // Order does not matter.
        let pos_1: VPPos = (VPRow::new(1), VPCol::new(2)).into();
        let pos_2: VPPos = (VPCol::new(2), VPRow::new(1)).into();

        assert_eq!(pos_1, pos_2);
    }

    #[test]
    fn test_pos_add() {
        // Order matters!
        let pos1 = VPPos::new((vp_row(1), vp_col(2)));
        let pos2 = VPPos::new((vp_row(3), vp_col(4)));
        let result = pos1 + pos2;
        assert_eq!(result, VPPos::new((vp_row(4), vp_col(6))));
    }

    #[test]
    fn test_pos_sub() {
        let pos1 = VPPos::new((vp_row(5), vp_col(7)));
        let pos2 = VPPos::new((vp_row(2), vp_col(3)));
        let result = pos1 - pos2;
        assert_eq!(result, VPPos::new((vp_row(3), vp_col(4))));
    }

    #[test]
    fn test_add_box_size_to_pos() {
        let pos = vp_row(1) + vp_col(2);
        let dim = vp_width(2) + vp_height(2);
        let result = pos + dim;
        assert_eq!(result, vp_row(3) + vp_col(4));
    }

    #[test]
    fn test_mul_box_pos_to_pair() {
        // [30, 10] * [1, 0] = [30, 0]
        {
            let pos = vp_col(30) + vp_row(10);
            let pair_cancel_row = (vp_width(1), vp_height(0));
            let new_pos = pos * pair_cancel_row;
            assert_eq!(new_pos, vp_col(30) + vp_row(0));

            let dim_cancel_row = vp_width(1) + vp_height(0);
            let new_pos = pos * dim_cancel_row;
            assert_eq!(new_pos, vp_col(30) + vp_row(0));
        }

        // [30, 10] * [0, 1] = [0, 10]
        {
            let pos = vp_col(30) + vp_row(10);
            let pair_cancel_col = (vp_width(0), vp_height(1));
            let new_pos = pos * pair_cancel_col;
            assert_eq!(new_pos, vp_col(0) + vp_row(10));

            let dim_cancel_col = vp_width(0) + vp_height(1);
            let new_pos = pos * dim_cancel_col;
            assert_eq!(new_pos, vp_col(0) + vp_row(10));
        }
    }

    #[test]
    fn test_ch_unit_add_and_add_assign() {
        let mut pos0 = vp_row(1) + vp_col(2);
        pos0 += ch(3);
        assert_eq!(pos0, vp_row(4) + vp_col(5));

        let pos1 = pos0 + ch(12);
        assert_eq!(pos1, vp_row(16) + vp_col(17));
    }

    #[test]
    fn test_convert_pos_to_row_or_col() {
        let pos = vp_row(1) + vp_col(2);
        let r: VPRow = pos.into();
        let c: VPCol = pos.into();
        assert_eq!(c, vp_col(2));
        assert_eq!(r, vp_row(1));
    }

    #[test]
    fn test_bounds_clamping_semantic() {
        // This test explicitly documents that bounds methods clamp to the "after last"
        // position (index == length), not the last valid index (index == length -
        // 1). This is essential for cursor positioning in text editors.

        // Test 1: clip_col_to_bounds with overflow
        {
            let mut pos = vp_row(0) + vp_col(10); // Start at col 10
            let max_width = vp_width(5); // Maximum width is 5

            pos.clip_col_to_bounds(max_width);

            // CRITICAL: We expect vp_col(5), NOT vp_col(4)!
            // vp_col(5) is the "after last" position for vp_width(5)
            assert_eq!(
                *pos.col_index,
                ch(5),
                "clip_col_to_bounds should clamp to position equal to width (after last), not width-1"
            );
        }

        // Test 2: add_col_with_bounds with overflow
        {
            let mut pos = vp_row(0) + vp_col(2); // Start at col 2
            let max_width = vp_width(4); // Maximum width is 4

            // Adding 5 to vp_col(2) = vp_col(7), which exceeds vp_width(4)
            pos.add_col_with_bounds(vp_width(5), max_width);

            assert_eq!(
                *pos.col_index,
                ch(4),
                "add_col_with_bounds should clamp to position equal to width when overflow occurs"
            );
        }

        // Test 3: add_row_with_bounds with overflow
        {
            let mut pos = vp_row(1) + vp_col(0); // Start at row 1
            let max_height = vp_height(3); // Maximum height is 3

            // Adding 5 to vp_row(1) = vp_row(6), which exceeds vp_height(3)
            pos.add_row_with_bounds(vp_height(5), max_height);

            assert_eq!(
                *pos.row_index,
                ch(3),
                "add_row_with_bounds should clamp to position equal to height when overflow occurs"
            );
        }

        // Test 4: Verify exact boundary behavior
        {
            let mut pos = vp_row(0) + vp_col(3);
            let max_width = vp_width(3);

            // vp_col(3) == vp_width(3), so this is exactly at the "after last" position
            pos.clip_col_to_bounds(max_width);
            assert_eq!(
                *pos.col_index,
                ch(3),
                "Position exactly at width should remain unchanged"
            );

            // But vp_col(4) > vp_width(3), so it should clamp to 3
            pos.col_index = vp_col(4);
            pos.clip_col_to_bounds(max_width);
            assert_eq!(
                *pos.col_index,
                ch(3),
                "Position beyond width should clamp to width value"
            );
        }

        // Test 5: Edge case with zero width
        {
            let mut pos = vp_row(0) + vp_col(5);
            let zero_width = vp_width(0);

            pos.clip_col_to_bounds(zero_width);
            assert_eq!(
                *pos.col_index,
                ch(0),
                "Zero width should clamp any position to 0"
            );
        }

        // Test 6: No clamping when within bounds
        {
            let mut pos = vp_row(0) + vp_col(2);
            let max_width = vp_width(5);

            pos.clip_col_to_bounds(max_width);
            assert_eq!(
                *pos.col_index,
                ch(2),
                "Position within bounds should remain unchanged"
            );
        }
    }
}
