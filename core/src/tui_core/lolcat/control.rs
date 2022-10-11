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

use std::fmt::Display;

use atty::Stream;
use get_size::GetSize;
use rand::random;
use serde::*;

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

impl Display for ColorChangeSpeed {
  fn fmt(&self, f: &mut __private::Formatter<'_>) -> std::fmt::Result {
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
  pub fn new(seed: &str, spread: &str, frequency: &str, color_change: ColorChangeSpeed) -> ColorWheelControl {
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
      print_color: atty::is(Stream::Stdout),
      color_change_speed: color_change,
    }
  }
}

impl Default for ColorWheelControl {
  fn default() -> Self { Self::new("0.0", "3.0", "0.1", ColorChangeSpeed::Slow) }
}
