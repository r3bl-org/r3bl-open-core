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

use std::{fmt::{Debug, Formatter, Result},
          ops::{Add, AddAssign, Mul, Sub, SubAssign}};

use crate::{ColIndex, ColWidth, RowHeight, RowIndex, Size, ch};

// Type aliases for better code readability.

pub type Row = RowIndex;
pub type Col = ColIndex;

/// `Pos` is a struct that holds the `row` and `col` indices of a character in a text
/// buffer. [`RowIndex`] and [`ColIndex`] are the types of the `row` and `col` indices
/// respectively. This ensures that it isn't possible to use a `col` when you intended to
/// use a `row` and vice versa.
///
/// Also [`crate::ScrOfs`] is a "newtype" built around `Pos`, since a scroll offset is
/// just a position after all, but semantically it is used for different reasons in the
/// API. It is used to declare a different intention on how `Pos` is used.
///
/// Here is a visual representation of how position and sizing work for the layout
/// engine.
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
/// allow for easy conversion between [`crate::ChUnit`] and [`RowIndex`]/[`ColIndex`].
/// - You can use [`crate::pos()`] function and pass it a [`RowIndex`] and [`ColIndex`]
///   tuple, or pass a sequence of them with the [Add] `+` operator.
/// - Just using the [Add] `+` operator:
///     - You can use [Add] to convert: [`RowIndex`] + [`ColIndex`], into: a `Pos`.
///     - You can use [Add] to convert: [`ColIndex`] + [`RowIndex`], into: a `Pos`.
///
/// # Examples
///
/// ```
/// use r3bl_tui::{
///     ch,
///     ScrOfs, Pos, RowIndex, ColIndex,
///     row, col, pos, scr_ofs
/// };
///
/// // So many different ways to create a Pos.
/// let pos_1: Pos = pos(row(2) + col(3));
/// let pos_1: Pos = (row(2) + col(3)).into();
/// let pos_1: Pos = (row(2), col(3)).into();
/// let pos_1: Pos = (col(3), row(2)).into();
///
/// // Create a ScrOfs from a Pos.
/// let scr_ofs_1: ScrOfs = (row(2) + col(3)).into();
/// let scr_ofs_1: ScrOfs = pos_1.into();
///
/// assert!(matches!(pos_1.row_index, RowIndex(_)));
/// assert!(matches!(pos_1.col_index, ColIndex(_)));
/// assert_eq!(*pos_1.row_index, ch(2));
/// assert_eq!(*pos_1.col_index, ch(3));
///
/// let pos_a = pos(row(4) + col(10));
/// let pos_b = pos(row(2) + col(6));
///
/// let pos_sum = pos_a + pos_b;
/// assert_eq!(*pos_sum.row_index, ch(6));
/// assert_eq!(*pos_sum.col_index, ch(16));
///
/// let pos_diff = pos_a - pos_b;
/// assert_eq!(*pos_diff.row_index, ch(2));
/// assert_eq!(*pos_diff.col_index, ch(4));
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct Pos {
    /// Row index, 0-based.
    pub row_index: RowIndex,
    /// Column index, 0-based.
    pub col_index: ColIndex,
}

pub fn pos(arg_pos: impl Into<Pos>) -> Pos { arg_pos.into() }

mod constructor {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Pos {
        pub fn new(arg_pos: impl Into<Pos>) -> Self { arg_pos.into() }
    }

    impl From<(RowIndex, ColIndex)> for Pos {
        fn from((row, col): (RowIndex, ColIndex)) -> Self {
            Pos {
                row_index: row,
                col_index: col,
            }
        }
    }

    impl From<(ColIndex, RowIndex)> for Pos {
        fn from((col, row): (ColIndex, RowIndex)) -> Self {
            Pos {
                row_index: row,
                col_index: col,
            }
        }
    }

    impl Add<ColIndex> for RowIndex {
        type Output = Pos;

        fn add(self, rhs: ColIndex) -> Self::Output {
            Pos {
                row_index: self,
                col_index: rhs,
            }
        }
    }

    impl Add<RowIndex> for ColIndex {
        type Output = Pos;

        fn add(self, rhs: RowIndex) -> Self::Output {
            Pos {
                row_index: rhs,
                col_index: self,
            }
        }
    }
}

mod convert {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<Pos> for RowIndex {
        fn from(pos: Pos) -> Self { pos.row_index }
    }

