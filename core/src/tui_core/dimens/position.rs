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
/// Position, defined as [col, row].
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Copy, Default)]
pub struct Position {
  pub col: UnitType,
  pub row: UnitType,
}

impl AddAssign<UnitType> for Position {
  fn add_assign(&mut self, other: UnitType) {
    self.col += other;
    self.row += other;
  }
}

impl From<Pair> for Position {
  fn from(pair: Pair) -> Self {
    Self {
      col: pair.first,
      row: pair.second,
    }
  }
}

impl From<(UnitType, UnitType)> for Position {
  fn from(pair: (UnitType, UnitType)) -> Self {
    Self {
      col: pair.0,
      row: pair.1,
    }
  }
}

impl From<Position> for (UnitType, UnitType) {
  fn from(position: Position) -> Self { (position.col, position.row) }
}

impl Position {
  /// Add given `col` value to `self`.
  pub fn add_col(&mut self, value: usize) -> Self {
    let value: UnitType = value as UnitType;
    self.col += value;
    *self
  }

  /// Add given `row` value to `self`.
  pub fn add_row(&mut self, value: usize) -> Self {
    let value = value as UnitType;
    self.row += value;
    *self
  }

  /// Add given `row` value to `self` w/ bounds check for max rows.
  pub fn add_row_with_bounds(&mut self, value: usize, box_bounding_size: Size) -> Self {
    let value: UnitType = value as UnitType;
    let max: UnitType = box_bounding_size.rows;

    if (self.row + value) >= max {
      self.row = max
    } else {
      self.row += value;
    }

    *self
  }
}

impl Debug for Position {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[col:{}, row:{}]", self.col, self.row)
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
      col: self.col + other.cols,
      row: self.row + other.rows,
    }
  }
}

/// Mul: BoxPosition * Pair = BoxPosition.
/// <https://doc.rust-lang.org/book/ch19-03-advanced-traits.html>
impl Mul<Pair> for Position {
  type Output = Position;
  fn mul(self, other: Pair) -> Self {
    Self {
      col: self.col * other.first,
      row: self.row * other.second,
    }
  }
}
