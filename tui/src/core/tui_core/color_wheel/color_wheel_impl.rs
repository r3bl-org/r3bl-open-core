/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::fmt::Write;

use sizing::VecConfigs;
use smallvec::SmallVec;

use super::{config::sizing::VecSteps,
            defaults::{get_default_gradient_stops, Defaults},
            ColorWheelConfig,
            ColorWheelDirection,
            ColorWheelSpeed,
            GradientKind,
            GradientLengthKind,
            Lolcat,
            LolcatBuilder,
            Seed};
use crate::{ast,
            ch,
            color_helpers,
            generate_random_truecolor_gradient,
            generate_truecolor_gradient,
            get_gradient_array_for,
            glyphs::SPACER_GLYPH as SPACER,
            tui_color,
            tui_styled_text,
            u8,
            usize,
            Ansi256GradientIndex,
            ChUnit,
            GCString,
            GCStringExt as _,
            GradientGenerationPolicy,
            InlineString,
            RgbValue,
            TextColorizationPolicy,
            TuiColor,
            TuiStyle,
            TuiStyledText,
            TuiStyledTexts};

/// These are sized to allow for stack allocation rather than heap allocation. If for some
/// reason these are exceeded, then they will [`smallvec::SmallVec::spilled`] over into
/// the heap.
pub(in crate::core::tui_core) mod sizing {
    use super::*;

    pub type VecConfigs = SmallVec<[ColorWheelConfig; MAX_CONFIGS]>;
    const MAX_CONFIGS: usize = 2;
}

#[derive(Clone, PartialEq, Debug)]
pub struct ColorWheel {
    pub configs: VecConfigs,
    pub gradient_kind: GradientKind,
    pub gradient_length_kind: GradientLengthKind,
    pub index: ChUnit,
    pub index_direction: ColorWheelDirection,
    pub counter: ChUnit,
}

impl Default for ColorWheel {
    fn default() -> Self {
        let mut acc = VecConfigs::new();

        let config_1 = {
            ColorWheelConfig::Rgb(
                get_default_gradient_stops(),
                ColorWheelSpeed::Medium,
                Defaults::Steps as u8,
            )
        };

        let config_2 = ColorWheelConfig::Ansi256(
            Ansi256GradientIndex::MediumGreenToMediumBlue,
            ColorWheelSpeed::Medium,
        );

        acc.push(config_1);
        acc.push(config_2);

        Self::new(acc)
    }
}

impl ColorWheel {
    /// This will lazily create a color wheel. It does not compute the gradient and
    /// memoize it when this function is called.
    ///
    /// 1. The heavy lifting is done when
    ///    [`generate_color_wheel`](ColorWheel::generate_color_wheel) is called.
    /// 2. When you use
    ///    [`colorize_into_styled_texts`](ColorWheel::colorize_into_styled_texts) it will
    ///    also also call this method.
    ///
    /// # Arguments
    /// 1. `configs`: A list of color wheel configs. The order of the configs is not
    ///    important. However, at the very least, one Truecolor config & one ANSI 256
    ///    config should be provided. The fallback is always grayscale. See
    ///    [`ColorWheelConfig::narrow_config_based_on_color_support`],
    ///    [`crate::global_color_support::detect`] for more info.
    #[must_use]
    pub fn new(configs: VecConfigs) -> Self {
        Self {
            configs,
            gradient_kind: GradientKind::NotCalculatedYet,
            gradient_length_kind: GradientLengthKind::NotCalculatedYet,
            index: ch(0),
            index_direction: ColorWheelDirection::Forward,
            counter: ch(0),
        }
    }

    /// This method will return the index of the current color in the gradient. If the
    /// color wheel is a lolcat, then the seed * 1000 is returned. If the gradient has
    /// not been computed yet, then 0 is returned.
    #[must_use]
    pub fn get_index(&self) -> ChUnit {
        match self.gradient_kind {
            GradientKind::ColorWheel(_) => self.index,
            GradientKind::Lolcat(lolcat) => lolcat_helper::convert_lolcat_seed_to_index(
                lolcat.color_wheel_control.seed,
            ),
            GradientKind::NotCalculatedYet => ch(0),
        }
    }

