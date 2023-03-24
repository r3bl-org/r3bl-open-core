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

use std::{fmt::{self, Debug, Display},
          ops::SubAssign};

use get_size::GetSize;
use serde::*;

use crate::*;

/// Here is a visual representation of how position and sizing works for the
/// layout engine.
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
/// Size, defined as [height, width]. Here are some examples.
/// ```ignore
/// let max_size: Size = (/* max_col: */ 10, /* max_row: */ 10).into();
/// ```
///
/// ```ignore
/// let size: Size = size!(cols: 10, rows: 10);
/// ```
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, GetSize, Hash)]
pub struct Size {
    pub col_count: ChUnit, // width = number of cols (y).
    pub row_count: ChUnit, // height = number of rows (x).
}

impl Size {
    pub fn deser_from_str(ser_str: &str) -> Option<Size> {
        if let Ok(size) = serde_json::from_str(ser_str) {
            Some(size)
        } else {
            None
        }
    }

    pub fn ser_to_string(&self) -> Option<String> {
        let ser_str = serde_json::to_string(self);
        if let Ok(ser_str) = ser_str {
            Some(ser_str)
        } else {
            None
        }
    }
}

impl Size {
    pub fn is_too_small_to_display(&self, min_col: u8, min_row: u8) -> bool {
        self.col_count < ch!(min_col) || self.row_count < ch!(min_row)
    }
}

pub mod size_debug_formatter {
    use super::*;

    impl Display for Size {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Size: [{}, {}]", *self.row_count, *self.col_count)
        }
    }

    impl Debug for Size {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "[width:{}, height:{}]", *self.col_count, *self.row_count)
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

/// Example:
/// ```ignore
/// let size: Size = size!(col: 10, row: 10);
/// ```
#[macro_export]
macro_rules! size {
    (
        col_count: $arg_col:expr,
        row_count: $arg_row:expr
    ) => {
        Size {
            col_count: $arg_col.into(),
            row_count: $arg_row.into(),
        }
    };
}
