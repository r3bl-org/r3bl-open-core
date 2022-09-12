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

use std::ops::Deref;

use get_size::GetSize;
use serde::{Deserialize, Serialize};

use crate::*;

pub type BaseUnitUnderlyingType = u16;

#[derive(
  Copy, Clone, Default, PartialEq, Serialize, Deserialize, GetSize, Ord, PartialOrd, Eq, Debug,
)]
pub struct BaseUnit {
  pub value: BaseUnitUnderlyingType,
}

impl BaseUnit {
  pub fn new(value: BaseUnitUnderlyingType) -> Self { Self { value } }
}

#[macro_export]
macro_rules! base_unit {
  // Returns BaseUnit.
  ($arg: expr) => {{
    let value: BaseUnit = $arg.into();
    value
  }};
  // Returns usize.
  (@to_usize $arg: expr) => {{
    let value: usize = $arg.into();
    value
  }};
}

impl Deref for BaseUnit {
  type Target = BaseUnitUnderlyingType;

  fn deref(&self) -> &Self::Target { &self.value }
}

pub mod math_ops {
  use super::*;

  impl std::ops::Add for BaseUnit {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output { base_unit!(add_unsigned!(self.value, rhs.value)) }
  }

  impl std::ops::AddAssign for BaseUnit {
    fn add_assign(&mut self, rhs: Self) { self.value = add_unsigned!(self.value, rhs.value); }
  }

  impl std::ops::Sub for BaseUnit {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output { base_unit!(sub_unsigned!(self.value, rhs.value)) }
  }

  impl std::ops::Sub<u16> for BaseUnit {
    type Output = Self;

    fn sub(self, rhs: u16) -> Self::Output { base_unit!(sub_unsigned!(self.value, rhs)) }
  }

  impl std::ops::SubAssign for BaseUnit {
    fn sub_assign(&mut self, rhs: Self) { self.value = sub_unsigned!(self.value, rhs.value); }
  }

  impl std::ops::SubAssign<u16> for BaseUnit {
    fn sub_assign(&mut self, rhs: u16) { self.value = sub_unsigned!(self.value, rhs); }
  }

  impl std::ops::Mul for BaseUnit {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output { base_unit!(mul_unsigned!(self.value, rhs.value)) }
  }

  impl std::ops::Mul<u16> for BaseUnit {
    type Output = Self;

    fn mul(self, rhs: u16) -> Self::Output { base_unit!(mul_unsigned!(self.value, rhs)) }
  }
}

pub mod convert_to_number {
  use super::*;

  impl From<BaseUnit> for usize {
    fn from(arg: BaseUnit) -> Self { arg.value as usize }
  }
}

pub mod convert_from_number {
  use super::*;

  impl From<u8> for BaseUnit {
    fn from(value: u8) -> Self {
      Self {
        value: value.try_into().unwrap_or(value as BaseUnitUnderlyingType),
      }
    }
  }

  impl From<BaseUnitUnderlyingType> for BaseUnit {
    fn from(value: BaseUnitUnderlyingType) -> Self { Self { value } }
  }

  impl From<usize> for BaseUnit {
    fn from(value: usize) -> Self {
      Self {
        value: value.try_into().unwrap_or(value as BaseUnitUnderlyingType),
      }
    }
  }

  impl From<i32> for BaseUnit {
    fn from(value: i32) -> Self {
      Self {
        value: value.try_into().unwrap_or(value as BaseUnitUnderlyingType),
      }
    }
  }
}
