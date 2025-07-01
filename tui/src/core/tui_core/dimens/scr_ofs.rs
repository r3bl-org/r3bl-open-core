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
          ops::{Deref, DerefMut}};

use crate::{ColIndex, Pos, RowIndex};

/// `ScrOfs` is just a "newtype" built on top of (wrapping) [`crate::Pos`]. You can use
/// this exactly like a [`crate::Pos`], when you deref it using `*` prefix, but it's more
/// semantically meaningful to use this when you're dealing with the offset of a position
/// on the screen.
///
/// ```
/// use r3bl_tui::{ScrOfs, scr_ofs, row, col, Pos};
/// let pos = row(1) + col(2);
/// let so = scr_ofs(pos);
/// let pos: Pos = *so;
/// ```
#[derive(Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct ScrOfs(pub Pos);

pub fn scr_ofs(arg_scr_ofs: impl Into<ScrOfs>) -> ScrOfs { arg_scr_ofs.into() }

mod basic {
    use super::*;

    impl ScrOfs {
        pub fn new(arg_pos: impl Into<ScrOfs>) -> Self { arg_pos.into() }
    }

    impl From<Pos> for ScrOfs {
        fn from(pos: Pos) -> Self { ScrOfs(pos) }
    }

    impl From<ScrOfs> for RowIndex {
        fn from(pos: ScrOfs) -> Self { pos.row_index }
    }

    impl From<ScrOfs> for ColIndex {
        fn from(pos: ScrOfs) -> Self { pos.col_index }
    }

    impl Deref for ScrOfs {
        type Target = Pos;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for ScrOfs {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }
}

mod debug {
    use super::*;

    impl Debug for ScrOfs {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            write!(
                f,
                "ScrOfs [c: {a:?}, r: {b:?}]",
                a = *self.col_index,
                b = *self.row_index
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::{ch, col, height, row, width, ColWidth};

    #[test]
    fn test_api() {
        // Constructor.
        {
            let so_1 = scr_ofs(row(1) + col(2));
            assert_eq!(*so_1.row_index, ch(1));
            assert_eq!(*so_1.col_index, ch(2));

            let pos_1 = row(10) + col(20);
            let so_2: ScrOfs = pos_1.into();
            assert_eq!(*so_2.row_index, ch(10));
            assert_eq!(*so_2.col_index, ch(20));
        }

        // Methods.
        {
            let row_idx = RowIndex::new(ch(1));
            let col_idx = ColIndex::new(ch(2));
            let wid = ColWidth::new(ch(3));

            let mut pos: ScrOfs = (col_idx + row_idx).into();
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
            let pos = ScrOfs::new(col(2) + row(1));
            let mut acc = String::new();
            let _ = write!(acc, "{pos:?}");
            assert_eq!(acc, "ScrOfs [c: 2, r: 1]");
        }
    }
}