    impl From<Pos> for ColIndex {
        fn from(pos: Pos) -> Self { pos.col_index }
    }
}

mod ops {
    #[allow(clippy::wildcard_imports)]
    use super::*;
    use crate::{ChUnit, col, row};

    // Dim is equivalent to (ColWidthCount, RowHeightCount).
    impl Mul<Size> for Pos {
        type Output = Pos;

        fn mul(self, rhs: Size) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.row_height;
            self_copy.col_index = self.col_index * rhs.col_width;
            self_copy
        }
    }

    // (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount) is equivalent to
    // Dim.
    impl Mul<(ColWidth, RowHeight)> for Pos {
        type Output = Pos;

        fn mul(self, rhs: (ColWidth, RowHeight)) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.1;
            self_copy.col_index = self.col_index * rhs.0;
            self_copy
        }
    }

    // (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount) is equivalent to
    // Dim.
    impl Mul<(RowHeight, ColWidth)> for Pos {
        type Output = Pos;

        fn mul(self, rhs: (RowHeight, ColWidth)) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index * rhs.0;
            self_copy.col_index = self.col_index * rhs.1;
            self_copy
        }
    }

    impl Add<Size> for Pos {
        type Output = Pos;

        fn add(self, rhs: Size) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index + rhs.row_height;
            self_copy.col_index = self.col_index + rhs.col_width;
            self_copy
        }
    }

    impl Sub<Size> for Pos {
        type Output = Pos;

        fn sub(self, rhs: Size) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = {
                let it = self.row_index - rhs.row_height;
                row(*it)
            };
            self_copy.col_index = {
                let it = self.col_index - rhs.col_width;
                col(*it)
            };
            self_copy
        }
    }

    impl AddAssign<Size> for Pos {
        fn add_assign(&mut self, rhs: Size) { *self = *self + rhs; }
    }

    impl SubAssign<Size> for Pos {
        fn sub_assign(&mut self, rhs: Size) { *self = *self - rhs; }
    }

    impl Add<Pos> for Pos {
        type Output = Pos;

        fn add(self, rhs: Pos) -> Self::Output {
            let mut self_copy = self;
            *self_copy.row_index += *rhs.row_index;
            *self_copy.col_index += *rhs.col_index;
            self_copy
        }
    }

    impl Sub<Pos> for Pos {
        type Output = Pos;

        fn sub(self, rhs: Pos) -> Self::Output {
            let mut self_copy = self;
            *self_copy.row_index -= *rhs.row_index;
            *self_copy.col_index -= *rhs.col_index;
            self_copy
        }
    }

    impl AddAssign<Pos> for Pos {
        fn add_assign(&mut self, rhs: Pos) { *self = *self + rhs; }
    }

    impl SubAssign<Pos> for Pos {
        fn sub_assign(&mut self, rhs: Pos) { *self = *self - rhs; }
    }

    impl Add<ColWidth> for Pos {
        type Output = Pos;

        fn add(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index = self.col_index + rhs;
            self_copy
        }
    }

    impl AddAssign<ColWidth> for Pos {
        fn add_assign(&mut self, rhs: ColWidth) { *self = *self + rhs; }
    }

    impl Sub<ColWidth> for Pos {
        type Output = Pos;

        fn sub(self, rhs: ColWidth) -> Self::Output {
            let mut self_copy = self;
            self_copy.col_index -= rhs;
            self_copy
        }
    }

    impl SubAssign<ColWidth> for Pos {
        fn sub_assign(&mut self, rhs: ColWidth) { *self = *self - rhs; }
    }

    impl Add<RowHeight> for Pos {
        type Output = Pos;

        fn add(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index = self.row_index + rhs;
            self_copy
        }
    }

    impl Sub<RowHeight> for Pos {
        type Output = Pos;

        fn sub(self, rhs: RowHeight) -> Self::Output {
            let mut self_copy = self;
            self_copy.row_index -= rhs;
            self_copy
        }
    }

    impl AddAssign<RowHeight> for Pos {
        fn add_assign(&mut self, rhs: RowHeight) { *self = *self + rhs; }
    }

    impl SubAssign<RowHeight> for Pos {
        fn sub_assign(&mut self, rhs: RowHeight) { *self = *self - rhs; }
    }

    impl AddAssign<ChUnit> for Pos {
        fn add_assign(&mut self, rhs: ChUnit) {
            *self.row_index += rhs;
            *self.col_index += rhs;
        }
    }

    impl Add<ChUnit> for Pos {
        type Output = Pos;

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
    impl Pos {
        /// Reset col and row index to `0`.
        pub fn reset(&mut self) {
            *self.col_index = ch(0);
            *self.row_index = ch(0);
        }

        /// Reset row index to `0`.
        pub fn reset_row(&mut self) { *self.row_index = ch(0); }

        /// Reset col index to `0`.
        pub fn reset_col(&mut self) { *self.col_index = ch(0); }
    }

    // Row index API.
    impl Pos {
        /// Set row index to `value`.
        pub fn set_row(&mut self, arg_row_index: impl Into<RowIndex>) {
            self.row_index = arg_row_index.into();
        }

        /// Increment row index by `value`.
        pub fn add_row(&mut self, arg_row_index: impl Into<RowHeight>) {
            let value: RowHeight = arg_row_index.into();
            *self.row_index += *value;
        }

        /// Increment row index by `value`, while making sure it will never exceed
        /// `max_row`.
        #[allow(clippy::return_self_not_must_use)]
        pub fn add_row_with_bounds(
            &mut self,
            arg_row_height: impl Into<RowHeight>,
            arg_max_row_height: impl Into<RowHeight>,
        ) {
            let value: RowHeight = arg_row_height.into();
            let max: RowHeight = arg_max_row_height.into();
            *self.row_index = std::cmp::min(*self.row_index + *value, *max);
        }

        /// Decrement row index by `value`.
        pub fn sub_row(&mut self, arg_row_height: impl Into<RowHeight>) {
            let value: RowHeight = arg_row_height.into();
            *self.row_index -= *value;
        }
    }

    // Col index API.
    impl Pos {
        /// Set col index to `value`.
        pub fn set_col(&mut self, arg_col_index: impl Into<ColIndex>) {
            let value: ColIndex = arg_col_index.into();
            self.col_index = value;
        }

        /// Increment col index by `value`. Returns a copy of `Pos`.
        #[allow(clippy::return_self_not_must_use)]
        pub fn add_col(&mut self, arg_col_width: impl Into<ColWidth>) -> Self {
            let width: ColWidth = arg_col_width.into();
            *self.col_index += *width;
            *self
        }

        /// Increment col index by `col_amt`, while making sure it will never exceed
        /// `max_col_amt`. This function is not concerned with scrolling or
        /// [`crate::ScrOfs`].
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
        /// 0 ▸hello░   │
        ///   └─────▴───┘
        ///   C0123456789
        /// ```
        ///
        /// Keep in mind these dynamics change when introducing scrolling, but this struct
        /// does not take scrolling into account. See
        /// [r3bl_tui::tui::editor_engine::scroll_editor_buffer](https://github.com/r3bl-org/r3bl-open-core/blob/main/tui/src/tui/editor/editor_engine/editor_engine_internal_api.rs)
        /// for that.
        pub fn add_col_with_bounds(
            &mut self,
            arg_col_width: impl Into<ColWidth>,
            arg_max_col_width: impl Into<ColWidth>,
        ) {
            let value: ColWidth = arg_col_width.into();
            let max: ColWidth = arg_max_col_width.into();
            *self.col_index = std::cmp::min(*self.col_index + *value, *max);
        }

        /// Clip col index to `max_col` if it exceeds it.
        pub fn clip_col_to_bounds(&mut self, arg_max_col_width: impl Into<ColWidth>) {
            let max: ColWidth = arg_max_col_width.into();
            *self.col_index = std::cmp::min(*self.col_index, *max);
        }

        /// Decrement col index by `value`.
        pub fn sub_col(&mut self, arg_col_width: impl Into<ColWidth>) {
            let value: ColWidth = arg_col_width.into();
            *self.col_index -= *value;
        }
    }
}

