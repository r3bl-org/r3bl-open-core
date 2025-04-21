/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use std::{fmt::{Debug, Formatter, Result},
          ops::Deref};

use crate::{ch, glyphs, ChUnit, ChUnitPrimitiveType, CommonError, CommonErrorType};

/// Represents an integer value between 0 and 100 (inclusive). You can't directly create
/// it, since it has to validate that the value is between 0 and 100. You can create it
/// one of two ways (depending on how you want to handle out-of-range errors):
///
/// 1. Using the [crate::pc!] macro, which returns a [Result] type so that you can handle
///    any conversion-out-of-range errors.
/// 2. Using the [Pc::try_and_convert] method, which returns an [Option] type, so that you
///    can handle any conversion-out-of-range errors.
///
/// # Fields
/// - `value`: The percentage value as an unsigned 8-bit integer.
///
/// # Traits Implementations
///
/// - [Deref]: Dereferences to [u8].
/// - [std::fmt::Debug]: Formats the percentage value followed by a `%` sign.
/// - [TryFrom]: Attempts to convert a [ChUnitPrimitiveType] to a `pc`.
/// - [TryFrom]: Attempts to convert an [i32] to a `pc`.
///
/// # How to use it
///
/// - [crate::pc!]: A macro that attempts to convert a given expression to `pc`. Returns
///   [Err] if the value is not between 0 and 100.
/// - [Pc::try_and_convert]: Attempts to convert a given [ChUnit] value to `pc`. Returns
///   [None] if the value is not between 0 and 100.
/// - [Pc::apply_to]: Returns the calculated percentage of the given value.
///
/// # Example
///
/// ```rust
/// use r3bl_tui::{Pc, pc};
///
/// // Get as a result.
/// let percent = pc!(50);
/// assert_eq!(percent.is_ok(), true);
/// assert_eq!(*percent.unwrap(), 50);
///
/// // Get as an option.
/// let percent = Pc::try_and_convert(50);
/// assert_eq!(percent.is_some(), true);
/// assert_eq!(*percent.unwrap(), 50);
///
/// // It implements Debug, not Display.
/// assert_eq!(format!("{:?}", percent.unwrap()), "50%");
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Default, Hash)]
pub struct Pc {
    value: u8,
}

/// Create a [crate::Pc] instance from the given value. It returns a `Result` type.
#[macro_export]
macro_rules! pc {
    (
        $arg_val: expr
    ) => {
        $crate::Pc::try_from($arg_val)
    };
}

impl Deref for Pc {
    type Target = u8;

    fn deref(&self) -> &Self::Target { &self.value }
}

impl Debug for Pc {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result { write!(f, "{}%", self.value) }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<ChUnitPrimitiveType> for Pc {
    type Error = miette::Error;
    fn try_from(arg: ChUnitPrimitiveType) -> miette::Result<Pc> {
        match Pc::try_and_convert(arg) {
            Some(pc) => Ok(pc),
            None => CommonError::new_error_result(
                CommonErrorType::ValueOutOfRange,
                "Invalid percentage value",
            ),
        }
    }
}

/// <https://doc.rust-lang.org/stable/std/convert/trait.TryFrom.html#>
impl TryFrom<i32> for Pc {
    type Error = miette::Error;
    fn try_from(arg: i32) -> miette::Result<Pc> {
        match Pc::try_and_convert(arg as u16) {
            Some(pc) => Ok(pc),
            None => CommonError::new_error_result(
                CommonErrorType::ValueOutOfRange,
                "Invalid percentage value",
            ),
        }
    }
}

/// Try and convert the given ` ChUnit ` value to `pc`.
///
/// Return `None` if the given value is not between 0 and 100.
impl Pc {
    pub fn try_and_convert(arg_num: impl Into<ChUnit>) -> Option<Pc> {
        let item = *arg_num.into();
        if !(0..=100).contains(&item) {
            return None;
        }
        Pc { value: item as u8 }.into()
    }

    /// Given the value, calculate the result of the percentage.
    ///
    /// # Example
    ///
    /// ```rust
    /// use r3bl_tui::{pc, ChUnit, ch, Pc};
    ///
    /// let percent = pc!(50).unwrap();
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

#[cfg(test)]
mod tests {
    use crate::{ch,
                Pc,
                STATS_100P_GLYPH,
                STATS_25P_GLYPH,
                STATS_50P_GLYPH,
                STATS_75P_GLYPH};

    #[test]
    fn test_pc_works_as_expected() {
        let maybe_pc_100 = pc!(100i32);
        if let Ok(pc_100) = maybe_pc_100 {
            assert_eq!(*pc_100, 100);
            let result = pc_100.apply_to(ch(500));
            assert_eq!(*result, 500);
        } else {
            panic!("Failed to create pc from 100");
        }

        let pc_50 = Pc::try_from(50i32).unwrap();
        assert_eq!(*pc_50, 50);
        let result = pc_50.apply_to(ch(500));
        assert_eq!(*result, 250);

        let pc_0 = Pc::try_from(0i32).unwrap();
        assert_eq!(*pc_0, 0);
        let result = pc_0.apply_to(ch(500));
        assert_eq!(*result, 0);
    }

    #[test]
    fn test_pc_parsing_fails_as_expected() {
        Pc::try_from(-1i32).unwrap_err();

        Pc::try_from(0i32).unwrap();
        Pc::try_from(0u16).unwrap();

        Pc::try_from(100i32).unwrap();
        Pc::try_from(100u16).unwrap();

        Pc::try_from(101i32).unwrap_err();
        Pc::try_from(101u16).unwrap_err();
    }

    #[test]
    fn test_pc_to_glyph_works_as_expected() {
        let pc_0_to_25 = pc!(25i32).unwrap();
        assert_eq!(pc_0_to_25.as_glyph(), STATS_25P_GLYPH);

        let pc_25_to_50 = pc!(50i32).unwrap();
        assert_eq!(pc_25_to_50.as_glyph(), STATS_50P_GLYPH);

        let pc_50_to_75 = pc!(75i32).unwrap();
        assert_eq!(pc_50_to_75.as_glyph(), STATS_75P_GLYPH);

        let pc_100 = pc!(100i32).unwrap();
        assert_eq!(pc_100.as_glyph(), STATS_100P_GLYPH);
    }
}
