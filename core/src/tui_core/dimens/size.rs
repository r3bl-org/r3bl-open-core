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

use crate::*;
use std::{
  fmt::{self, Debug, Display},
  ops::SubAssign,
};

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
/// Size, defined as [height, width].
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Size {
  pub width: UnitType,  // number of cols (y).
  pub height: UnitType, // number of rows (x).
}

impl Display for Size {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Size: [{}, {}]", self.height, self.width)
  }
}

impl SubAssign<UnitType> for Size {
  fn sub_assign(&mut self, other: UnitType) {
    self.width -= other;
    self.height -= other;
  }
}

impl From<Pair> for Size {
  fn from(pair: Pair) -> Self {
    Self {
      width: pair.first,
      height: pair.second,
    }
  }
}

impl From<(UnitType, UnitType)> for Size {
  /// 1. First (pair.0) is width or cols.
  /// 2. Second (pair.1) is height or rows.
  fn from(pair: (UnitType, UnitType)) -> Self {
    Self {
      width: pair.0,
      height: pair.1,
    }
  }
}

impl From<(usize, usize)> for Size {
  fn from(pair: (usize, usize)) -> Self {
    Self {
      width: convert_to_base_unit!(pair.0),
      height: convert_to_base_unit!(pair.1),
    }
  }
}

impl From<(i32, i32)> for Size {
  fn from(pair: (i32, i32)) -> Self {
    Self {
      width: convert_to_base_unit!(pair.0),
      height: convert_to_base_unit!(pair.1),
    }
  }
}

impl Size {
  /// Return an `Option` with `self`.
  pub fn as_some(&self) -> Option<Self> {
    Some(*self)
  }
}

impl Debug for Size {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[width:{}, height:{}]", self.width, self.height)
  }
}
