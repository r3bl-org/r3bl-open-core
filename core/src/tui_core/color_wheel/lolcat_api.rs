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

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use super::Lolcat;
use crate::{ColorChangeSpeed, TuiStyledTexts, UnicodeString};

pub fn colorize_to_styled_texts(
    lolcat: &mut Lolcat,
    input: &UnicodeString,
) -> TuiStyledTexts {
    lolcat.colorize_to_styled_texts(input)
}

pub fn lolcat_each_char_in_unicode_string(
    unicode_string: &UnicodeString,
    lolcat: Option<&mut Lolcat>,
) -> TuiStyledTexts {
    let mut saved_orig_speed = None;

    let mut my_lolcat: Cow<'_, Lolcat> = match lolcat {
        Some(lolcat_arg) => {
            saved_orig_speed = Some(lolcat_arg.color_wheel_control.color_change_speed);
            lolcat_arg.color_wheel_control.color_change_speed = ColorChangeSpeed::Rapid;
            Cow::Borrowed(lolcat_arg)
        }
        None => {
            let lolcat_temp = LolcatBuilder::new()
                .set_color_change_speed(ColorChangeSpeed::Rapid)
                .build();
            Cow::Owned(lolcat_temp)
        }
    };

    let it = my_lolcat.to_mut().colorize_to_styled_texts(unicode_string);

    // Restore saved_orig_speed if it was set.
    if let Some(orig_speed) = saved_orig_speed {
        my_lolcat.to_mut().color_wheel_control.color_change_speed = orig_speed;
    }

    it
}

/// A builder struct for the [Lolcat] struct. Example usage:
///
/// ```rust
/// use r3bl_core::*;
///
/// let mut lolcat = LolcatBuilder::new()
///   .set_color_change_speed(ColorChangeSpeed::Rapid)
///   .set_seed(1.0)
///   .set_seed_delta(1.0)
///   .build();
///
/// let content = "Hello, world!";
/// let unicode_string = UnicodeString::from(content);
/// let lolcat_mut = &mut lolcat;
/// let st = lolcat_mut.colorize_to_styled_texts(&unicode_string);
///
/// lolcat.next_color();
/// ```
///
/// This [Lolcat] that is returned by `build()` is safe to re-use.
/// - The colors it cycles through are "stable" meaning that once constructed via the
///   [builder](LolcatBuilder) (which sets the speed, seed, and delta that determine where the color
///   wheel starts when it is used). For eg, when used in a dialog box component that re-uses the
///   instance, repeated calls to the `render()` function of this component will produce the
///   same generated colors over and over again.
/// - If you want to change where the color wheel "begins", you have to change the speed, seed, and
///   delta of this [Lolcat] instance.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct LolcatBuilder {
    /// Rate at which the color changes when
    /// [colorize_to_styled_texts](Lolcat::colorize_to_styled_texts) is called.
    pub color_change_speed: ColorChangeSpeed,
    /// Initial color of the wheel.
    pub seed: f64,
    /// Delta that should be applied to the seed for it to change colors.
    pub seed_delta: f64,
    /// - `true` means the background is colorized, and the foreground is computed for contrast. The
    ///   primary effect here is that the background of the generated colors is what is being
    ///   lolcat'd.
    /// - `false` means that only the foreground color is cycled, background is left alone.
    pub background_mode: bool,
}

impl Default for LolcatBuilder {
    fn default() -> Self {
        Self {
            color_change_speed: ColorChangeSpeed::Slow,
            seed: 1.0,
            seed_delta: 1.0,
            background_mode: false, /* color only the foreground */
        }
    }
}

impl LolcatBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn set_background_mode(mut self, background_mode: bool) -> Self {
        self.background_mode = background_mode;
        self
    }

    pub fn set_color_change_speed(
        mut self,
        color_change_speed: ColorChangeSpeed,
    ) -> Self {
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
        new_lolcat.color_wheel_control.background_mode = self.background_mode;

        new_lolcat
    }

    pub fn apply(&self, lolcat: &mut Lolcat) {
        lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        lolcat.color_wheel_control.seed = self.seed;
        lolcat.seed_delta = self.seed_delta;
    }
}