    /// This method will return the length of the gradient. This is
    /// [`GradientLengthKind::NotCalculatedYet`] if the gradient has not been computed &
    /// memoized yet via a call to
    /// [`generate_color_wheel`](ColorWheel::generate_color_wheel).
    #[must_use]
    pub fn get_gradient_len(&self) -> GradientLengthKind { self.gradient_length_kind }

    pub fn get_gradient_kind(&mut self) -> &mut GradientKind { &mut self.gradient_kind }

    /// Every time this method is called, it will generate the gradient & memoize it.
    ///
    /// # Arguments
    /// * `steps_override` - If `Some` then the number of steps will be overridden. If
    ///   `None` then the number of steps will be determined by the `ColorWheelConfig`.
    ///
    /// Here's the priority order of how `steps` is determined:
    /// 1. If `steps_override` is `Some` then use that.
    /// 2. If `steps_override` is `None` then use the steps from the `ColorWheelConfig`.
    /// 3. If nothing is found in `ColorWheelConfig` then use `DEFAULT_STEPS`.
    ///
    /// # Errors
    /// If the RGB color is invalid, then this method will panic.
    pub fn generate_color_wheel(
        &mut self,
        maybe_steps_override: Option<u8>,
    ) -> &GradientKind {
        let my_config =
            ColorWheelConfig::narrow_config_based_on_color_support(&self.configs);

        let steps: u8 =
            gradient_generation_helper::determine_steps(maybe_steps_override, &my_config);

        // Generate new gradient and replace the old one.
        // More info: https://github.com/Ogeon/palette/tree/master/palette#gradients
        match &my_config {
            ColorWheelConfig::Lolcat(builder) => {
                gradient_generation_helper::set_lolcat_gradient(self, builder);
            }

            ColorWheelConfig::Rgb(color_stops, _, _) => {
                // Generate new gradient.
                let new_gradient = generate_truecolor_gradient(color_stops, steps);
                gradient_generation_helper::set_gradient_and_length(self, new_gradient);
            }

            ColorWheelConfig::RgbRandom(_) => {
                // Generate new random gradient.
                let new_gradient = generate_random_truecolor_gradient(steps);
                gradient_generation_helper::set_gradient_and_length(self, new_gradient);
            }

            ColorWheelConfig::Ansi256(index, _) => {
                let gradient_vec =
                    gradient_generation_helper::generate_ansi256_gradient(*index);
                gradient_generation_helper::set_gradient_and_length(self, gradient_vec);
            }
        }

        &self.gradient_kind
    }

    /// This method will return the next color in the gradient. It updates the index. When
    /// it reaches the end of the gradient, it will flip direction and go in reverse.
    /// And then flip again when it reaches the start. And so on.
    pub fn next_color(&mut self) -> Option<TuiColor> {
        // Early return if the following can't be found.
        if let GradientKind::NotCalculatedYet = self.gradient_kind {
            return None;
        }

        // Get the gradient.
        let my_config =
            ColorWheelConfig::narrow_config_based_on_color_support(&self.configs);

        // Early return if lolcat.
        if let ColorWheelConfig::Lolcat(_) = &my_config {
            return if let GradientKind::Lolcat(lolcat) = &mut self.gradient_kind {
                Some(lolcat_helper::generate_next_lolcat_color(lolcat))
            } else {
                None
            };
        }

        // Determine if the index should be changed (depending on the speed).
        let (should_change_index, new_counter) =
            color_wheel_navigation::should_update_index(&my_config, self.counter);
        self.counter = new_counter;

        let GradientKind::ColorWheel(gradient) = &mut self.gradient_kind else {
            return None;
        };

        // Actually change the index if it should be changed.
        if should_change_index {
            color_wheel_navigation::update_index_with_direction(
                &mut self.index,
                &mut self.index_direction,
                gradient.len(),
            );
        }

        // Return the color for the correct index.
        color_wheel_navigation::get_color_at_index(gradient, self.index)
    }

    /// This method will reset the index to zero.
    fn reset_index(&mut self) {
        // If this is a lolcat, reset the seed, and early return.
        if let GradientLengthKind::Lolcat(seed) = self.get_gradient_len() {
            if let GradientKind::Lolcat(lolcat) = self.get_gradient_kind() {
                lolcat.color_wheel_control.seed = seed;
                return;
            }
        }

        // Not a lolcat so reset the index and direction.
        self.index = ch(0);
        self.index_direction = ColorWheelDirection::Forward;
    }

