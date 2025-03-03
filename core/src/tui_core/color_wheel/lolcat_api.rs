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

use std::borrow::Cow;

use super::{Lolcat, Seed, SeedDelta};
use crate::{ColorChangeSpeed, GCString, GCStringExt as _, TuiStyledTexts};

pub fn colorize_to_styled_texts(
    lolcat: &mut Lolcat,
    arg_str: impl AsRef<str>,
) -> TuiStyledTexts {
    let str = arg_str.as_ref();
    let string_gcs = str.grapheme_string();
    lolcat.colorize_to_styled_texts(&string_gcs)
}

pub fn lolcat_each_char_in_unicode_string(
    us: &GCString,
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

    let it = my_lolcat.to_mut().colorize_to_styled_texts(us);

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
/// let string = "Hello, world!";
/// let string_gcs = string.grapheme_string();
/// let lolcat_mut = &mut lolcat;
/// let st = lolcat_mut.colorize_to_styled_texts(&string_gcs);
/// lolcat.next_color();
/// ```
///
/// This [Lolcat] that is returned by `build()` is safe to re-use.
/// - The colors it cycles through are "stable" meaning that once constructed via the
///   [builder](LolcatBuilder) (which sets the speed, seed, and delta that determine where
///   the color wheel starts when it is used). For eg, when used in a dialog box component
///   that re-uses the instance, repeated calls to the `render()` function of this
///   component will produce the same generated colors over and over again.
/// - If you want to change where the color wheel "begins", you have to change the speed,
///   seed, and delta of this [Lolcat] instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LolcatBuilder {
    /// Rate at which the color changes when
    /// [colorize_to_styled_texts](Lolcat::colorize_to_styled_texts) is called.
    pub color_change_speed: ColorChangeSpeed,
    /// Initial color of the wheel.
    pub seed: Seed,
    /// Delta that should be applied to the seed for it to change colors.
    pub seed_delta: SeedDelta,
    /// Colorize the background and foreground, or just the foreground.
    pub colorization_strategy: Colorize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colorize {
    /// The background is colorized, and the foreground is computed for contrast. The
    /// primary effect here is that the background of the generated colors is what is
    /// being lolcat'd.
    BothBackgroundAndForeground,
    /// Only the foreground color is cycled, background is left alone.
    OnlyForeground,
}

impl Default for LolcatBuilder {
    fn default() -> Self {
        Self {
            color_change_speed: ColorChangeSpeed::Slow,
            seed: 1.0.into(),
            seed_delta: 1.0.into(),
            colorization_strategy: Colorize::OnlyForeground, /* color only the foreground */
        }
    }
}

impl LolcatBuilder {
    pub fn new() -> Self { Self::default() }

    pub fn set_background_mode(mut self, background_mode: Colorize) -> Self {
        self.colorization_strategy = background_mode;
        self
    }

    pub fn set_color_change_speed(
        mut self,
        color_change_speed: ColorChangeSpeed,
    ) -> Self {
        self.color_change_speed = color_change_speed;
        self
    }

    pub fn set_seed(mut self, arg_seed: impl Into<Seed>) -> Self {
        self.seed = arg_seed.into();
        self
    }

    pub fn set_seed_delta(mut self, arg_seed_delta: impl Into<SeedDelta>) -> Self {
        self.seed_delta = arg_seed_delta.into();
        self
    }

    pub fn build(self) -> Lolcat {
        let mut new_lolcat = Lolcat {
            seed_delta: self.seed_delta,
            color_wheel_control: Default::default(),
        };

        new_lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        new_lolcat.color_wheel_control.seed = self.seed;
        new_lolcat.color_wheel_control.background_mode = self.colorization_strategy;

        new_lolcat
    }

    pub fn apply(&self, lolcat: &mut Lolcat) {
        lolcat.color_wheel_control.color_change_speed = self.color_change_speed;
        lolcat.color_wheel_control.seed = self.seed;
        lolcat.seed_delta = self.seed_delta;
    }
}
