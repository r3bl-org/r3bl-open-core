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

use std::fmt::{Debug, Display, Formatter, Result};

use get_size::GetSize;
use is_terminal::IsTerminal;
use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::*;
use rand::random;
use serde::*;

use crate::*;

/// Please use the [LolcatBuilder] to create this struct (lots of documentation is provided here).
/// Please do not use this struct directly.
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, GetSize)]
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
    /// This function does not respect [ColorSupport] (it will always colorize to truecolor
    /// regardless of terminal limitations). Use [ColorWheel] if you want to respect [ColorSupport].
    pub fn colorize_to_styled_texts(&mut self, input: &UnicodeString) -> StyledTexts {
        let mut acc = StyledTexts::default();

        for segment in &input.vec_segment {
            let new_color = ColorUtils::get_color_tuple(&self.color_wheel_control);
            let derived_from_new_color = ColorUtils::calc_fg_color(new_color);

            let style = if self.color_wheel_control.background_mode {
                style! (
                    color_fg: TuiColor::Rgb(RgbValue::from_u8(derived_from_new_color.0, derived_from_new_color.1, derived_from_new_color.2))
                    color_bg: TuiColor::Rgb(RgbValue::from_u8(new_color.0, new_color.1, new_color.2))
                )
            } else {
                style! (
                    color_fg: TuiColor::Rgb(RgbValue::from_u8(new_color.0, new_color.1, new_color.2))
                )
            };

            acc += styled_text!(
                @style: style,
                @text: segment.string.clone(),
            );

            self.color_wheel_control.seed += f64::from(self.color_wheel_control.color_change_speed);
        }

        acc
    }

    pub fn next_color(&mut self) { self.color_wheel_control.seed += self.seed_delta; }
}

mod control_wheel_control {
    use super::*;

    /// A struct to contain info we need to print with every character.
    #[derive(Debug, Clone, Copy, Serialize, Deserialize, GetSize)]
    pub struct ColorWheelControl {
        pub seed: f64,
        pub spread: f64,
        pub frequency: f64,
        pub background_mode: bool,
        pub dialup_mode: bool,
        pub print_color: bool,
        pub color_change_speed: ColorChangeSpeed,
    }

    impl PartialEq for ColorWheelControl {
        /// More info:
        /// 1. <https://stackoverflow.com/questions/67951688/comparing-structs-with-floating-point-numbers-in-rust>
        /// 2. <https://doc.rust-lang.org/std/primitive.f64.html#associatedconstant.EPSILON>
        /// 3. <https://rust-lang.github.io/rust-clippy/master/index.html#float_equality_without_abs>
        fn eq(&self, other: &Self) -> bool {
            (self.seed - other.seed).abs() < f64::EPSILON // self.seed == other.seed
      && self.spread == other.spread
      && self.frequency == other.frequency
      && self.background_mode == other.background_mode
      && self.dialup_mode == other.dialup_mode
      && self.print_color == other.print_color
      && self.color_change_speed == other.color_change_speed
        }
    }

    #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, GetSize)]
    pub enum ColorChangeSpeed {
        Rapid,
        Slow,
    }

    impl Default for ColorChangeSpeed {
        fn default() -> Self { Self::Rapid }
    }

    impl Display for ColorChangeSpeed {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                ColorChangeSpeed::Rapid => write!(f, "Rapid"),
                ColorChangeSpeed::Slow => write!(f, "Slow"),
            }
        }
    }

    impl From<ColorChangeSpeed> for f64 {
        /// The float is added to seed in [crate::Lolcat] after every iteration. If
        /// the number is `Rapid` then the changes in color between new lines is
        /// quite abrupt. If it is `Slow` then the changes are much much smoother.
        /// And so this is the default.
        fn from(value: ColorChangeSpeed) -> Self {
            match value {
                ColorChangeSpeed::Rapid => 1.0,
                ColorChangeSpeed::Slow => 0.1,
            }
        }
    }

    impl ColorWheelControl {
        pub fn new(
            seed: &str,
            spread: &str,
            frequency: &str,
            color_change: ColorChangeSpeed,
        ) -> ColorWheelControl {
            let mut seed: f64 = seed.parse().unwrap();
            if seed == 0.0 {
                seed = random::<f64>() * 10e9;
            }
            let spread: f64 = spread.parse().unwrap();
            let frequency: f64 = frequency.parse().unwrap();
            let color_change = color_change;

            ColorWheelControl {
                seed,
                spread,
                frequency,
                background_mode: false,
                dialup_mode: false,
                print_color: std::io::stdout().is_terminal(),
                color_change_speed: color_change,
            }
        }
    }

    impl Default for ColorWheelControl {
        fn default() -> Self { Self::new("0.0", "3.0", "0.1", ColorChangeSpeed::Slow) }
    }
}
pub use control_wheel_control::*;