    /// Simplified version of [`ColorWheel::colorize_into_string`] with some defaults.
    #[must_use]
    pub fn lolcat_into_string(
        text: &str,
        maybe_default_style: Option<TuiStyle>,
    ) -> InlineString {
        ColorWheel::default().colorize_into_string(
            text,
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
            maybe_default_style,
        )
    }

    /// See [`ColorWheel::lolcat_into_string`] for an easy to use version of this
    /// function.
    pub fn colorize_into_string(
        &mut self,
        string: &str,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
        maybe_default_style: Option<TuiStyle>,
    ) -> InlineString {
        let string_gcs = string.grapheme_string();
        let spans_in_line = self.colorize_into_styled_texts(
            &string_gcs,
            gradient_generation_policy,
            text_colorization_policy,
        );

        let mut acc = InlineString::new();

        for TuiStyledText { mut style, text } in spans_in_line.inner {
            if let Some(default_style) = maybe_default_style {
                style += default_style;
            }
            let ansi_styled_text = ast(text, style);
            _ = write!(acc, "{ansi_styled_text}");
        }

        acc
    }

    /// This method gives you fine grained control over the color wheel. It returns a
    /// gradient-colored string. It respects the [`crate::ColorSupport`]
    /// restrictions for the terminal.
    ///
    /// # Colorization Policy
    ///
    /// - [`GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength`] - The
    ///   first time this method is called it will generate a gradient w/ the number of
    ///   steps. Subsequent calls will use the same gradient and index **if** the number
    ///   of steps is the same. However, if the number of steps are different, then a new
    ///   gradient will be generated & the index reset.
    ///
    /// - [`GradientGenerationPolicy::ReuseExistingGradientAndIndex`] - The first time
    ///   this method is called it will generate a gradient w/ the number of steps.
    ///   Subsequent calls will use the same gradient and index.
    pub fn colorize_into_styled_texts(
        &mut self,
        us: &GCString,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
    ) -> TuiStyledTexts {
        self.generate_gradient(us, gradient_generation_policy);
        self.generate_styled_texts(text_colorization_policy, us)
    }

    fn generate_styled_texts(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCString,
    ) -> TuiStyledTexts {
        if ColorWheelConfig::config_contains_bg_lolcat(&self.configs) {
            self.generate_styled_texts_for_lolcat_bg(text_colorization_policy, us)
        } else {
            self.generate_styled_texts_regular(text_colorization_policy, us)
        }
    }

    /// Handle special case for lolcat background mode.
    fn generate_styled_texts_for_lolcat_bg(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCString,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        let maybe_style =
            lolcat_bg_helper::extract_style_from_policy(text_colorization_policy);

        // Loop: Colorize each (next) character w/ (next) color.
        for next_seg_str in us {
            let styled_text =
                self.create_lolcat_bg_styled_text(maybe_style, next_seg_str);
            acc += styled_text;
        }

        acc
    }

    /// Handle regular colorization cases.
    fn generate_styled_texts_regular(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        us: &GCString,
    ) -> TuiStyledTexts {
        match text_colorization_policy {
            TextColorizationPolicy::ColorEachCharacter(maybe_style) => {
                self.colorize_each_character(maybe_style, us)
            }
            TextColorizationPolicy::ColorEachWord(maybe_style) => {
                self.colorize_each_word(maybe_style, us)
            }
        }
    }

