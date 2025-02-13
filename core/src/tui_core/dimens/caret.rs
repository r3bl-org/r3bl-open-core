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

//! [Caret] is an enum that represents the position of the caret in the text buffer. It
//! can be in one of two states:
//! - [CaretRaw]: A struct that represents the "raw" position is the `col_index` and
//!   `row_index` of the caret INSIDE the viewport, without making any adjustments for
//!   scrolling.
//! - [CaretScrAdj]: A struct that represents the "scroll adjusted" position is the
//!   `col_index` and `row_index` of the caret OUTSIDE the viewport, after making
//!   adjustments for scrolling.
//!
//! The [Caret] enum is a wrapper around the [CaretRaw] and [CaretScrAdj] structs. It can
//! be converted to and from both types. You can either indirectly use both of these
//! structs using [Caret] or you can just use them directly using [CaretRaw] or
//! [CaretScrAdj].
//!
//! # The many ways to create one
//!
//! - This API uses the `impl Into<struct>` pattern and [Add] `+` operator overloading to
//!   allow for easy conversion between [CaretRaw] and [CaretScrAdj].
//! - You can use the [caret()], [caret_raw()], [caret_scr_adj()] functions to create a
//!   [Caret], [CaretRaw], [CaretScrAdj] struct respectively. These functions can take a
//!   sequence of [Add]ed [Pos] and [ScrOfs] as input, or tuples of them in any order.
//! - Just using using the [Add] `+` operator:
//!     - You can use [Add] to convert: [ScrOfs] + [CaretRaw], into: a [CaretScrAdj].
//!     - You can use [Add] to convert: [CaretScrAdj] + [ScrOfs], into: a [CaretRaw].
//!
//! # Examples
//!
//! ```rust
//! use r3bl_core::{
//!     ch,
//!     Pos, ScrOfs, CaretRaw, CaretScrAdj, Caret,
//!     col, row, caret_raw, scr_ofs, pos, caret_scr_adj, caret
//! };
//!
//! let scroll_offset_1: ScrOfs = row(2) + col(3);
//!
//! //
//! // Directly using CaretRaw and CaretScrAdj.
//! //
//!
//! // Note the order of the arguments don't matter below.
//! let raw_caret_1: CaretRaw = caret_raw(col(5) + row(5));
//! let scr_adj_caret_1: CaretScrAdj = caret_scr_adj(col(7) + row(8));
//!
//! assert!(matches!(Caret::new(raw_caret_1), Caret::Raw(_)));
//! assert_eq!(pos(row(5) + col(5)), *raw_caret_1);
//! assert!(matches!(Caret::new(scr_adj_caret_1), Caret::ScrollAdjusted(_)));
//! assert_eq!(pos(row(8) + col(7)), *scr_adj_caret_1);
//!
//! //
//! // Using Caret (and not directly specifying CaretRaw or CaretScrAdj).
//! //
//!
//! // Convert CaretScrAdj (and ScrollOffset) to CaretRaw.
//! let caret_1: Caret = caret(scr_adj_caret_1 + scroll_offset_1);
//! let caret_2: Caret = caret(scroll_offset_1 + scr_adj_caret_1);
//! assert!(matches!(caret_1, Caret::Raw(_)));
//! assert!(matches!(caret_2, Caret::Raw(_)));
//! let expected_1 = pos(row(8) + col(7)) - scr_ofs(row(2) + col(3));
//! assert_eq!(expected_1, *caret_1);
//! assert_eq!(expected_1, *caret_2);
//!
//! // Convert CaretRaw (and ScrollOffset) to CaretScrAdj.
//! let caret_3: Caret = caret(raw_caret_1 + scroll_offset_1);
//! let caret_4: Caret = caret(scroll_offset_1 + raw_caret_1);
//! assert!(matches!(caret_3, Caret::ScrollAdjusted(_)));
//! assert!(matches!(caret_4, Caret::ScrollAdjusted(_)));
//! let expected_2 = pos(row(5) + col(5)) + scr_ofs(row(2) + col(3));
//! assert_eq!(expected_2, *caret_3);
//! assert_eq!(expected_2, *caret_4);
//! ```

