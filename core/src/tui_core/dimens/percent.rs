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
          ops::Deref};

use serde::*;

use crate::*;

/// Represents an integer value between 0 and 100 (inclusive).
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
pub struct Percent {
  pub value: u8,
}

impl Deref for Percent {
  type Target = u8;

  fn deref(&self) -> &Self::Target { &self.value }
}

impl fmt::Display for Percent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}%", self.value) }
}

impl Debug for Percent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "PerCent value:{}%", self.value) }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<ChUnitPrimitiveType> for Percent {
  type Error = String;
  fn try_from(arg: ChUnitPrimitiveType) -> Result<Self, Self::Error> {
    match Percent::try_and_convert(arg) {
      Some(percent) => Ok(percent),
      None => Err("Invalid percentage value".to_string()),
    }
  }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<i32> for Percent {
  type Error = String;
  fn try_from(arg: i32) -> Result<Self, Self::Error> {
    match Percent::try_and_convert(arg as u16) {
      Some(percent) => Ok(percent),
      None => Err("Invalid percentage value".to_string()),
    }
  }
}

/// Try and convert given `ChUnit` value to `Percent`. Return `None` if given value is not
/// between 0 and 100.
impl Percent {
  fn try_and_convert(item: ChUnitPrimitiveType) -> Option<Percent> {
    if !(0..=100).contains(&item) {
      return None;
    }
    Percent { value: item as u8 }.into()
  }
}

/// Return the calculated percentage of the given value.
pub fn calc_percentage(percentage: Percent, value: ChUnit) -> ChUnit {
  let percentage_int = percentage.value;
  let percentage_f32 = f32::from(percentage_int) / 100.0;
  let result_f32 = percentage_f32 * f32::from(*value);
  unsafe {
    let converted_value: ChUnitPrimitiveType = result_f32.to_int_unchecked::<ChUnitPrimitiveType>();
    ch!(converted_value)
  }
}

/// Size, defined as [height, width].
#[derive(Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct RequestedSizePercent {
  pub width_pc: Percent,
  pub height_pc: Percent,
}

impl Debug for RequestedSizePercent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "[width:{}, height:{}]", self.width_pc, self.height_pc)
  }
}

#[macro_export]
macro_rules! percent {
  (
    $arg_val: expr
  ) => {
    Percent::try_from($arg_val)
  };
}

#[macro_export]
macro_rules! requested_size_percent {
  (
    width:  $arg_width: expr,
    height: $arg_height: expr
  ) => {
    RequestedSizePercent {
      width_pc: percent!($arg_width)?,
      height_pc: percent!($arg_height)?,
    }
  };
}