    /// Colorize each character with a different color.
    fn colorize_each_character(
        &mut self,
        maybe_style: Option<TuiStyle>,
        us: &GCString,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        for next_seg_str in us {
            acc += tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_color_for(maybe_style, self.next_color()),
                @text: next_seg_str,
            );
        }
        acc
    }

    /// Colorize each word with a different color.
    fn colorize_each_word(
        &mut self,
        maybe_style: Option<TuiStyle>,
        us: &GCString,
    ) -> TuiStyledTexts {
        let mut acc = TuiStyledTexts::default();
        // More info on peekable: https://stackoverflow.com/a/67872822/2085356
        let mut peekable = us.string.split_ascii_whitespace().peekable();
        while let Some(next_word) = peekable.next() {
            // Loop: Colorize each (next) word w/ (next) color.
            acc += tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_color_for(maybe_style, self.next_color()),
                @text: next_word,
            );
            if peekable.peek().is_some() {
                acc += tui_styled_text!(
                    @style: TuiStyle::default(),
                    @text: SPACER,
                );
            }
        }
        acc
    }

    /// Create a styled text for lolcat background mode.
    fn create_lolcat_bg_styled_text(
        &mut self,
        maybe_style: Option<TuiStyle>,
        text: &str,
    ) -> TuiStyledText {
        let maybe_next_bg_color = self.next_color();

        if let Some(next_bg_color) = maybe_next_bg_color {
            let maybe_bg_color =
                lolcat_bg_helper::convert_tui_color_to_rgb_tuple(next_bg_color);

            if let Some((bg_red, bg_green, bg_blue)) = maybe_bg_color {
                let (fg_red, fg_green, fg_blue) =
                    color_helpers::calc_fg_color((bg_red, bg_green, bg_blue));
                tui_styled_text!(
                    @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(
                        maybe_style,
                        Some(tui_color!(fg_red, fg_green, fg_blue)),
                        Some(tui_color!(bg_red, bg_green, bg_blue)),
                    ),
                    @text: text,
                )
            } else {
                tui_styled_text!(
                    @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(maybe_style, None, None,),
                    @text: text,
                )
            }
        } else {
            tui_styled_text!(
                @style: generate_styled_texts_helper::gen_style_fg_bg_color_for(maybe_style, None, None,),
                @text: text,
            )
        }
    }

    fn generate_gradient(
        &mut self,
        us: &GCString,
        gradient_generation_policy: GradientGenerationPolicy,
    ) {
        match gradient_generation_policy {
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength => {
                let steps = u8(*us.len());

                // Generate a new gradient if one doesn't exist.
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(Some(steps));
                    return;
                }

                // Re-use gradient if possible.
                if let GradientLengthKind::ColorWheel(length) = self.get_gradient_len() {
                    if u8(ch(length)) != steps {
                        self.generate_color_wheel(Some(steps));
                    }
                }

                self.reset_index();
            }

            GradientGenerationPolicy::ReuseExistingGradientAndIndex => {
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(None);
                }
            }

            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex => {
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(None);
                }

                self.reset_index();
            }
        }
    }
}

mod generate_styled_texts_helper {
    use super::*;

    // Inner function.
    pub fn gen_style_fg_color_for(
        maybe_style: Option<TuiStyle>,
        next_color: Option<TuiColor>,
    ) -> TuiStyle {
        let mut it = TuiStyle {
            color_fg: next_color,
            ..Default::default()
        };
        it += &maybe_style;
        it
    }

    // Inner function.
    pub fn gen_style_fg_bg_color_for(
        maybe_style: Option<TuiStyle>,
        next_fg_color: Option<TuiColor>,
        next_background_color: Option<TuiColor>,
    ) -> TuiStyle {
        let mut it = TuiStyle {
            color_fg: next_fg_color,
            color_bg: next_background_color,
            ..Default::default()
        };
        it += &maybe_style;
        it
    }
}

/// Helper module for color wheel index management and navigation.
mod color_wheel_navigation {
    use super::*;

    /// Determine if the color wheel index should be updated based on speed settings.
    pub fn should_update_index(
        config: &ColorWheelConfig,
        counter: ChUnit,
    ) -> (bool, ChUnit) {
        let speed_threshold = match config {
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Fast, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Fast) => {
                ColorWheelSpeed::Fast as u8
            }
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Medium, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Medium)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Medium) => {
                ColorWheelSpeed::Medium as u8
            }
            ColorWheelConfig::Rgb(_, ColorWheelSpeed::Slow, _)
            | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Slow)
            | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Slow) => {
                ColorWheelSpeed::Slow as u8
            }
            _ => return (false, counter),
        };

        if counter == ch(speed_threshold) {
            (true, ch(1)) // Reset counter and update index
        } else {
            (false, counter + 1) // Increment counter, don't update index
        }
    }

    /// Update the color wheel index and handle direction changes.
    pub fn update_index_with_direction(
        index: &mut ChUnit,
        direction: &mut ColorWheelDirection,
        gradient_len: usize,
    ) -> Option<TuiColor> {
        match *direction {
            ColorWheelDirection::Forward => {
                *index += 1;

                // Hit the end of the gradient, reverse direction
                if *index == ch(gradient_len) {
                    *direction = ColorWheelDirection::Reverse;
                    *index -= 2;
                }
            }
            ColorWheelDirection::Reverse => {
                *index -= 1;

                // Hit the start of the gradient, forward direction
                if *index == ch(0) {
                    *direction = ColorWheelDirection::Forward;
                }
            }
        }
        None // Color will be retrieved separately
    }

    /// Get color at the current index from the gradient.
    pub fn get_color_at_index(gradient: &VecSteps, index: ChUnit) -> Option<TuiColor> {
        gradient.get(usize(index)).copied()
    }
}

