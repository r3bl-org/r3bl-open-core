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

use serde::{Deserialize, Serialize};

use crate::{ChUnit, ChUnitPrimitiveType, CommonError, CommonErrorType, ch, glyphs};

/// Represents an integer value between 0 and 100 (inclusive). You can't directly create
/// it, since it has to validate that the value is between 0 and 100. You can create it
/// one of two ways (depending on how you want to handle out of range errors):
///
/// 1. Using the [crate::percent!] macro (which returns a [Result] type, so that you can
///    handle the any conversion out of range errors.
/// 2. Using the [Percent::try_and_convert] method, which returns an [Option] type, so
///    that you can handle the any conversion out of range errors.
///
/// # Fields
/// - `value`: The percentage value as an unsigned 8-bit integer.
///
/// # Traits Implementations
///
/// - [Deref]: Dereferences to [u8].
/// - [fmt::Display]: Formats the percentage value followed by a `%` sign.
/// - [fmt::Debug]: Formats the percentage value with a debug string.
/// - [TryFrom]: Attempts to convert a [ChUnitPrimitiveType] to a `Percent`.
/// - [TryFrom]: Attempts to convert an [i32] to a `Percent`.
///
/// # How to use it
///
/// - [crate::percent!]: A macro that attempts to convert a given expression to `Percent`.
///   Returns [Err] if the value not between 0 and 100.
/// - [Percent::try_and_convert]: Attempts to convert a given [ChUnit] value to `Percent`.
///   Returns [None] if the value is not between 0 and 100.
/// - [Percent::apply_to]: Returns the calculated percentage of the given value.
///
/// # Example
///
/// ```rust
/// use r3bl_core::{Percent, percent};
///
/// // Get as result.
/// let percent = percent!(50);
/// assert_eq!(percent.is_ok(), true);
/// assert_eq!(*percent.unwrap(), 50);
///
/// // Get as option.
/// let percent = Percent::try_and_convert(50);
/// assert_eq!(percent.is_some(), true);
/// assert_eq!(*percent.unwrap(), 50);
///
/// // It implements Display.
/// assert_eq!(percent.unwrap().to_string(), "50%");
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Hash)]
pub struct Percent {
    value: u8,
}

/// Create a [crate::Percent] instance from the given value. It returns a `Result` type,
#[macro_export]
macro_rules! percent {
    (
        $arg_val: expr
    ) => {
        $crate::Percent::try_from($arg_val)
    };
}

impl Deref for Percent {
    type Target = u8;

    fn deref(&self) -> &Self::Target { &self.value }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.value)
    }
}

impl Debug for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PerCent value:{}%", self.value)
    }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<ChUnitPrimitiveType> for Percent {
    type Error = miette::Error;
    fn try_from(arg: ChUnitPrimitiveType) -> miette::Result<Percent> {
        match Percent::try_and_convert(arg) {
            Some(percent) => Ok(percent),
            None => CommonError::new_error_result(
                CommonErrorType::ValueOutOfRange,
                "Invalid percentage value",
            ),
        }
    }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<i32> for Percent {
    type Error = miette::Error;
    fn try_from(arg: i32) -> miette::Result<Percent> {
        match Percent::try_and_convert(arg as u16) {
            Some(percent) => Ok(percent),
            None => CommonError::new_error_result(
                CommonErrorType::ValueOutOfRange,
                "Invalid percentage value",
            ),
        }
    }
}

/// Try and convert given `ChUnit` value to `Percent`. Return `None` if given value is not
/// between 0 and 100.
impl Percent {
    pub fn try_and_convert(item: impl Into<ChUnit>) -> Option<Percent> {
        let item = *item.into();
        if !(0..=100).contains(&item) {
            return None;
        }
        Percent { value: item as u8 }.into()
    }

    /// Given the value, calculate the result of the percentage.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_core::{Percent, ChUnit, ch, percent};
    ///
    /// let percent = percent!(50).unwrap();
    /// let value = ch(5000);
    /// let result = percent.apply_to(value);
    /// assert_eq!(result, ch(2500));
    /// ```
    pub fn apply_to(&self, value: ChUnit) -> ChUnit {
        let percentage_int = self.value;
        let percentage_f32 = f32::from(percentage_int) / 100.0;
        let result_f32 = percentage_f32 * f32::from(*value);
        let converted_value = result_f32.trunc() as ChUnitPrimitiveType;
        ch(converted_value)
    }

    pub fn as_glyph(&self) -> &'static str {
        match self.value {
            0..=25 => glyphs::STATS_25P_GLYPH,
            26..=50 => glyphs::STATS_50P_GLYPH,
            51..=75 => glyphs::STATS_75P_GLYPH,
            _ => glyphs::STATS_100P_GLYPH,
        }
    }
}