use std::ops::{Add, Deref, DerefMut};

use super::{ColIndex, ColWidth, RowHeight, RowIndex, col, row};
use crate::{Pos, ScrOfs};

pub fn caret(arg: impl Into<Caret>) -> Caret { arg.into() }

pub fn caret_raw(arg: impl Into<CaretRaw>) -> CaretRaw { arg.into() }

pub fn caret_scr_adj(arg: impl Into<CaretScrAdj>) -> CaretScrAdj { arg.into() }

/// The "raw" position is the `col_index` and `row_index` of the caret INSIDE the
/// viewport, without making any adjustments for scrolling.
/// - It does not take into account the amount of scrolling (vertical, horizontal) that is
///   currently active.
/// - When scrolling is "active", this position will be different from the "scroll
///   adjusted" position.
/// - This is the default `CaretKind`.
#[derive(Copy, Clone, PartialEq, Debug, Default, size_of::SizeOf)]
pub struct CaretRaw(pub Pos);

mod caret_raw_impl {
    use super::*;

    impl CaretRaw {
        pub fn new(arg: impl Into<CaretRaw>) -> Self { arg.into() }
    }

    impl Deref for CaretRaw {
        type Target = Pos;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for CaretRaw {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<(CaretScrAdj, ScrOfs)> for CaretRaw {
        fn from((caret_scr_adj, scr_ofs): (CaretScrAdj, ScrOfs)) -> Self {
            let position = *caret_scr_adj - scr_ofs;
            CaretRaw(position)
        }
    }

    impl From<(ScrOfs, CaretScrAdj)> for CaretRaw {
        fn from((scr_ofs, caret_scr_adj): (ScrOfs, CaretScrAdj)) -> Self {
            (caret_scr_adj, scr_ofs).into()
        }
    }

    impl From<Pos> for CaretRaw {
        fn from(position: Pos) -> Self { CaretRaw(position) }
    }

    // CaretScrAdj + ScrOfs = CaretRaw
    impl Add<CaretScrAdj> for ScrOfs {
        type Output = CaretRaw;

        fn add(self, rhs: CaretScrAdj) -> Self::Output { (rhs, self).into() }
    }

    // ScrOfs + CaretScrAdj = CaretRaw
    impl Add<ScrOfs> for CaretScrAdj {
        type Output = CaretRaw;

        fn add(self, rhs: ScrOfs) -> Self::Output { (self, rhs).into() }
    }
}

/// The "scroll adjusted" position is the `col_index` and `row_index` of the caret OUTSIDE
/// the viewport, after making adjustments for scrolling.
/// - It takes into account the amount of scrolling (vertical, horizontal) that is
///   currently active.
/// - When scrolling is "active", this position will be different from the "raw" position.
#[derive(Copy, Clone, PartialEq, Debug, Default, size_of::SizeOf)]
pub struct CaretScrAdj(pub Pos);

mod caret_scr_adj_impl {
    use super::*;

    impl CaretScrAdj {
        pub fn new(arg: impl Into<CaretScrAdj>) -> Self { arg.into() }
    }

    impl Deref for CaretScrAdj {
        type Target = Pos;

        fn deref(&self) -> &Self::Target { &self.0 }
    }

    impl DerefMut for CaretScrAdj {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
    }

    impl From<(CaretRaw, ScrOfs)> for CaretScrAdj {
        fn from((caret_raw, scr_ofs): (CaretRaw, ScrOfs)) -> Self {
            let position = *caret_raw + scr_ofs;
            CaretScrAdj(position)
        }
    }

    impl From<(ScrOfs, CaretRaw)> for CaretScrAdj {
        fn from((scr_ofs, caret_raw): (ScrOfs, CaretRaw)) -> Self {
            (caret_raw, scr_ofs).into()
        }
    }

    impl From<Pos> for CaretScrAdj {
        fn from(position: Pos) -> Self { CaretScrAdj(position) }
    }

    // CaretRaw + ScrOfs = CaretScrAdj
    impl Add<CaretRaw> for ScrOfs {
        type Output = CaretScrAdj;

        fn add(self, rhs: CaretRaw) -> Self::Output { (rhs, self).into() }
    }

