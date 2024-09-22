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

use std::fmt::{Debug, Formatter, Result};

use r3bl_rs_utils_core::{tui_styled_text,
                         RgbValue,
                         TuiColor,
                         TuiStyle,
                         TuiStyledText,
                         TuiStyledTexts,
                         UnicodeString};
use r3bl_rs_utils_macro::tui_style;
use serde::{Deserialize, Serialize};

pub use super::*;
use crate::ColorUtils;

/// Please use the [LolcatBuilder] to create this struct (lots of documentation is provided here).
/// Please do not use this struct directly.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Lolcat {
    pub color_wheel_control: ColorWheelControl,
    pub seed_delta: f64,
}

impl Default for Lolcat {
    fn default() -> Self { LolcatBuilder::new().build() }
}

impl Debug for Lolcat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        return write! { f,
          "lolcat: [{}, {}, {}, {}]",
          pretty_print_f64(self.color_wheel_control.seed),
          pretty_print_f64(self.color_wheel_control.spread),
          pretty_print_f64(self.color_wheel_control.frequency),
          self.color_wheel_control.color_change_speed
        };

        /// More info: <https://stackoverflow.com/questions/63214346/how-to-truncate-f64-to-2-decimal-places>
        fn pretty_print_f64(before: f64) -> f64 { f64::trunc(before * 100.0) / 100.0 }
    }
}

impl Lolcat {
    /// This function does not respect [r3bl_ansi_color::global_color_support::detect()]
    /// (it will always colorize to truecolor regardless of terminal limitations). Use
    /// [crate::ColorWheel] if you want to respect
    /// [r3bl_ansi_color::global_color_support::detect].
    pub fn colorize_to_styled_texts(&mut self, input: &UnicodeString) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();

        for segment in &input.vec_segment {
            let new_color = ColorUtils::get_color_tuple(&self.color_wheel_control);
            let derived_from_new_color = ColorUtils::calc_fg_color(new_color);

            let style = if self.color_wheel_control.background_mode {
                tui_style! (
                    color_fg: TuiColor::Rgb(RgbValue::from_u8(derived_from_new_color.0, derived_from_new_color.1, derived_from_new_color.2))
                    color_bg: TuiColor::Rgb(RgbValue::from_u8(new_color.0, new_color.1, new_color.2))
                )
            } else {
                tui_style! (
                    color_fg: TuiColor::Rgb(RgbValue::from_u8(new_color.0, new_color.1, new_color.2))
                )
            };

            acc += tui_styled_text!(
                @style: style,
                @text: segment.string.clone(),
            );

            self.color_wheel_control.seed +=
                f64::from(self.color_wheel_control.color_change_speed);
        }

        acc
    }

    pub fn next_color(&mut self) { self.color_wheel_control.seed += self.seed_delta; }
}
