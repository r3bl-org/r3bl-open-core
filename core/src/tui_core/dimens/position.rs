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
          ops::{Add, AddAssign, Mul}};

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
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default, GetSize)]
pub struct Position {
  pub col: ChUnit,
  pub row: ChUnit,
}

impl Position {
  /// Reset given `col` count to `0`.
  pub fn reset_cols(&mut self) -> Self {
    self.col = ch!(0);
    *self
  }

  /// Set given `col` count to `value`.
  pub fn set_cols(&mut self, value: ChUnit) -> Self {
    self.col = value;
    *self
  }

  /// Add given `col` count to `self`.
  pub fn add_cols(&mut self, num_cols_to_add: usize) -> Self {
    let value: ChUnit = ch!(num_cols_to_add);
    self.col += value;
    *self
  }

  /// Add given `col` count to `self` w/ bounds check for max cols.
  pub fn add_cols_with_bounds(&mut self, value: ChUnit, max: ChUnit) -> Self {
    if (self.col + value) >= max {
      self.col = max;
    } else {
      self.col += value;
    }
    *self
  }

  /// Set `col` count to `max` if `self.col` is greater than `max`.
  pub fn clip_cols_to_bounds(&mut self, max: ChUnit) -> Self {
    if self.col >= max {
      self.col = max;
    }
    *self
  }

  /// Add given `row` count to `self`.
  pub fn add_rows(&mut self, num_rows_to_add: usize) -> Self {
    let value: ChUnit = ch!(num_rows_to_add);
    self.row += value;
    *self
  }

  /// Add given `row` count to `self` w/ bounds check for max rows.
  pub fn add_rows_with_bounds(&mut self, value: ChUnit, max: ChUnit) -> Self {
    if (self.row + value) >= max {
      self.row = max;
    } else {
      self.row += value;
    }
    *self
  }

  pub fn sub_rows(&mut self, num_rows_to_sub: usize) -> Self {
    let value: ChUnit = ch!(num_rows_to_sub);
    self.row -= value;
    *self
  }

  pub fn sub_cols(&mut self, num_cols_to_sub: usize) -> Self {
    let value: ChUnit = ch!(num_cols_to_sub);
    self.col -= value;
    *self
  }
}

pub mod math_ops {
  use super::*;

  impl AddAssign<ChUnit> for Position {
    fn add_assign(&mut self, other: ChUnit) {
      self.col += other;
      self.row += other;
    }
  }

  impl Add<Position> for Position {
    type Output = Position;
    fn add(self, other: Position) -> Self::Output {
      Position {
        col: self.col + other.col,
        row: self.row + other.row,
      }
    }
  }

  /// Add: BoxPosition + BoxSize = BoxPosition.
  /// <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
  impl Add<Size> for Position {
    type Output = Position;
    fn add(self, other: Size) -> Self {
      Self {
        col: self.col + other.col,
        row: self.row + other.row,
      }
    }
  }

  /// Mul: BoxPosition * Pair = BoxPosition.
  /// <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
  impl Mul<(u16, u16)> for Position {
    type Output = Position;
    fn mul(self, other: (u16, u16)) -> Self {
      Self {
        col: self.col * ch!(other.0),
        row: self.row * ch!(other.1),
      }
    }
  }
}

pub mod convert_position_to_other_type {
  use super::*;

  impl From<Position> for (ChUnit, ChUnit) {
    fn from(position: Position) -> Self { (position.col, position.row) }
  }
}

pub mod debug_formatter {
  use super::*;

  impl Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      write!(f, "[col:{}, row:{}]", *self.col, *self.row)
    }
  }
}

#[macro_export]
macro_rules! position {
  (
    col: $arg_col:expr,
    row: $arg_row:expr
  ) => {
    Position {
      col: $arg_col.into(),
      row: $arg_row.into(),
    }
  };
}
