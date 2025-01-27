/*
 *   Copyright (c) 2022 R3BL LLC
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

use std::{fmt::{self, Debug},
          ops::SubAssign};

use super::ChUnit;
use crate::{ch, sub_unsigned};

/// Size is defined as: (col_count, row_count).
///
/// Here is a visual representation of how position and sizing works for the layout
/// engine.
///
/// ```text
///     0   4    9    1    2    2
///                   4    0    5
///    ┌────┴────┴────┴────┴────┴──→ col
///  0 ┤     ╭─────────────╮
///  1 ┤     │ origin pos: │
///  2 ┤     │ [5, 0]      │
///  3 ┤     │ size:       │
///  4 ┤     │ [16, 5]     │
///  5 ┤     ╰─────────────╯
///    ↓
///    row
/// ```
///
/// # Examples
///
/// ```rust
/// use r3bl_core::{size, Size, ch};
/// let max_size: Size = size!(col_count: 10, row_count: 10);
/// ```
///
/// ```rust
/// use r3bl_core::{size, Size, ch};
/// let size: Size = size!(col_count: 10, row_count: 10);
/// ```
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash, size_of::SizeOf)]
pub struct Size {
    pub col_count: ChUnit, // width = number of cols (y).
    pub row_count: ChUnit, // height = number of rows (x).
}

impl Size {
    pub fn fits_min_size(&self, min_col: u8, min_row: u8) -> TooSmallToDisplayResult {
        match self.col_count < ch(min_col) || self.row_count < ch(min_row) {
            false => TooSmallToDisplayResult::IsLargeEnough,
            true => TooSmallToDisplayResult::IsTooSmall,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TooSmallToDisplayResult {
    IsLargeEnough,
    IsTooSmall,
}

pub mod size_debug_formatter {
    use super::*;

    impl Debug for Size {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Size: [w: {w}, h: {h}]",
                w = *self.row_count,
                h = *self.col_count
            )
        }
    }
}

pub mod size_math_ops {
    use super::*;

    impl SubAssign<ChUnit> for Size {
        fn sub_assign(&mut self, other: ChUnit) {
            self.col_count = sub_unsigned!(*self.col_count, *other).into();
            self.row_count = sub_unsigned!(*self.row_count, *other).into();
        }
    }
}

/// # Example
///
/// ```
/// use r3bl_core::{size, Size, ch};
/// let size: Size = size!(col_count: 10, row_count: 10);
/// ```
#[macro_export]
macro_rules! size {
    (
        col_count: $arg_col:expr,
        row_count: $arg_row:expr
    ) => {
        $crate::Size {
            col_count: $arg_col.into(),
            row_count: $arg_row.into(),
        }
    };
}