mod debug {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for Pos {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
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
    use std::fmt::Write;

    use super::*;
    use crate::{col, height, row, width};

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_api() {
        // Constructor.
        {
            let pos_0 = row(1) + col(2);
            assert_eq!(*pos_0.row_index, ch(1));
            assert_eq!(*pos_0.col_index, ch(2));

            let pos_1 = pos(row(1) + col(2));
            assert_eq!(*pos_1.row_index, ch(1));
            assert_eq!(*pos_1.col_index, ch(2));

            let pos_2 = pos(col(2) + row(1));
            assert_eq!(*pos_2.row_index, ch(1));
            assert_eq!(*pos_2.col_index, ch(2));
        }

        // Methods.
        {
            let row_idx = RowIndex::new(ch(1));
            let col_idx = ColIndex::new(ch(2));
            let wid = ColWidth::new(ch(3));

            let mut pos: Pos = (col_idx, row_idx).into();
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
                let col_idx = wid - width(1);
                width(*col_idx)
            });
            assert_eq!(*pos.col_index, ch(2));

            pos.sub_col(width(1));
            assert_eq!(*pos.col_index, ch(1));

            pos.sub_col(width(10));
            assert_eq!(*pos.col_index, ch(0));

            pos.reset_row();
            assert_eq!(*pos.row_index, ch(0));