    // ScrOfs + CaretRaw = CaretScrAdj
    impl Add<ScrOfs> for CaretRaw {
        type Output = CaretScrAdj;

        fn add(self, rhs: ScrOfs) -> Self::Output { (self, rhs).into() }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, size_of::SizeOf)]
pub enum Caret {
    Raw(CaretRaw),
    ScrollAdjusted(CaretScrAdj),
}

mod caret_impl {
    use super::*;

    impl Caret {
        pub fn new(arg: impl Into<Caret>) -> Self { arg.into() }
    }

    impl Deref for Caret {
        type Target = Pos;

        fn deref(&self) -> &Self::Target {
            match self {
                Caret::Raw(caret_raw) => caret_raw,
                Caret::ScrollAdjusted(caret_scr_adj) => caret_scr_adj,
            }
        }
    }

    impl Default for Caret {
        fn default() -> Self { Caret::Raw(CaretRaw::default()) }
    }

    impl From<CaretScrAdj> for Caret {
        fn from(caret_scr_adj: CaretScrAdj) -> Self {
            Caret::ScrollAdjusted(caret_scr_adj)
        }
    }

    impl From<CaretRaw> for Caret {
        fn from(caret_raw: CaretRaw) -> Self { Caret::Raw(caret_raw) }
    }

    impl From<(CaretRaw, ScrOfs)> for Caret {
        fn from((caret_raw, scr_ofs): (CaretRaw, ScrOfs)) -> Self {
            let scr_adj_caret: CaretScrAdj = (caret_raw, scr_ofs).into();
            Caret::ScrollAdjusted(scr_adj_caret)
        }
    }

    impl From<(ScrOfs, CaretRaw)> for Caret {
        fn from((scr_ofs, caret_raw): (ScrOfs, CaretRaw)) -> Self {
            (caret_raw, scr_ofs).into()
        }
    }

    impl From<(CaretScrAdj, ScrOfs)> for Caret {
        fn from((caret_scr_adj, scr_ofs): (CaretScrAdj, ScrOfs)) -> Self {
            let caret_raw: CaretRaw = (caret_scr_adj, scr_ofs).into();
            Caret::Raw(caret_raw)
        }
    }

    impl From<(ScrOfs, CaretScrAdj)> for Caret {
        fn from((scr_ofs, scr_adj_caret): (ScrOfs, CaretScrAdj)) -> Self {
            (scr_adj_caret, scr_ofs).into()
        }
    }

    impl From<Pos> for Caret {
        fn from(position: Pos) -> Self {
            let caret_raw: CaretRaw = position.into();
            Caret::Raw(caret_raw)
        }
    }
}

impl Caret {
    /// The caret max index which is the scroll index goes 1 past the end of the given
    /// width's index. Which just happens to be the same number as the given width.
    ///
    /// Equivalent to:
    /// ```text
    /// col_amt_index = col_amt - 1;
    /// scroll_past_col_amt_index = col_amt_index + 1;
    /// ```
    ///
    /// Here's an example:
    /// ```text
    /// R ┌──────────┐
    /// 0 ▸hello░    │
    ///   └─────▴────┘
    ///   C0123456789
    /// ```
    pub fn scroll_col_index_for_width(col_amt: ColWidth) -> ColIndex {
        col_amt.convert_to_col_index() /* -1 */ + col(1) /* +1 */
    }

    /// The caret max index which is the scroll index goes 1 past the end of the given
    /// height's index. Which just happens to be the same number as the given height.
    pub fn scroll_row_index_for_height(row_amt: RowHeight) -> RowIndex {
        row_amt.convert_to_row_index() /* -1 */ +  row(1) /* +1 */
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ch, col, pos, row, scr_ofs, width};

