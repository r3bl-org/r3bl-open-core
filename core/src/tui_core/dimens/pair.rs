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

use std::fmt::{self, Debug};

use crate::*;

/// Pair, defined as [left, right].
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Pair {
  pub first: UnitType,
  pub second: UnitType,
}

impl Debug for Pair {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "Pair [first:{}, second:{}]", self.first, self.second) }
}

impl From<(UnitType, UnitType)> for Pair {
  fn from(pair: (UnitType, UnitType)) -> Self {
    Self {
      first: pair.0,
      second: pair.1,
    }
  }
}

impl From<(u8, u8)> for Pair {
  fn from(pair: (u8, u8)) -> Self {
    Self {
      first: pair.0.into(),
      second: pair.1.into(),
    }
  }
}

impl From<(i32, i32)> for Pair {
  fn from(pair: (i32, i32)) -> Self {
    Self {
      first: convert_to_base_unit!(pair.0),
      second: convert_to_base_unit!(pair.1),
    }
  }
}

/// https://stackoverflow.com/a/28280042/2085356
impl From<(usize, usize)> for Pair {
  fn from(pair: (usize, usize)) -> Self {
    Self {
      first: convert_to_base_unit!(pair.0),
      second: convert_to_base_unit!(pair.1),
    }
  }
}