            pos.set_row(row_idx);
            assert_eq!(*pos.row_index, ch(1));

            pos.add_row(height(ch(3)));
            assert_eq!(*pos.row_index, ch(4));

            pos.add_row_with_bounds(height(ch(10)), height(ch(5)));
            assert_eq!(*pos.row_index, ch(5));

            pos.sub_row(height(ch(2)));
            assert_eq!(*pos.row_index, ch(3));

            pos.sub_row(height(ch(10)));
            assert_eq!(*pos.row_index, ch(0));
        }

        // Debug Pos.
        {
            let pos = Pos::new((ColIndex::new(ch(2)), RowIndex::new(ch(1))));
            let mut acc = String::new();
            // We don't care about the result of this operation.
            write!(acc, "{pos:?}").ok();
            assert_eq!(acc, "Pos [c: 2, r: 1]");
        }

        // Mul (ColWidthCount, RowHeightCount) or (RowHeightCount, ColWidthCount).
        {
            let pos = Pos::new((row(1), col(2)));

            let pos_1 = pos * (height(ch(2)), width(ch(2)));
            assert_eq!(*pos_1.row_index, ch(2));
            assert_eq!(*pos_1.col_index, ch(4));

            let pos_2 = pos * (width(ch(2)), height(ch(2)));
            assert_eq!(*pos_2.row_index, ch(2));
            assert_eq!(*pos_2.col_index, ch(4));
        }

        // Add, Sub Dim.
        {
            let pos = Pos::new((row(1), col(2)));
            let dim: Size = (width(ch(2)), height(ch(2))).into();

            let pos_1 = pos + dim;
            assert_eq!(*pos_1.row_index, ch(3));
            assert_eq!(*pos_1.col_index, ch(4));

            let pos_2 = pos_1 - dim;
            assert_eq!(*pos_2.row_index, ch(1));
            assert_eq!(*pos_2.col_index, ch(2));
        }

        // AddAssign, SubAssign Dim.
        {
            let mut pos = Pos::new((RowIndex::new(ch(1)), ColIndex::new(ch(2))));
            pos += Size::new((width(ch(2)), height(ch(2))));
            assert_eq!(*pos.row_index, ch(3));
            assert_eq!(*pos.col_index, ch(4));

            pos -= Size::new((width(ch(2)), height(ch(2))));
            assert_eq!(*pos.row_index, ch(1));
            assert_eq!(*pos.col_index, ch(2));
        }

        // Add, Sub Pos.
        {
            let pos = Pos::new((row(2), col(2)));
            let pos_1 = pos - Pos::new((row(1), col(1)));
            assert_eq!(*pos_1.row_index, ch(1));
            assert_eq!(*pos_1.col_index, ch(1));

            let pos_2 = pos + Pos::new((row(1), col(1)));
            assert_eq!(*pos_2.row_index, ch(3));
            assert_eq!(*pos_2.col_index, ch(3));
        }

        // AddAssign, SubAssign Pos.
        {
            let mut pos_1 = Pos::new((row(1), col(2)));
            pos_1 += Pos::new((row(3), col(4)));
            assert_eq!(*pos_1.row_index, ch(4));
            assert_eq!(*pos_1.col_index, ch(6));

            let mut pos_2 = Pos::new((row(5), col(7)));
            pos_2 -= Pos::new((row(2), col(3)));
            assert_eq!(*pos_2.row_index, ch(3));
            assert_eq!(*pos_2.col_index, ch(4));
        }