    #[test]
    fn test_caret_usage() {
        // This is how caret information is stored in the EditorBuffer::EditorContent
        // struct. This is the "source of truth" for this data.
        let editor_content_caret_raw = caret_raw(col(5) + row(5));
        let editor_content_scr_ofs = col(1) + row(1);

        // Some code requires the caret as raw. It is then Deref'd and used as a Pos.
        {
            let caret = caret(editor_content_caret_raw);
            assert_eq!(*caret, pos(row(5) + col(5)));
            assert!(matches!(caret, Caret::Raw(_)));
        }

        // Some code requires the caret as scroll adjusted. It is then Deref'd and used as
        // a Pos.
        {
            let scr_adj_caret = editor_content_caret_raw + editor_content_scr_ofs;
            let caret = caret(scr_adj_caret);
            assert_eq!(*caret, pos(row(6) + col(6)));
            assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        }
    }

    #[test]
    fn test_constructor_fns() {
        let pos_1 = row(5) + col(5);
        let pos_2 = col(2) + row(3);

        // raw_caret constructor fn.
        {
            let rc = caret_raw(pos_1);
            assert!(matches!(rc, CaretRaw { .. }));
            assert_eq!(*rc, pos_1);
        }

        // scr_adj_caret constructor fn.
        {
            let sac = caret_scr_adj(pos_1);
            assert!(matches!(sac, CaretScrAdj { .. }));
            assert_eq!(*sac, pos_1);
        }

        // Into CaretRaw, from ...
        {
            let scr_ofs = scr_ofs(pos_2);
            let scr_adj_caret = caret_scr_adj(pos_1);

            let raw_caret_1 = caret(scr_ofs + scr_adj_caret);
            assert!(matches!(raw_caret_1, Caret::Raw(_)));
            assert_eq!(*raw_caret_1, *scr_adj_caret - scr_ofs);

            let raw_caret_2 = caret(scr_adj_caret + scr_ofs);
            assert!(matches!(raw_caret_2, Caret::Raw(_)));
            assert_eq!(*raw_caret_2, *scr_adj_caret - scr_ofs);
        }

        // Into CaretScrAdj, from ...
        {
            let raw_caret = caret_raw(pos_1);
            let scr_ofs = scr_ofs(pos_1);

            let scr_adj_caret_1 = caret(raw_caret + scr_ofs);
            assert!(matches!(scr_adj_caret_1, Caret::ScrollAdjusted(_)));
            assert_eq!(*scr_adj_caret_1, *raw_caret + scr_ofs);

            let scr_adj_caret_2 = caret(scr_ofs + raw_caret);
            assert!(matches!(scr_adj_caret_2, Caret::ScrollAdjusted(_)));
            assert_eq!(*scr_adj_caret_2, *raw_caret + scr_ofs);
        }
    }

    #[test]
    fn test_default_caret_kind() {
        let default_caret = Caret::default();

        assert!(matches!(default_caret, Caret::Raw(_)));
        assert_eq!(default_caret, Caret::Raw(CaretRaw::default()));
        assert_eq!(*default_caret, Pos::default());

        let caret: Caret = Pos::default().into();

        assert!(matches!(caret, Caret::Raw(_)));
        assert_eq!(caret, Caret::Raw(CaretRaw::default()));
    }

    #[test]
    fn test_caret_new() {
        let raw_caret = CaretRaw::new(Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        });

        let scr_adj_caret = CaretScrAdj::new(Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        });

        {
            let caret = Caret::new(scr_adj_caret);
            assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        }

        {
            let caret = Caret::new(raw_caret);
            assert!(matches!(caret, Caret::Raw(_)));
        }

        {
            let caret = Caret::new(Pos {
                col_index: ch(7).into(),
                row_index: ch(8).into(),
            });
            assert!(matches!(caret, Caret::Raw(_)));
        }

        {
            let caret = Caret::new((
                raw_caret,
                ScrOfs {
                    col_index: ch(2).into(),
                    row_index: ch(3).into(),
                },
            ));
            assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        }

        {
            let caret = Caret::new((
                scr_adj_caret,
                ScrOfs {
                    col_index: ch(2).into(),
                    row_index: ch(3).into(),
                },
            ));
            assert!(matches!(caret, Caret::Raw(_)));
        }
    }

    #[test]
    fn test_caret_from() {
        let raw_caret = CaretRaw::new(Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        });

