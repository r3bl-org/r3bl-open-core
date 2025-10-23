// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! A caret represents the insertion point or cursor in a text buffer. It can be one of
//! two kinds:
//! - [`CaretRaw`]: A struct that represents the "raw" position is the `col_index` and
//!   `row_index` of the caret INSIDE the viewport, without making any adjustments for
//!   scrolling.
//! - [`CaretScrAdj`]: A struct that represents the "scroll adjusted" position is the
//!   `col_index` and `row_index` of the caret OUTSIDE the viewport, after making
//!   adjustments for scrolling.
//!
//! # The many ways to create one
//!
//! - This API uses the `impl Into<struct>` pattern and [Add] `+` operator overloading to
//!   allow for easy conversion between [`CaretRaw`] and [`CaretScrAdj`].
//! - You can use the [`caret_raw()`], [`caret_scr_adj()`] functions to create a
//!   [`CaretRaw`], [`CaretScrAdj`] struct respectively. These functions can take a
//!   sequence of [Add]ed [Pos] and [`ScrOfs`] as input, or tuples of them in any order.
//! - Just using using the [Add] `+` operator:
//!     - You can use [Add] to compose: [`ScrOfs`] + [`CaretRaw`], into: a
//!       [`CaretScrAdj`].
//!     - You can use [Add] to compose: [`CaretScrAdj`] + [`ScrOfs`], into: a
//!       [`CaretRaw`].
//!
//! # Examples
//!
//! ```
//! use r3bl_tui::{
//!     ch,
//!     Pos, ScrOfs, CaretRaw, CaretScrAdj,
//!     col, row, caret_raw, scr_ofs, pos, caret_scr_adj
//! };
//!
//! let scroll_offset_1: ScrOfs = scr_ofs(row(2) + col(3));
//!
//! //
//! // Directly using CaretRaw and CaretScrAdj.
//! //
//!
//! // The order of the arguments doesn't matter below.
//! let raw_caret_1: CaretRaw = caret_raw(col(5) + row(5));
//! let scr_adj_caret_1: CaretScrAdj = caret_scr_adj(col(7) + row(8));
//!
//! assert_eq!(pos(row(5) + col(5)), *raw_caret_1);
//! assert_eq!(pos(row(8) + col(7)), *scr_adj_caret_1);
//!
//! //
//! // Using Caret (and not directly specifying CaretRaw or CaretScrAdj).
//! //
//!
//! // Convert CaretScrAdj (and ScrollOffset) to CaretRaw.
//! let caret_1 = scr_adj_caret_1 + scroll_offset_1;
//! let caret_2 = scroll_offset_1 + scr_adj_caret_1;
//! let expected_1 = pos(row(8) + col(7)) - *scr_ofs(row(2) + col(3));
//! assert_eq!(expected_1, *caret_1);
//! assert_eq!(expected_1, *caret_2);
//!
//! // Convert CaretRaw (and ScrollOffset) to CaretScrAdj.
//! let caret_3 = raw_caret_1 + scroll_offset_1;
//! let caret_4 = scroll_offset_1 + raw_caret_1;
//! let expected_2 = pos(row(5) + col(5)) + *scr_ofs(row(2) + col(3));
//! assert_eq!(expected_2, *caret_3);
//! assert_eq!(expected_2, *caret_4);
//! ```

use crate::{Pos, ScrOfs};
use std::ops::{Add, Deref, DerefMut};

pub fn caret_raw(arg_caret_raw: impl Into<CaretRaw>) -> CaretRaw { arg_caret_raw.into() }

pub fn caret_scr_adj(arg_caret_scr_adj: impl Into<CaretScrAdj>) -> CaretScrAdj {
    arg_caret_scr_adj.into()
}

/// The "raw" position is the `col_index` and `row_index` of the caret INSIDE the
/// viewport, without making any adjustments for scrolling.
/// - It does not take into account the amount of scrolling (vertical, horizontal) that is
///   currently active.
/// - When scrolling is "active", this position will be different from the "scroll
///   adjusted" position.
/// - This is the default `CaretKind`.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct CaretRaw(pub Pos);

mod caret_raw_impl {
    use super::{Add, CaretRaw, CaretScrAdj, Deref, DerefMut, Pos, ScrOfs};

    impl CaretRaw {
        pub fn new(arg_caret_raw: impl Into<CaretRaw>) -> Self { arg_caret_raw.into() }
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
            let position = *caret_scr_adj - *scr_ofs;
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
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct CaretScrAdj(pub Pos);

mod caret_scr_adj_impl {
    use super::{Add, CaretRaw, CaretScrAdj, Deref, DerefMut, Pos, ScrOfs};

    impl CaretScrAdj {
        pub fn new(arg_caret_scr_adj: impl Into<CaretScrAdj>) -> Self {
            arg_caret_scr_adj.into()
        }
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
            let position = *caret_raw + *scr_ofs;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ch, col, row, scr_ofs};

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

        // Into CaretRaw, from.
        {
            let scr_ofs = scr_ofs(pos_2);
            let scr_adj_caret = caret_scr_adj(pos_1);

            let raw_caret_1 = scr_ofs + scr_adj_caret;
            assert_eq!(*raw_caret_1, *scr_adj_caret - *scr_ofs);

            let raw_caret_2 = scr_adj_caret + scr_ofs;
            assert_eq!(*raw_caret_2, *scr_adj_caret - *scr_ofs);
        }

        // Into CaretScrAdj, from.
        {
            let raw_caret = caret_raw(pos_1);
            let scr_ofs = scr_ofs(pos_1);

            let scr_adj_caret_1 = raw_caret + scr_ofs;
            assert_eq!(*scr_adj_caret_1, *raw_caret + *scr_ofs);

            let scr_adj_caret_2 = scr_ofs + raw_caret;
            assert_eq!(*scr_adj_caret_2, *raw_caret + *scr_ofs);
        }
    }

    #[test]
    fn test_raw_to_scroll_adjusted() {
        let position = Pos {
            col_index: ch(5).into(),
            row_index: ch(5).into(),
        };

        let scr_ofs = scr_ofs(Pos {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        });

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
        let caret: CaretScrAdj = raw_caret + scr_ofs;
        assert_eq!(*caret, *scr_adj_caret);
    }

    #[test]
    fn test_scroll_adjusted_to_raw() {
        let scr_adj_caret: CaretScrAdj = Pos {
            col_index: ch(7).into(),
            row_index: ch(8).into(),
        }
        .into();

        let scr_ofs = scr_ofs(Pos {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        });

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

        let scr_ofs = scr_ofs(Pos {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        });

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

        let scr_ofs = scr_ofs(Pos {
            col_index: ch(2).into(),
            row_index: ch(3).into(),
        });

        let raw_caret: CaretRaw = (scr_adj_caret, scr_ofs).into();

        assert_eq!(
            *raw_caret,
            Pos {
                col_index: ch(5).into(),
                row_index: ch(5).into(),
            }
        );
    }
}
