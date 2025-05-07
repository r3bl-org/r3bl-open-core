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

use std::fmt::{Debug, Formatter, Result};

use super::{Colorize, LolcatBuilder, Seed, SeedDelta};
use crate::{color_helpers,
            tui_color,
            tui_styled_text,
            ColorWheelControl,
            GCString,
            TuiStyle,
            TuiStyledTexts};

/// Please use the [LolcatBuilder] to create this struct (lots of documentation is
/// provided here). Please do not use this struct directly.
#[derive(Clone, Copy, PartialEq)]
pub struct Lolcat {
    pub color_wheel_control: ColorWheelControl,
    pub seed_delta: SeedDelta,
}

impl Default for Lolcat {
    fn default() -> Self { LolcatBuilder::new().build() }
}

impl Debug for Lolcat {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        return write! { f,
          "lolcat: [{}, {}, {}, {}]",
          pretty_print_f64(*self.color_wheel_control.seed),
          pretty_print_f64(*self.color_wheel_control.spread),
          pretty_print_f64(*self.color_wheel_control.frequency),
          self.color_wheel_control.color_change_speed
        };

        /// More info: <https://stackoverflow.com/questions/63214346/how-to-truncate-f64-to-2-decimal-places>
        fn pretty_print_f64(before: f64) -> f64 { f64::trunc(before * 100.0) / 100.0 }
    }
}

impl Lolcat {
    /// This function does not respect [crate::global_color_support::detect()]
    /// (it will always colorize to truecolor regardless of terminal limitations). Use
    /// [crate::ColorWheel] if you want to respect
    /// [crate::global_color_support::detect].
    pub fn colorize_to_styled_texts(&mut self, us: &GCString) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();

        for seg_str in us.iter() {
            let new_color = color_helpers::get_color_tuple(&self.color_wheel_control);
            let derived_from_new_color = color_helpers::calc_fg_color(new_color);

            let style = if self.color_wheel_control.background_mode
                == Colorize::BothBackgroundAndForeground
            {
                TuiStyle {
                    color_fg: Some(tui_color!(
                        derived_from_new_color.0,
                        derived_from_new_color.1,
                        derived_from_new_color.2,
                    )),
                    color_bg: Some(tui_color!(new_color.0, new_color.1, new_color.2,)),
                    ..Default::default()
                }
            } else {
                TuiStyle {
                    color_fg: Some(tui_color!(new_color.0, new_color.1, new_color.2,)),
                    ..Default::default()
                }
            };

            acc += tui_styled_text!(
                @style: style,
                @text: seg_str,
            );

            self.color_wheel_control.seed +=
                Seed::from(self.color_wheel_control.color_change_speed);
        }

        acc
    }

    pub fn next_color(&mut self) { self.color_wheel_control.seed += self.seed_delta; }
}
