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
          ops::{Add, AddAssign, Mul}};

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
/// Position, defined as [col, row]. Here are some examples.
///
/// ```ignore
/// let pos: Position = (/* col: */ 0, /* row: */ 0).into();
/// pos.add_col(1);
/// pos.add_row(1);
/// pos += (/* col: */ 1, /* row: */ 1);
/// let max_size: Size = (/* _max_col: */ 10, /* max_row: */ 10).into();
/// pos.add_row_with_bounds(20, max_size);
/// ```
///
/// ```ignore
/// let pos: Position = position!(0, 0);
/// ```
#[derive(
    Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default, Hash, size_of::SizeOf,
)]
pub struct Position {
    pub col_index: ChUnit,
    pub row_index: ChUnit,
}

impl Position {
    pub fn deser_from_str(ser_str: &str) -> Option<Position> {
        if let Ok(position) = serde_json::from_str(ser_str) {
            Some(position)
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

impl Position {
    /// Reset given `col` count to `0`.
    pub fn reset_col(&mut self) -> Self {
        self.col_index = ch!(0);
        *self
    }

    /// Set given `col` count to `value`.
    pub fn set_col(&mut self, value: ChUnit) -> Self {
        self.col_index = value;
        *self
    }

    /// Add given `col` count to `self`.
    pub fn add_col(&mut self, num_cols_to_add: usize) -> Self {
        let value: ChUnit = ch!(num_cols_to_add);
        self.col_index += value;
        *self
    }

    /// Add given `col` count to `self` w/ bounds check for max cols.
    pub fn add_col_with_bounds(&mut self, value: ChUnit, max: ChUnit) -> Self {
        if (self.col_index + value) >= max {
            self.col_index = max;
        } else {
            self.col_index += value;
        }
        *self
    }

    /// Set `col` count to `max` if `self.col` is greater than `max`.
    pub fn clip_col_to_bounds(&mut self, max: ChUnit) -> Self {
        if self.col_index >= max {
            self.col_index = max;
        }
        *self
    }

    /// Reset given `row` count to `0`.
    pub fn reset_row(&mut self) -> Self {
        self.row_index = ch!(0);
        *self
    }

    /// Set given `row` count to `value`.
    pub fn set_row(&mut self, value: ChUnit) -> Self {
        self.row_index = value;
        *self
    }

    /// Add given `row` count to `self`.
    pub fn add_row(&mut self, num_rows_to_add: usize) -> Self {
        let value: ChUnit = ch!(num_rows_to_add);
        self.row_index += value;
        *self
    }

    /// Add given `row` count to `self` w/ bounds check for max rows.
    pub fn add_row_with_bounds(&mut self, value: ChUnit, max: ChUnit) -> Self {
        if (self.row_index + value) >= max {
            self.row_index = max;
        } else {
            self.row_index += value;
        }
        *self
    }

    pub fn sub_row(&mut self, num_rows_to_sub: usize) -> Self {
        let value: ChUnit = ch!(num_rows_to_sub);
        self.row_index -= value;
        *self
    }

    pub fn sub_col(&mut self, num_cols_to_sub: usize) -> Self {
        let value: ChUnit = ch!(num_cols_to_sub);
        self.col_index -= value;
        *self
    }
}

pub mod position_math_ops {
    use super::*;

    impl AddAssign<ChUnit> for Position {
        fn add_assign(&mut self, other: ChUnit) {
            self.col_index += other;
            self.row_index += other;
        }
    }

    impl AddAssign<Position> for Position {
        fn add_assign(&mut self, other: Position) {
            self.col_index += other.col_index;
            self.row_index += other.row_index;
        }
    }

    impl Add<Position> for Position {
        type Output = Position;
        fn add(self, other: Position) -> Self::Output {
            Position {
                col_index: self.col_index + other.col_index,
                row_index: self.row_index + other.row_index,
            }
        }
    }

    /// Add: BoxPosition + BoxSize = BoxPosition.
    /// <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
    impl Add<Size> for Position {
        type Output = Position;
        fn add(self, other: Size) -> Self {
            Self {
                col_index: self.col_index + other.col_count,
                row_index: self.row_index + other.row_count,
            }
        }
    }

    /// Mul: BoxPosition * Pair = BoxPosition.
    /// <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
    impl Mul<(u16, u16)> for Position {
        type Output = Position;
        fn mul(self, other: (u16, u16)) -> Self {
            Self {
                col_index: self.col_index * ch!(other.0),
                row_index: self.row_index * ch!(other.1),
            }
        }
    }
}

pub mod convert_position_to_other_type {
    use super::*;

    impl From<Position> for (ChUnit, ChUnit) {
        fn from(position: Position) -> Self { (position.col_index, position.row_index) }
    }
}

pub mod position_debug_formatter {
    use super::*;

    fn fmt_position(position: &Position, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[col:{}, row:{}]",
            *position.col_index, *position.row_index
        )
    }

    impl Debug for Position {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_position(self, f) }
    }

    impl Display for Position {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_position(self, f) }
    }
}

#[macro_export]
macro_rules! position {
    (
        col_index: $arg_col:expr,
        row_index: $arg_row:expr
    ) => {
        Position {
            col_index: $arg_col.into(),
            row_index: $arg_row.into(),
        }
    };
}