        // Add, Sub ColWidthCount.
        {
            let pos = Pos::new((col(ch(5)), row(ch(7))));

            let pos_1 = pos + ColWidth::new(ch(2));
            assert_eq!(*pos_1.col_index, ch(7));
            assert_eq!(*pos_1.row_index, ch(7));

            let pos_2 = pos - ColWidth::new(ch(2));
            assert_eq!(*pos_2.col_index, ch(3));
            assert_eq!(*pos_2.row_index, ch(7));
        }

        // AddAssign, SubAssign ColWidthCount.
        {
            let mut pos_1 = Pos::new((row(5), col(7)));
            pos_1 += ColWidth::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(5));

            let mut pos_2 = Pos::new((row(5), col(7)));
            pos_2 -= ColWidth::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(5));
        }

        // Add, Sub RowWidthCount.
        {
            let pos = Pos::new((row(ch(5)), col(ch(7))));
            let pos_1 = pos + RowHeight::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(7));

            let pos_2 = pos - RowHeight::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(3));
        }

        // AddAssign, SubAssign RowWidthCount.
        {
            let mut pos_1 = Pos::new((row(ch(5)), col(ch(7))));
            pos_1 += RowHeight::new(ch(2));
            assert_eq!(*pos_1.row_index, ch(7));

            let mut pos_2 = Pos::new((row(ch(5)), col(ch(7))));
            pos_2 -= RowHeight::new(ch(2));
            assert_eq!(*pos_2.row_index, ch(3));
        }
    }

    #[test]
    fn test_pos_new() {
        // Order matters.
        let pos = Pos::new((row(1), col(2)));
        assert_eq!(pos.row_index, ch(1).into());
        assert_eq!(pos.col_index, ch(2).into());
        assert_eq!(*pos.row_index, ch(1));
        assert_eq!(*pos.col_index, ch(2));

        let pos_2 = Pos {
            row_index: ch(1).into(),
            col_index: ch(2).into(),
        };
        assert_eq!(pos, pos_2);
    }

    #[test]
    fn test_pos_from() {
        // Order does not matter.
        let pos_1: Pos = (RowIndex::new(1), ColIndex::new(2)).into();
        let pos_2: Pos = (ColIndex::new(2), RowIndex::new(1)).into();

        assert_eq!(pos_1, pos_2);
    }

    #[test]
    fn test_pos_add() {
        // Order matters!
        let pos1 = Pos::new((row(1), col(2)));
        let pos2 = Pos::new((row(3), col(4)));
        let result = pos1 + pos2;
        assert_eq!(result, Pos::new((row(4), col(6))));
    }

    #[test]
    fn test_pos_sub() {
        let pos1 = Pos::new((row(5), col(7)));
        let pos2 = Pos::new((row(2), col(3)));
        let result = pos1 - pos2;
        assert_eq!(result, Pos::new((row(3), col(4))));
    }

    #[test]
    fn test_add_box_size_to_pos() {
        let pos = row(1) + col(2);
        let dim = width(2) + height(2);
        let result = pos + dim;
        assert_eq!(result, row(3) + col(4));
    }

    #[test]
    fn test_mul_box_pos_to_pair() {
        // [30, 10] * [1, 0] = [30, 0]
        {
            let pos = col(30) + row(10);
            let pair_cancel_row = (width(1), height(0));
            let new_pos = pos * pair_cancel_row;
            assert_eq!(new_pos, col(30) + row(0));

            let dim_cancel_row = width(1) + height(0);
            let new_pos = pos * dim_cancel_row;
            assert_eq!(new_pos, col(30) + row(0));
        }

        // [30, 10] * [0, 1] = [0, 10]
        {
            let pos = col(30) + row(10);
            let pair_cancel_col = (width(0), height(1));
            let new_pos = pos * pair_cancel_col;
            assert_eq!(new_pos, col(0) + row(10));

            let dim_cancel_col = width(0) + height(1);
            let new_pos = pos * dim_cancel_col;
            assert_eq!(new_pos, col(0) + row(10));
        }
    }

    #[test]
    fn test_ch_unit_add_and_add_assign() {
        let mut pos0 = row(1) + col(2);
        pos0 += ch(3);
        assert_eq!(pos0, row(4) + col(5));

        let pos1 = pos0 + ch(12);
        assert_eq!(pos1, row(16) + col(17));
    }

    #[test]
    fn test_convert_pos_to_row_or_col() {
        let pos = row(1) + col(2);
        let r: RowIndex = pos.into();
        let c: ColIndex = pos.into();
        assert_eq!(c, col(2));
        assert_eq!(r, row(1));
    }
}