/// Helper module for lolcat-specific operations.
mod lolcat_helper {
    use super::*;

    /// Handle lolcat color generation and seed advancement.
    pub fn generate_next_lolcat_color(lolcat: &mut Lolcat) -> TuiColor {
        let new_color = color_helpers::get_color_tuple(&lolcat.color_wheel_control);
        lolcat.color_wheel_control.seed +=
            Seed::from(lolcat.color_wheel_control.color_change_speed);
        tui_color!(new_color.0, new_color.1, new_color.2)
    }

    /// Convert lolcat seed to [`ChUnit`] for indexing.
    pub fn convert_lolcat_seed_to_index(seed: Seed) -> ChUnit {
        let seed_f64 = *seed * 1000.0;
        let converted_seed = if seed_f64.is_finite() && seed_f64 >= 0.0 {
            // Use the largest integer exactly representable by f64
            // (2^MANTISSA_DIGITS) to avoid precision loss when clamping.
            #[allow(clippy::cast_precision_loss)]
            const MAX_SAFE_F64_INT: f64 = (1u64 << f64::MANTISSA_DIGITS) as f64;
            let clamped_seed = seed_f64.min(MAX_SAFE_F64_INT).round();
            // Safe conversion - clamped_seed is guaranteed to be in valid range
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let seed_u64 = clamped_seed as u64;
            usize::try_from(seed_u64).unwrap_or(0)
        } else {
            0
        };
        ch(converted_seed)
    }
}

/// Helper module for gradient generation operations.
mod gradient_generation_helper {
    use super::*;

    /// Determine the number of steps for gradient generation.
    pub fn determine_steps(
        maybe_steps_override: Option<u8>,
        config: &ColorWheelConfig,
    ) -> u8 {
        match maybe_steps_override {
            // 1. Try use steps from `steps_override`.
            Some(steps_override) => steps_override,
            None => {
                // 2. Try use steps from `ColorWheelConfig`.
                if let ColorWheelConfig::Rgb(_, _, steps_from_config) = config {
                    *steps_from_config
                }
                // 3. Otherwise use the default.
                else {
                    Defaults::Steps as u8
                }
            }
        }
    }

    /// Generate gradient for Ansi256 color configuration.
    pub fn generate_ansi256_gradient(index: Ansi256GradientIndex) -> VecSteps {
        let gradient_array = get_gradient_array_for(index);
        let size_hint = gradient_array.len();
        let mut gradient_vec: VecSteps = VecSteps::with_capacity(size_hint);
        for color_u8 in gradient_array {
            gradient_vec.push(tui_color!(ansi * color_u8));
        }
        gradient_vec
    }

    /// Set the gradient and length based on the generated gradient.
    pub fn set_gradient_and_length(color_wheel: &mut ColorWheel, gradient: VecSteps) {
        color_wheel.gradient_length_kind = GradientLengthKind::ColorWheel(gradient.len());
        color_wheel.gradient_kind = GradientKind::ColorWheel(gradient);
        color_wheel.index = ch(0);
    }

    /// Set up lolcat gradient configuration.
    pub fn set_lolcat_gradient(color_wheel: &mut ColorWheel, builder: &LolcatBuilder) {
        color_wheel.gradient_kind = GradientKind::Lolcat(builder.build());
        color_wheel.index = ch(0);
        color_wheel.gradient_length_kind = GradientLengthKind::Lolcat(builder.seed);
    }
}

/// Helper module for lolcat background functionality.
mod lolcat_bg_helper {
    use super::*;