        let scr_adj_caret = CaretScrAdj::new(Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        });

        {
            let caret: Caret = scr_adj_caret.into();
            assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        }

        {
            let caret: Caret = raw_caret.into();
            assert!(matches!(caret, Caret::Raw(_)));
        }

        {
            let caret: Caret = Pos {
                col_index: ch(7).into(),
                row_index: ch(8).into(),
            }
            .into();
            assert!(matches!(caret, Caret::Raw(_)));
        }

        {
            let caret: Caret = (
                raw_caret,
                ScrOfs {
                    col_index: ch(2).into(),
                    row_index: ch(3).into(),
                },
            )
                .into();
            assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        }

        {
            let caret: Caret = (
                scr_adj_caret,
                ScrOfs {
                    col_index: ch(2).into(),
                    row_index: ch(3).into(),
                },
            )
                .into();
            assert!(matches!(caret, Caret::Raw(_)));
        }
    }

    #[test]
    fn test_raw_to_scroll_adjusted() {
        let position = Pos {
            col_index: ch(5).into(),
            row_index: ch(5).into(),
        };

        let scr_ofs = ScrOfs {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        };

        // Create CaretRaw from Position.
        let raw_caret: CaretRaw = position.into();

        assert_eq!(raw_caret.0, position);
        assert_eq!(*raw_caret, position);

        // Convert CaretRaw (and ScrollOffset) to CaretScrAdj.
        let scr_adj_caret: CaretScrAdj = (raw_caret, scr_ofs).into();

        assert_eq!(
            scr_adj_caret.0,
            Pos {
                col_index: ch(7).into(),
                row_index: ch(8).into()
            }
        );
        assert_eq!(
            *scr_adj_caret,
            Pos {
                col_index: ch(7).into(),
                row_index: ch(8).into()
            }
        );

        // Convert CaretRaw (and ScrollOffset) to Caret.
        let caret: Caret = (raw_caret, scr_ofs).into();

        assert!(matches!(caret, Caret::ScrollAdjusted(_)));
        assert!(!matches!(caret, Caret::Raw(_)));
        assert_eq!(*caret, *scr_adj_caret);
    }

    #[test]
    fn test_scroll_adjusted_to_raw() {
        let scr_adj_caret: CaretScrAdj = Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        }
        .into();

        let scr_ofs = ScrOfs {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        };

        let raw_caret: CaretRaw = (scr_adj_caret, scr_ofs).into();

        assert_eq!(
            *raw_caret,
            Pos {
                col_index: ch(5).into(),
                row_index: ch(5).into(),
            }
        );

        let back_to_scroll_adjusted_caret: CaretScrAdj = (raw_caret, scr_ofs).into();

        assert_eq!(*back_to_scroll_adjusted_caret, *scr_adj_caret);
    }

    #[test]
    fn test_caret_conversion_to_scroll_adjusted() {
        let raw_caret: CaretRaw = Pos {
            col_index: ch(5).into(),
            row_index: ch(5).into(),
        }
        .into();

        let caret: Caret = raw_caret.into();

        let Caret::Raw(raw_caret) = caret else {
            panic!("Expected CaretRaw");
        };

        let scr_ofs = ScrOfs {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        };

        let scr_adj_caret: CaretScrAdj = (raw_caret, scr_ofs).into();

        assert_eq!(
            *scr_adj_caret,
            Pos {
                col_index: ch(7).into(),
                row_index: ch(8).into(),
            }
        );
    }

    #[test]
    fn test_caret_conversion_to_raw() {
        let scr_adj_caret: CaretScrAdj = Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        }
        .into();

        let caret: Caret = scr_adj_caret.into();

        let Caret::ScrollAdjusted(scr_adj_caret) = caret else {
            panic!("Expected CaretScrAdj");
        };

        let scr_ofs = ScrOfs {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        };

        let raw_caret: CaretRaw = (scr_adj_caret, scr_ofs).into();

        assert_eq!(
            *raw_caret,
            Pos {
                col_index: ch(5).into(),
                row_index: ch(5).into(),
            }
        );
    }

    #[test]
    fn test_scroll_col_index_for_width() {
        let width = width(5);
        let scroll_col_index = Caret::scroll_col_index_for_width(width);
        assert_eq!(*scroll_col_index, *width);
    }
}
