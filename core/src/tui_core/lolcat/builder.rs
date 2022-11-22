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

use get_size::GetSize;
use serde::*;

use crate::*;

/// A builder struct for the [Lolcat] struct. Example usage:
///
/// ```rust
/// use r3bl_rs_utils_core::*;
///
/// let mut lolcat = LolcatBuilder::new()
///   .set_color_change_speed(ColorChangeSpeed::Rapid)
///   .set_seed(1.0)
///   .set_seed_delta(1.0)
///   .build();
///
/// let content = "Hello, world!";
/// let colored_content = colorize_using_lolcat!(
///   &mut lolcat, "{}", content
/// );
///
/// lolcat.next_color();
/// ```

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, GetSize)]
pub struct LolcatBuilder {
    /// Rate at which the color changes when [format_str](Lolcat::format_str) is called.
    color_change_speed: ColorChangeSpeed,
    /// Initial color of the wheel.
    seed: f64,
    /// Delta that should be applied to the seed for it to change colors.
    seed_delta: f64,
}

impl Default for LolcatBuilder {
    fn default() -> Self {
        Self {
            color_change_speed: ColorChangeSpeed::Slow,
            seed: 1.0,
            seed_delta: 1.0,
        }
    }
}

impl LolcatBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn set_color_change_speed(mut self, color_change_speed: ColorChangeSpeed) -> Self {
        self.color_change_speed = color_change_speed;
        self
    }

    pub fn set_seed(mut self, seed: f64) -> Self {
        self.seed = seed;
        self
    }

    pub fn set_seed_delta(mut self, seed_delta: f64) -> Self {
        self.seed_delta = seed_delta;
        self
    }

    pub fn build(self) -> Lolcat {
        let mut new_lolcat = Lolcat {
            seed_delta: self.seed_delta,
            color_wheel_control: Default::default(),
        };

        new_lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        new_lolcat.color_wheel_control.seed = self.seed;

        new_lolcat
    }

    pub fn apply(&self, lolcat: &mut Lolcat) {
        lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        lolcat.color_wheel_control.seed = self.seed;
        lolcat.seed_delta = self.seed_delta;
    }
}