    /// Extract the style from the text colorization policy for lolcat background mode.
    pub fn extract_style_from_policy(
        text_colorization_policy: TextColorizationPolicy,
    ) -> Option<TuiStyle> {
        match text_colorization_policy {
            TextColorizationPolicy::ColorEachCharacter(maybe_style)
            | TextColorizationPolicy::ColorEachWord(maybe_style) => maybe_style,
        }
    }

    /// Convert a [`TuiColor`] to an RGB tuple for use as background color.
    pub fn convert_tui_color_to_rgb_tuple(color: TuiColor) -> Option<(u8, u8, u8)> {
        match color {
            TuiColor::Rgb(RgbValue {
                red: bg_red,
                green: bg_green,
                blue: bg_blue,
            }) => Some((bg_red, bg_green, bg_blue)),
            TuiColor::Ansi(ansi_value) => {
                let rgb_value = RgbValue::from(ansi_value);
                Some((rgb_value.red, rgb_value.green, rgb_value.blue))
            }
            TuiColor::Basic(basic_color) => {
                match RgbValue::try_from_tui_color(TuiColor::Basic(basic_color)) {
                    Ok(RgbValue { red, green, blue }) => Some((red, green, blue)),
                    Err(_) => None,
                }
            }
            TuiColor::Reset => None,
        }
    }
}

#[cfg(test)]
mod tests_color_wheel_rgb {
    use serial_test::serial;

    use super::*;
    use crate::{assert_eq2,
                global_color_support,
                tui_style::tui_style_attrib::{Bold, Dim},
                ColorSupport};

    mod test_helpers {
        use smallvec::smallvec;

        use super::*;

        pub fn create_color_wheel_rgb() -> ColorWheel {
            let config_1 = ColorWheelConfig::Rgb(
                smallvec::smallvec!["#000000".into(), "#ffffff".into()],
                ColorWheelSpeed::Fast,
                10,
            );
            ColorWheel::new(smallvec![config_1])
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_color_wheel_config_narrowing() {
        let default_color_wheel = ColorWheel::default();
        let configs = &default_color_wheel.configs;

        // Set ColorSupport override to: Ansi 256.
        {
            global_color_support::set_override(ColorSupport::Ansi256);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::MediumGreenToMediumBlue,
                    ColorWheelSpeed::Medium,
                ),
            );
            global_color_support::clear_override();
        }

        // Set ColorSupport override to: Truecolor.
        {
            global_color_support::set_override(ColorSupport::Truecolor);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Rgb(
                    get_default_gradient_stops(),
                    ColorWheelSpeed::Medium,
                    Defaults::Steps as u8,
                ),
            );
            global_color_support::clear_override();
        }

