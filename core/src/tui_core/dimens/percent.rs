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

/// Represents an integer value between 0 and 100 (inclusive).
#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct Percent {
  pub value: u8,
}

impl fmt::Display for Percent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}%", self.value) }
}

impl Debug for Percent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "PerCent value:{}%", self.value) }
}

/// https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#
impl TryFrom<UnitType> for Percent {
  type Error = String;
  fn try_from(arg: UnitType) -> Result<Self, Self::Error> {
    match Percent::try_and_convert(arg.into()) {
      Some(percent) => Ok(percent),
      None => Err("Invalid percentage value".to_string()),
    }
  }
}

/// https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#
impl TryFrom<i32> for Percent {
  type Error = String;
  fn try_from(arg: i32) -> Result<Self, Self::Error> {
    match Percent::try_and_convert(arg) {
      Some(percent) => Ok(percent),
      None => Err("Invalid percentage value".to_string()),
    }
  }
}

impl Percent {
  /// Try and convert given `Pair` into a `(Percent, Percent)`. Return
  /// `InvalidLayoutSizePercentage` error if given values are not between 0 and
  /// 100.
  pub fn try_from_pair(pair: Pair) -> CommonResult<(Percent, Percent)> {
    let first = pair.first.try_into();
    let second = pair.second.try_into();

    match (first, second) {
      (Ok(first), Ok(second)) => Ok((first, second)),
      _ => {
        let err_msg = format!("Invalid percentage values in tuple: {:?}", pair);
        CommonError::new(CommonErrorType::ValueOutOfRange, &err_msg)
      }
    }
  }

  /// Try and convert given `UnitType` value to `Percent`. Return `None` if
  /// given value is not between 0 and 100.
  fn try_and_convert(item: i32) -> Option<Percent> {
    if !(0..=100).contains(&item) {
      return None;
    }
    Some(Percent { value: item as u8 })
  }

  /// Wrap `self` in `Option`.
  pub fn as_some(&self) -> Option<Percent> { Some(*self) }
}

/// Return the calculated percentage of the given value.
pub fn calc_percentage(percentage: Percent, value: UnitType) -> UnitType {
  type Integer = UnitType;
  let percentage_int = percentage.value;
  let percentage_f32 = f32::from(percentage_int) / 100.0;
  let result_f32 = percentage_f32 * f32::from(value);

  unsafe { result_f32.to_int_unchecked::<Integer>() }
}

/// Size, defined as [height, width].
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct RequestedSizePercent {
  pub width: Percent,
  pub height: Percent,
}

impl TryFrom<(i32, i32)> for RequestedSizePercent {
  type Error = String;
  fn try_from(arg: (i32, i32)) -> Result<Self, Self::Error> {
    let pair = Percent::try_from_pair(arg.into());
    if pair.is_err() {
      return Err("Problem converting pair to requested size percentage".to_string());
    }
    let pair = pair.unwrap();
    Ok(RequestedSizePercent {
      width: pair.0,
      height: pair.1,
    })
  }
}

impl From<(Percent, Percent)> for RequestedSizePercent {
  fn from(pair: (Percent, Percent)) -> Self {
    RequestedSizePercent {
      width: pair.0,
      height: pair.1,
    }
  }
}

impl RequestedSizePercent {
  /// Wrap `self` in `Option`.
  pub fn as_some(&self) -> Option<Self> { Some(*self) }
}

impl Debug for RequestedSizePercent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "[width:{}, height:{}]", self.width, self.height) }
}
