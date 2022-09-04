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
/// let size: Size = size!(10, 10);
/// ```
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
  pub cols: UnitType, // width = number of cols (y).
  pub rows: UnitType, // height = number of rows (x).
}

impl Display for Size {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Size: [{}, {}]", self.rows, self.cols)
  }
}

impl SubAssign<UnitType> for Size {
  fn sub_assign(&mut self, other: UnitType) {
    if other > self.cols {
      self.cols = 0;
    } else {
      self.cols -= other;
    }

    if other > self.rows {
      self.rows = 0;
    } else {
      self.rows -= other;
    }
  }
}

impl From<Pair> for Size {
  fn from(pair: Pair) -> Self {
    Self {
      cols: pair.first,
      rows: pair.second,
    }
  }
}

impl From<(UnitType, UnitType)> for Size {
  /// 1. First (pair.0) is width or cols.
  /// 2. Second (pair.1) is height or rows.
  fn from(pair: (UnitType, UnitType)) -> Self {
    Self {
      cols: pair.0,
      rows: pair.1,
    }
  }
}

impl From<(usize, usize)> for Size {
  fn from(pair: (usize, usize)) -> Self {
    Self {
      cols: convert_to_base_unit!(pair.0),
      rows: convert_to_base_unit!(pair.1),
    }
  }
}

impl From<(i32, i32)> for Size {
  fn from(pair: (i32, i32)) -> Self {
    Self {
      cols: convert_to_base_unit!(pair.0),
      rows: convert_to_base_unit!(pair.1),
    }
  }
}

impl Debug for Size {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[width:{}, height:{}]", self.cols, self.rows)
  }
}

/// Example:
/// ```ignore
/// let size: Size = size!(col: 10, row: 10);
/// ```
#[macro_export]
macro_rules! size {
  (
    col: $arg_col:expr,
    row: $arg_row:expr
  ) => {
    Size {
      cols: $arg_col,
      rows: $arg_row,
    }
  };
}