        // Set ColorSupport override to: Grayscale.
        {
            global_color_support::set_override(ColorSupport::Grayscale);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Medium,
                ),
            );
            global_color_support::clear_override();
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_color_wheel_iterator() {
        global_color_support::set_override(ColorSupport::Truecolor);

        let color_wheel = &mut test_helpers::create_color_wheel_rgb();

        // Didn't call generate_color_wheel() yet, so it should return the start color.
        assert!(color_wheel.next_color().is_none());

        // Call generate_color_wheel() with 10 steps.
        let gradient_kind = color_wheel.generate_color_wheel(None);
        let GradientKind::ColorWheel(lhs) = gradient_kind else {
            panic!()
        };
        let rhs = &[
            (0, 0, 0),
            (26, 26, 26),
            (51, 51, 51),
            (77, 77, 77),
            (102, 102, 102),
            (128, 128, 128),
            (153, 153, 153),
            (179, 179, 179),
            (204, 204, 204),
            (230, 230, 230),
        ]
        .iter()
        .map(|(r, g, b)| tui_color!(*r, *g, *b))
        .collect::<VecSteps>();
        assert_eq2!(lhs, rhs);

        // Call to next() should return the start_color.
        assert_eq2!(
            // 1st call to next(), index is 0
            color_wheel.next_color().unwrap(),
            tui_color!(0, 0, 0)
        );
        assert_eq2!(
            // 2nd call to next(), index is 0
            color_wheel.next_color().unwrap(),
            tui_color!(0, 0, 0)
        );
        assert_eq2!(
            // 3rd call to next(), index is 1
            color_wheel.next_color().unwrap(),
            tui_color!(26, 26, 26)
        );
        assert_eq2!(
            // # 4th call to next(), index is 1
            color_wheel.next_color().unwrap(),
            tui_color!(26, 26, 26)
        );
        assert_eq2!(
            // # 5th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            tui_color!(51, 51, 51)
        );
        assert_eq2!(
            // # 6th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            tui_color!(51, 51, 51)
        );

        // Advance color wheel to index = 8.
        for _ in 0..13 {
            color_wheel.next_color();
        }

        // Next call to next() which is the 20th call should return the end_color.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(230, 230, 230));

        // Next call to next() should return the end_color - 1.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(204, 204, 204));

        // Reverse color wheel to index = 0.
        for _ in 0..16 {
            color_wheel.next_color();
        }

        // Next call to next() should return the start_color.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(0, 0, 0));

        // Next call to next() should advance the index again to 1.
        assert_eq2!(color_wheel.next_color().unwrap(), tui_color!(26, 26, 26));

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_styled_texts_color_each_word() {
        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let string = "HELLO WORLD";
        let string_gcs = string.grapheme_string();
        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &string_gcs,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachWord(None),
        );
        assert_eq2!(styled_texts.len(), 3);

        // [0]: "HELLO", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text(), "HELLO");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );
        assert!(styled_texts[0].get_style().dim.is_none());
        assert!(styled_texts[0].get_style().bold.is_none());

        // [1]: " ", color_fg: None
        assert_eq2!(styled_texts[1].get_text(), " ");
        assert_eq2!(styled_texts[1].get_style().color_fg, None);

        // [2]: "WORLD", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[2].get_text(), "WORLD");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_to_styled_texts_color_each_character() {
        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let style = TuiStyle {
            dim: Some(Dim),
            bold: Some(Bold),
            ..Default::default()
        };

        let string = "HELLO";
        let string_gcs = string.grapheme_string();
        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &string_gcs,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(Some(style)),
        );
        assert_eq2!(styled_texts.len(), 5);

        // [0]: "H", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text(), "H");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );
        assert!(styled_texts[0].get_style().dim.is_some());
        assert!(styled_texts[0].get_style().bold.is_some());

        // [1]: "E", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[1].get_text(), "E");
        assert_eq2!(
            styled_texts[1].get_style().color_fg,
            Some(tui_color!(0, 0, 0))
        );

        // [2]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[2].get_text(), "L");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(tui_color!(51, 51, 51))
        );

        // [3]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[3].get_text(), "L");
        assert_eq2!(
            styled_texts[3].get_style().color_fg,
            Some(tui_color!(51, 51, 51))
        );

        // [4]: "O", color_fg: Rgb(102,102,102)
        assert_eq2!(styled_texts[4].get_text(), "O");
        assert_eq2!(
            styled_texts[4].get_style().color_fg,
            Some(tui_color!(102, 102, 102))
        );

        global_color_support::clear_override();
    }

    /// This strange test is needed because the color wheel uses a global variable to
    /// determine color support. This test ensures that the global variable is reset
    /// to its original value after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to
    /// ensure that the global variable is reset to its original value before each
    /// test. This is why `test_color_wheel_config_narrowing`,
    /// `test_color_wheel_iterator`, etc. are wrapped in a single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_ansi_styled_string_each_character() {
        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let string = "HELLO WORLD";

        let ansi_styled_string = color_wheel_rgb.colorize_into_string(
            string,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(None),
            None,
        );

        println!("ansi_styled_string: {ansi_styled_string}");
        println!("ansi_styled_string: {ansi_styled_string:?}");

        assert_eq2!(
            ansi_styled_string,
            "\u{1b}[38;2;0;0;0mH\u{1b}[0m\u{1b}[38;2;0;0;0mE\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;46;46;46mO\u{1b}[0m\u{1b}[38;2;46;46;46m \u{1b}[0m\u{1b}[38;2;70;70;70mW\u{1b}[0m\u{1b}[38;2;70;70;70mO\u{1b}[0m\u{1b}[38;2;93;93;93mR\u{1b}[0m\u{1b}[38;2;93;93;93mL\u{1b}[0m\u{1b}[38;2;116;116;116mD\u{1b}[0m"
        );

        global_color_support::clear_override();
    }
}
