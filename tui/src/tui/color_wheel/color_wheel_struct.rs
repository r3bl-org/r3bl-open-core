/*
 *   Copyright (c) 2023 R3BL LLC
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
use r3bl_ansi_color::{global_color_support, AnsiStyledText, ColorSupport};
use r3bl_rs_utils_core::{ch,
                         AnsiValue,
                         ChUnit,
                         GraphemeClusterSegment,
                         RgbValue,
                         TuiColor,
                         TuiStyle,
                         UnicodeString};
use serde::{Deserialize, Serialize};

use crate::{color_wheel_color_converter::convert_tui_color_into_r3bl_ansi_color,
            generate_random_truecolor_gradient,
            generate_truecolor_gradient,
            get_gradient_array_for,
            tui_styled_text,
            Ansi256GradientIndex,
            ColorUtils,
            Lolcat,
            LolcatBuilder,
            TuiStyledText,
            TuiStyledTexts,
            SPACER};

/// For RGB colors:
/// 1. The stops are the colors that will be used to create the gradient.
/// 2. The speed is how fast the color wheel will rotate.
/// 3. The steps are the number of colors that will be generated. The larger this number is the
///    smoother the transition will be between each color. 100 is a good number to start with.
#[derive(Serialize, Deserialize, Clone, PartialEq, GetSize, Debug)]
pub enum ColorWheelConfig {
    Rgb(
        /* stops */ Vec<String>,
        /* speed */ ColorWheelSpeed,
        /* steps */ usize,
    ),
    RgbRandom(/* speed */ ColorWheelSpeed),
    Ansi256(Ansi256GradientIndex, ColorWheelSpeed),
    Lolcat(LolcatBuilder),
}

impl ColorWheelConfig {
    pub fn config_contains_bg_lolcat(configs: &[ColorWheelConfig]) -> bool {
        for config in configs {
            if let ColorWheelConfig::Lolcat(LolcatBuilder {
                background_mode: true,
                ..
            }) = config
            {
                return true;
            }
        }
        false
    }

    // Narrow down the given configs into a single one based on color_support (and global override)
    pub fn narrow_config_based_on_color_support(
        configs: &[ColorWheelConfig],
    ) -> ColorWheelConfig {
        let color_support = global_color_support::detect();
        match color_support {
            // 1. If truecolor is supported, try and find a truecolor config.
            // 2. If not found, then look for an ANSI 256 config.
            // 3. If not found, then return a grayscale config.
            ColorSupport::Truecolor => {
                // All configs that will work w/ truecolor.
                let maybe_truecolor_config = configs.iter().find(|it| {
                    matches!(
                        it,
                        ColorWheelConfig::Lolcat(_)
                            | ColorWheelConfig::RgbRandom(_)
                            | ColorWheelConfig::Rgb(_, _, _)
                    )
                });
                if let Some(config) = maybe_truecolor_config {
                    return config.clone();
                }

                let maybe_ansi_256_config = configs
                    .iter()
                    .find(|it| matches!(it, ColorWheelConfig::Ansi256(_, _)));
                if let Some(config) = maybe_ansi_256_config {
                    return config.clone();
                }

                // Grayscale fallback.
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Medium,
                )
            }
            // 1. If ANSI 256 is supported, try and find an ANSI 256 config.
            // 2. If not found, then return a grayscale config.
            ColorSupport::Ansi256 => {
                let maybe_ansi_256_config = configs
                    .iter()
                    .find(|it| matches!(it, ColorWheelConfig::Ansi256(_, _)));
                if let Some(config) = maybe_ansi_256_config {
                    return config.clone();
                }

                // Grayscale fallback.
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                    ColorWheelSpeed::Medium,
                )
            }
            // Grayscale fallback.
            _ => ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::GrayscaleMediumGrayToWhite,
                ColorWheelSpeed::Medium,
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Debug)]
pub enum ColorWheelDirection {
    Forward,
    Reverse,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Debug)]
pub enum ColorWheelSpeed {
    Slow = 10,
    Medium = 5,
    Fast = 2,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum GradientKind {
    ColorWheel(Vec<TuiColor>),
    Lolcat(Lolcat),
    NotCalculatedYet,
}

/// Gradient has to be generated before this will be anything other than
/// [GradientLengthKind::NotCalculatedYet].
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub enum GradientLengthKind {
    ColorWheel(usize),
    Lolcat(/* seed */ f64),
    NotCalculatedYet,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct ColorWheel {
    pub configs: Vec<ColorWheelConfig>,
    pub gradient_kind: GradientKind,
    pub gradient_length_kind: GradientLengthKind,
    pub index: ChUnit,
    pub index_direction: ColorWheelDirection,
    pub counter: ChUnit,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Debug)]
pub enum Defaults {
    Steps = 50,
}

/// More info: <https://www.colorhexa.com/>
pub const DEFAULT_GRADIENT_STOPS: [&str; 3] = [
    /* cyan */ "#00ffff", /* magenta */ "#ff00ff", /* blue */ "#0000ff",
];

impl Default for ColorWheel {
    fn default() -> Self {
        Self::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(DEFAULT_GRADIENT_STOPS.map(String::from)),
                ColorWheelSpeed::Medium,
                Defaults::Steps as usize,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::MediumGreenToMediumBlue,
                ColorWheelSpeed::Medium,
            ),
        ])
    }
}

impl ColorWheel {
    /// This will lazily create a color wheel. It does not compute the gradient and memoize it when
    /// this function is called.
    ///
    /// 1. The heavy lifting is done when [generate_color_wheel](ColorWheel::generate_color_wheel)
    ///    is called.
    /// 2. When you use [colorize_into_styled_texts](ColorWheel::colorize_into_styled_texts) it will
    ///    also also call this method.
    ///
    /// # Arguments
    /// 1. `configs`: A list of color wheel configs. The order of the configs is not
    ///    important. However, at the very least, one Truecolor config & one ANSI 256
    ///    config should be provided. The fallback is always grayscale. See
    ///    [ColorWheelConfig::narrow_config_based_on_color_support],
    ///    [global_color_support::detect] for more info.
    pub fn new(configs: Vec<ColorWheelConfig>) -> Self {
        Self {
            configs,
            gradient_kind: GradientKind::NotCalculatedYet,
            gradient_length_kind: GradientLengthKind::NotCalculatedYet,
            index: ch!(0),
            index_direction: ColorWheelDirection::Forward,
            counter: ch!(0),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Debug)]
pub enum GradientGenerationPolicy {
    RegenerateGradientAndIndexBasedOnTextLength,
    ReuseExistingGradientAndIndex,
    ReuseExistingGradientAndResetIndex,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Debug)]
pub enum TextColorizationPolicy {
    ColorEachCharacter(Option<TuiStyle>),
    ColorEachWord(Option<TuiStyle>),
}

impl ColorWheel {
    /// This method will return the index of the current color in the gradient.
    pub fn get_index(&self) -> ChUnit {
        match self.gradient_kind {
            GradientKind::ColorWheel(_) => self.index,
            GradientKind::Lolcat(lolcat) => {
                let seed = (lolcat.color_wheel_control.seed * 1000.0) as usize;
                ch!(seed)
            }
            GradientKind::NotCalculatedYet => ch!(0),
        }
    }

    /// This method will return the length of the gradient. This is
    /// [GradientLengthKind::NotCalculatedYet] if the gradient has not been computed & memoized yet
    /// via a call to [`generate_color_wheel`](ColorWheel::generate_color_wheel).
    pub fn get_gradient_len(&self) -> GradientLengthKind { self.gradient_length_kind }

    pub fn get_gradient_kind(&self) -> &GradientKind { &self.gradient_kind }

    /// Every time this method is called, it will generate the gradient & memoize it.
    ///
    /// # Arguments
    /// * `steps_override` - If `Some` then the number of steps will be overridden. If `None` then
    ///                      the number of steps will be determined by the `ColorWheelConfig`.
    ///
    /// Here's the priority order of how `steps` is determined:
    /// 1. If `steps_override` is `Some` then use that.
    /// 2. If `steps_override` is `None` then use the steps from the `ColorWheelConfig`.
    /// 3. If nothing is found in `ColorWheelConfig` then use `DEFAULT_STEPS`.
    ///
    /// ## Errors
    /// If the RGB color is invalid, then this method will panic.
    pub fn generate_color_wheel(
        &mut self,
        maybe_steps_override: Option<usize>,
    ) -> &GradientKind {
        let my_config =
            ColorWheelConfig::narrow_config_based_on_color_support(&self.configs);

        let steps = match maybe_steps_override {
            // 1. Try use steps from `steps_override`.
            Some(steps_override) => steps_override,
            None => {
                // 2. Try use steps from `ColorWheelConfig`.
                if let ColorWheelConfig::Rgb(_, _, steps_from_config) = my_config {
                    steps_from_config
                }
                // 3. Otherwise use the default.
                else {
                    Defaults::Steps as usize
                }
            }
        };

        // Generate new gradient and replace the old one.
        // More info: https://github.com/Ogeon/palette/tree/master/palette#gradients
        match &my_config {
            ColorWheelConfig::Lolcat(builder) => {
                self.gradient_kind = GradientKind::Lolcat(builder.build());
                self.index = ch!(0);
                self.gradient_length_kind = GradientLengthKind::Lolcat(builder.seed);
            }

            ColorWheelConfig::Rgb(stops, _, _) => {
                // Generate new gradient.
                let new_gradient = generate_truecolor_gradient(stops, steps);
                self.gradient_length_kind =
                    GradientLengthKind::ColorWheel(new_gradient.len());
                self.gradient_kind = GradientKind::ColorWheel(new_gradient);
                self.index = ch!(0);
            }

            ColorWheelConfig::RgbRandom(_) => {
                // Generate new random gradient.
                let new_gradient = generate_random_truecolor_gradient(steps);
                self.gradient_length_kind =
                    GradientLengthKind::ColorWheel(new_gradient.len());
                self.gradient_kind = GradientKind::ColorWheel(new_gradient);
                self.index = ch!(0);
            }

            ColorWheelConfig::Ansi256(index, _) => {
                let gradient: Vec<TuiColor> = get_gradient_array_for(*index)
                    .iter()
                    .map(|color_u8| TuiColor::Ansi(AnsiValue::new(*color_u8)))
                    .collect();
                self.gradient_length_kind =
                    GradientLengthKind::ColorWheel(gradient.len());
                self.gradient_kind = GradientKind::ColorWheel(gradient);
                self.index = ch!(0);
            }
        }

        &self.gradient_kind
    }

    /// This method will return the next color in the gradient. It updates the index. When it
    /// reaches the end of the gradient, it will flip direction and go in reverse. And then flip
    /// again when it reaches the start. And so on.
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
                let new_color = ColorUtils::get_color_tuple(&lolcat.color_wheel_control);
                lolcat.color_wheel_control.seed +=
                    f64::from(lolcat.color_wheel_control.color_change_speed);
                Some(TuiColor::Rgb(RgbValue::from_u8(
                    new_color.0,
                    new_color.1,
                    new_color.2,
                )))
            } else {
                None
            };
        }

        // Determine if the index should be changed (depending on the speed).
        let should_change_index: bool = {
            let mut it = false;
            match my_config {
                ColorWheelConfig::Rgb(_, ColorWheelSpeed::Fast, _)
                | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Fast)
                | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Fast) => {
                    if self.counter == ch!(ColorWheelSpeed::Fast as u8) {
                        // Reset counter & change index below.
                        self.counter = ch!(1);
                        it = true;
                    } else {
                        // Increment counter, used for speed control.
                        self.counter += 1;
                    }
                }

                ColorWheelConfig::Rgb(_, ColorWheelSpeed::Medium, _)
                | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Medium)
                | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Medium) => {
                    if self.counter == ch!(ColorWheelSpeed::Medium as u8) {
                        // Reset counter & change index below.
                        self.counter = ch!(1);
                        it = true;
                    } else {
                        // Increment counter, used for speed control.
                        self.counter += 1;
                    }
                }

                ColorWheelConfig::Rgb(_, ColorWheelSpeed::Slow, _)
                | ColorWheelConfig::RgbRandom(ColorWheelSpeed::Slow)
                | ColorWheelConfig::Ansi256(_, ColorWheelSpeed::Slow) => {
                    if self.counter == ch!(ColorWheelSpeed::Slow as u8) {
                        // Reset counter & change index below.
                        self.counter = ch!(1);
                        it = true;
                    } else {
                        // Increment counter, used for speed control.
                        self.counter += 1;
                    }
                }

                _ => {}
            };
            it
        };

        let GradientKind::ColorWheel(gradient) = &mut self.gradient_kind else {
            return None;
        };

        // Actually change the index if it should be changed.
        if should_change_index {
            return match self.index_direction {
                ColorWheelDirection::Forward => {
                    self.index += 1;

                    // Hit the end of the gradient, so reverse the direction.
                    if self.index == ch!(gradient.len())
                        && self.index_direction == ColorWheelDirection::Forward
                    {
                        self.index_direction = ColorWheelDirection::Reverse;
                        self.index -= 2;
                    }

                    // Return the color for the correct index.
                    let color = gradient.get(ch!(@to_usize self.index))?;
                    Some(*color)
                }
                ColorWheelDirection::Reverse => {
                    self.index -= 1;

                    // Hit the start of the gradient, so reverse the direction.
                    if self.index == ch!(0)
                        && self.index_direction == ColorWheelDirection::Reverse
                    {
                        self.index_direction = ColorWheelDirection::Forward;
                    }

                    // Return the color for the correct index.
                    let color = gradient.get(ch!(@to_usize self.index))?;
                    Some(*color)
                }
            };
        }

        // Return the color for the correct index.
        let color = gradient.get(ch!(@to_usize self.index))?;
        Some(*color)
    }

    /// This method will reset the index to zero.
    fn reset_index(&mut self) {
        // If this is a lolcat, reset the seed, and early return.
        if let GradientLengthKind::Lolcat(seed) = self.get_gradient_len() {
            if let GradientKind::Lolcat(mut lolcat) = self.get_gradient_kind() {
                lolcat.color_wheel_control.seed = seed;
                return;
            }
        }

        // Not a lolcat so reset the index and direction.
        self.index = ch!(0);
        self.index_direction = ColorWheelDirection::Forward;
    }

    /// Simplified version of [ColorWheel::colorize_into_string] with some defaults.
    pub fn lolcat_into_string(text: &str) -> String {
        ColorWheel::default().colorize_into_string(
            &UnicodeString::from(text),
            GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
            TextColorizationPolicy::ColorEachCharacter(None),
        )
    }

    /// See [ColorWheel::lolcat_into_string] for an easy to use version of this function.
    pub fn colorize_into_string(
        &mut self,
        unicode_string: &UnicodeString,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
    ) -> String {
        let it = self.colorize_into_styled_texts(
            unicode_string,
            gradient_generation_policy,
            text_colorization_policy,
        );

        let mut acc_vec = vec![];

        for TuiStyledText(style, unicode_string) in it.items {
            let maybe_src_color_fg = style.color_fg;
            let maybe_src_color_bg = style.color_bg;

            let mut acc_style = vec![];

            if let Some(src_color_fg) = maybe_src_color_fg {
                acc_style.push(r3bl_ansi_color::Style::Foreground(
                    convert_tui_color_into_r3bl_ansi_color(src_color_fg),
                ));
            }

            if let Some(src_color_bg) = maybe_src_color_bg {
                acc_style.push(r3bl_ansi_color::Style::Background(
                    convert_tui_color_into_r3bl_ansi_color(src_color_bg),
                ));
            }

            let ansi_styled_text = AnsiStyledText {
                style: &acc_style,
                text: &(unicode_string.string),
            };

            let output = format!("{}", ansi_styled_text);
            acc_vec.push(output);
        }

        acc_vec.join("")
    }

    /// This method gives you fine grained control over the color wheel. It returns a
    /// gradient-colored string. It respects the [ColorSupport] restrictions for the terminal.
    ///
    /// ## Colorization Policy
    /// - [GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength]: The first time
    ///   this method is called it will generate a gradient w/ the number of steps. Subsequent calls
    ///   will use the same gradient and index **if** the number of steps is the same. However, if
    ///   the number of steps are different, then a new gradient will be generated & the index
    ///   reset.
    /// - [GradientGenerationPolicy::ReuseExistingGradientAndIndex]: The first time this method is
    ///   called it will generate a gradient w/ the number of steps. Subsequent calls will use the
    ///   same gradient and index.
    pub fn colorize_into_styled_texts(
        &mut self,
        text: &UnicodeString,
        gradient_generation_policy: GradientGenerationPolicy,
        text_colorization_policy: TextColorizationPolicy,
    ) -> TuiStyledTexts {
        self.generate_gradient(text, gradient_generation_policy);
        self.generate_styled_texts(text_colorization_policy, text)
    }

    fn generate_styled_texts(
        &mut self,
        text_colorization_policy: TextColorizationPolicy,
        text: &UnicodeString,
    ) -> TuiStyledTexts {
        mod inner {
            use r3bl_rs_utils_core::{TuiColor, TuiStyle};

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
                next_bg_color: Option<TuiColor>,
            ) -> TuiStyle {
                let mut it = TuiStyle {
                    color_fg: next_fg_color,
                    color_bg: next_bg_color,
                    ..Default::default()
                };
                it += &maybe_style;
                it
            }
        }

        let mut acc = TuiStyledTexts::default();

        // Handle special case for lolcat background mode is true.
        if ColorWheelConfig::config_contains_bg_lolcat(&self.configs) {
            let maybe_style = match text_colorization_policy {
                TextColorizationPolicy::ColorEachCharacter(maybe_style) => maybe_style,
                TextColorizationPolicy::ColorEachWord(maybe_style) => maybe_style,
            };

            // Loop: Colorize each (next) character w/ (next) color.
            for GraphemeClusterSegment {
                string: next_character,
                ..
            } in text.iter()
            {
                let maybe_next_bg_color = self.next_color();

                if let Some(next_bg_color) = maybe_next_bg_color {
                    let maybe_bg_color = match next_bg_color {
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
                            match RgbValue::try_from_tui_color(TuiColor::Basic(
                                basic_color,
                            )) {
                                Ok(RgbValue { red, green, blue }) => {
                                    Some((red, green, blue))
                                }
                                Err(_) => None,
                            }
                        }
                        TuiColor::Reset => None,
                    };

                    if let Some((bg_red, bg_green, bg_blue)) = maybe_bg_color {
                        let (fg_red, fg_green, fg_blue) =
                            ColorUtils::calc_fg_color((bg_red, bg_green, bg_blue));
                        acc += tui_styled_text!(
                            @style: inner::gen_style_fg_bg_color_for(
                                maybe_style,
                                Some(TuiColor::Rgb(RgbValue::from_u8(fg_red, fg_green, fg_blue))),
                                Some(TuiColor::Rgb(RgbValue::from_u8(bg_red, bg_green, bg_blue))),
                            ),
                            @text: next_character,
                        );
                    } else {
                        acc += tui_styled_text!(
                            @style: inner::gen_style_fg_bg_color_for(maybe_style, None, None,),
                            @text: next_character,
                        );
                    }
                } else {
                    acc += tui_styled_text!(
                        @style: inner::gen_style_fg_bg_color_for(maybe_style, None, None,),
                        @text: next_character,
                    );
                }
            }
            return acc;
        }

        // Handle regular case.
        match text_colorization_policy {
            TextColorizationPolicy::ColorEachCharacter(maybe_style) => {
                for GraphemeClusterSegment {
                    string: next_character,
                    ..
                } in text.iter()
                {
                    // Loop: Colorize each (next) character w/ (next) color.
                    acc += tui_styled_text!(
                        @style: inner::gen_style_fg_color_for(maybe_style, self.next_color()),
                        @text: next_character,
                    );
                }
            }
            TextColorizationPolicy::ColorEachWord(maybe_style) => {
                // More info on peekable: https://stackoverflow.com/a/67872822/2085356
                let mut peekable = text.string.split_ascii_whitespace().peekable();
                while let Some(next_word) = peekable.next() {
                    // Loop: Colorize each (next) word w/ (next) color.
                    acc += tui_styled_text!(
                        @style: inner::gen_style_fg_color_for(maybe_style, self.next_color()),
                        @text: next_word,
                    );
                    if peekable.peek().is_some() {
                        acc += tui_styled_text!(
                            @style: TuiStyle::default(),
                            @text: SPACER,
                        );
                    }
                }
            }
        }

        acc
    }

    fn generate_gradient(
        &mut self,
        text: &UnicodeString,
        gradient_generation_policy: GradientGenerationPolicy,
    ) {
        match gradient_generation_policy {
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength => {
                let steps = text.len();

                // Generate a new gradient if one doesn't exist.
                if let GradientLengthKind::NotCalculatedYet = self.get_gradient_len() {
                    self.generate_color_wheel(Some(steps));
                    return;
                }

                // Re-use gradient if possible.
                if let GradientLengthKind::ColorWheel(length) = self.get_gradient_len() {
                    if length != steps {
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
        };
    }
}

pub mod color_wheel_color_converter {
    use r3bl_rs_utils_core::{ANSIBasicColor, RgbValue, TuiColor};

    pub fn convert_tui_color_into_r3bl_ansi_color(
        color: TuiColor,
    ) -> r3bl_ansi_color::Color {
        match color {
            TuiColor::Rgb(RgbValue { red, green, blue }) => {
                r3bl_ansi_color::Color::Rgb(red, green, blue)
            }
            TuiColor::Ansi(ansi_value) => {
                r3bl_ansi_color::Color::Ansi256(ansi_value.color)
            }
            TuiColor::Basic(basic_color) => match basic_color {
                ANSIBasicColor::Black => r3bl_ansi_color::Color::Rgb(0, 0, 0),
                ANSIBasicColor::White => r3bl_ansi_color::Color::Rgb(255, 255, 255),
                ANSIBasicColor::Grey => r3bl_ansi_color::Color::Rgb(128, 128, 128),
                ANSIBasicColor::DarkGrey => r3bl_ansi_color::Color::Rgb(64, 64, 64),
                ANSIBasicColor::Red => r3bl_ansi_color::Color::Rgb(255, 0, 0),
                ANSIBasicColor::DarkRed => r3bl_ansi_color::Color::Rgb(128, 0, 0),
                ANSIBasicColor::Green => r3bl_ansi_color::Color::Rgb(0, 255, 0),
                ANSIBasicColor::DarkGreen => r3bl_ansi_color::Color::Rgb(0, 128, 0),
                ANSIBasicColor::Yellow => r3bl_ansi_color::Color::Rgb(255, 255, 0),
                ANSIBasicColor::DarkYellow => r3bl_ansi_color::Color::Rgb(128, 128, 0),
                ANSIBasicColor::Blue => r3bl_ansi_color::Color::Rgb(0, 0, 255),
                ANSIBasicColor::DarkBlue => r3bl_ansi_color::Color::Rgb(0, 0, 128),
                ANSIBasicColor::Magenta => r3bl_ansi_color::Color::Rgb(255, 0, 255),
                ANSIBasicColor::DarkMagenta => r3bl_ansi_color::Color::Rgb(128, 0, 128),
                ANSIBasicColor::Cyan => r3bl_ansi_color::Color::Rgb(0, 255, 255),
                ANSIBasicColor::DarkCyan => r3bl_ansi_color::Color::Rgb(0, 128, 128),
            },
            TuiColor::Reset => r3bl_ansi_color::Color::Rgb(0, 0, 0),
        }
    }

    #[cfg(test)]
    mod color_converter_tests {
        use r3bl_rs_utils_core::{ANSIBasicColor, AnsiValue, RgbValue, TuiColor};

        use crate::color_wheel_struct::color_wheel_color_converter;

        #[test]
        fn test_convert_tui_color_into_r3bl_ansi_color_rgb() {
            let tui_color = TuiColor::Rgb(RgbValue {
                red: 255,
                green: 0,
                blue: 0,
            });
            let expected_color = r3bl_ansi_color::Color::Rgb(255, 0, 0);
            let converted_color =
                color_wheel_color_converter::convert_tui_color_into_r3bl_ansi_color(
                    tui_color,
                );
            assert_eq!(converted_color, expected_color);
        }

        #[test]
        fn test_convert_tui_color_into_r3bl_ansi_color_ansi() {
            let tui_color = TuiColor::Ansi(AnsiValue { color: 42 });
            let expected_color = r3bl_ansi_color::Color::Ansi256(42);
            let converted_color =
                color_wheel_color_converter::convert_tui_color_into_r3bl_ansi_color(
                    tui_color,
                );
            assert_eq!(converted_color, expected_color);
        }

        #[test]
        fn test_convert_tui_color_into_r3bl_ansi_color_basic() {
            let tui_color = TuiColor::Basic(ANSIBasicColor::Red);
            let expected_color = r3bl_ansi_color::Color::Rgb(255, 0, 0);
            let converted_color =
                color_wheel_color_converter::convert_tui_color_into_r3bl_ansi_color(
                    tui_color,
                );
            assert_eq!(converted_color, expected_color);
        }

        #[test]
        fn test_convert_tui_color_into_r3bl_ansi_color_reset() {
            let tui_color = TuiColor::Reset;
            let expected_color = r3bl_ansi_color::Color::Rgb(0, 0, 0);
            let converted_color =
                color_wheel_color_converter::convert_tui_color_into_r3bl_ansi_color(
                    tui_color,
                );
            assert_eq!(converted_color, expected_color);
        }
    }
}

#[cfg(test)]
mod tests_color_wheel_rgb {
    use r3bl_rs_utils_core::assert_eq2;
    use serial_test::serial;

    use super::*;

    mod test_helpers {
        use super::*;

        pub fn create_color_wheel_rgb() -> ColorWheel {
            ColorWheel::new(vec![ColorWheelConfig::Rgb(
                vec!["#000000".into(), "#ffffff".into()],
                ColorWheelSpeed::Fast,
                10,
            )])
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to determine
    /// color support. This test ensures that the global variable is reset to its original value
    /// after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to ensure that
    /// the global variable is reset to its original value before each test. This is why
    /// `test_color_wheel_config_narrowing`, `test_color_wheel_iterator`, etc. are wrapped in a
    /// single test.
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
            global_color_support::clear_override()
        }

        // Set ColorSupport override to: Truecolor.
        {
            global_color_support::set_override(ColorSupport::Truecolor);
            let config = ColorWheelConfig::narrow_config_based_on_color_support(configs);
            assert_eq2!(
                config,
                ColorWheelConfig::Rgb(
                    Vec::from(DEFAULT_GRADIENT_STOPS.map(String::from)),
                    ColorWheelSpeed::Medium,
                    Defaults::Steps as usize,
                ),
            );
            global_color_support::clear_override()
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
            global_color_support::clear_override()
        }
    }

    /// This strange test is needed because the color wheel uses a global variable to determine
    /// color support. This test ensures that the global variable is reset to its original value
    /// after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to ensure that
    /// the global variable is reset to its original value before each test. This is why
    /// `test_color_wheel_config_narrowing`, `test_color_wheel_iterator`, etc. are wrapped in a
    /// single test.
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
        .map(|(r, g, b)| TuiColor::Rgb(RgbValue::from_u8(*r, *g, *b)))
        .collect::<Vec<_>>();
        assert_eq2!(lhs, rhs);

        // Call to next() should return the start_color.
        assert_eq2!(
            // 1st call to next(), index is 0
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(0, 0, 0))
        );
        assert_eq2!(
            // 2nd call to next(), index is 0
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(0, 0, 0))
        );
        assert_eq2!(
            // 3rd call to next(), index is 1
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(26, 26, 26))
        );
        assert_eq2!(
            // # 4th call to next(), index is 1
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(26, 26, 26))
        );
        assert_eq2!(
            // # 5th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(51, 51, 51))
        );
        assert_eq2!(
            // # 6th call to next(), index is 2
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(51, 51, 51))
        );

        // Advance color wheel to index = 8.
        for _ in 0..13 {
            color_wheel.next_color();
        }

        // Next call to next() which is the 20th call should return the end_color.
        assert_eq2!(
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(230, 230, 230))
        );

        // Next call to next() should return the end_color - 1.
        assert_eq2!(
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(204, 204, 204))
        );

        // Reverse color wheel to index = 0.
        for _ in 0..16 {
            color_wheel.next_color();
        }

        // Next call to next() should return the start_color.
        assert_eq2!(
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(0, 0, 0))
        );

        // Next call to next() should advance the index again to 1.
        assert_eq2!(
            color_wheel.next_color().unwrap(),
            TuiColor::Rgb(RgbValue::from_u8(26, 26, 26))
        );

        global_color_support::clear_override()
    }

    /// This strange test is needed because the color wheel uses a global variable to determine
    /// color support. This test ensures that the global variable is reset to its original value
    /// after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to ensure that
    /// the global variable is reset to its original value before each test. This is why
    /// `test_color_wheel_config_narrowing`, `test_color_wheel_iterator`, etc. are wrapped in a
    /// single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_styled_texts_color_each_word() {
        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let unicode_string = UnicodeString::from("HELLO WORLD");

        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &unicode_string,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachWord(None),
        );
        assert_eq2!(styled_texts.len(), 3);

        // [0]: "HELLO", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text().string, "HELLO");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(0, 0, 0)))
        );
        assert_eq2!(styled_texts[0].get_style().dim, false);
        assert_eq2!(styled_texts[0].get_style().bold, false);

        // [1]: " ", color_fg: None
        assert_eq2!(styled_texts[1].get_text().string, " ");
        assert_eq2!(styled_texts[1].get_style().color_fg, None);

        // [2]: "WORLD", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[2].get_text().string, "WORLD");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(0, 0, 0)))
        );

        global_color_support::clear_override()
    }

    /// This strange test is needed because the color wheel uses a global variable to determine
    /// color support. This test ensures that the global variable is reset to its original value
    /// after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to ensure that
    /// the global variable is reset to its original value before each test. This is why
    /// `test_color_wheel_config_narrowing`, `test_color_wheel_iterator`, etc. are wrapped in a
    /// single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_to_styled_texts_color_each_character() {
        use r3bl_rs_utils_macro::tui_style;

        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let unicode_string = UnicodeString::from("HELLO");

        let style = tui_style!(attrib: [bold, dim]);

        let styled_texts = color_wheel_rgb.colorize_into_styled_texts(
            &unicode_string,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(Some(style)),
        );
        assert_eq2!(styled_texts.len(), 5);

        // [0]: "H", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[0].get_text().string, "H");
        assert_eq2!(
            styled_texts[0].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(0, 0, 0)))
        );
        assert_eq2!(styled_texts[0].get_style().dim, true);
        assert_eq2!(styled_texts[0].get_style().bold, true);

        // [1]: "E", color_fg: Rgb(0, 0, 0)
        assert_eq2!(styled_texts[1].get_text().string, "E");
        assert_eq2!(
            styled_texts[1].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(0, 0, 0)))
        );

        // [2]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[2].get_text().string, "L");
        assert_eq2!(
            styled_texts[2].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(51, 51, 51)))
        );

        // [3]: "L", color_fg: Rgb(51, 51, 51)
        assert_eq2!(styled_texts[3].get_text().string, "L");
        assert_eq2!(
            styled_texts[3].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(51, 51, 51)))
        );

        // [4]: "O", color_fg: Rgb(102,102,102)
        assert_eq2!(styled_texts[4].get_text().string, "O");
        assert_eq2!(
            styled_texts[4].get_style().color_fg,
            Some(TuiColor::Rgb(RgbValue::from_u8(102, 102, 102)))
        );

        global_color_support::clear_override()
    }

    /// This strange test is needed because the color wheel uses a global variable to determine
    /// color support. This test ensures that the global variable is reset to its original value
    /// after each test.
    ///
    /// Additionally, since Rust runs tests in a multi-threaded environment, we need to ensure that
    /// the global variable is reset to its original value before each test. This is why
    /// `test_color_wheel_config_narrowing`, `test_color_wheel_iterator`, etc. are wrapped in a
    /// single test.
    ///
    /// If these two are left as separate tests, then these tests will be flaky.
    #[serial]
    #[test]
    fn test_colorize_into_ansi_styled_string_each_character() {
        let color_wheel_rgb = &mut test_helpers::create_color_wheel_rgb();

        global_color_support::set_override(ColorSupport::Truecolor);

        let unicode_string = UnicodeString::from("HELLO WORLD");

        let ansi_styled_string = color_wheel_rgb.colorize_into_string(
            &unicode_string,
            GradientGenerationPolicy::RegenerateGradientAndIndexBasedOnTextLength,
            TextColorizationPolicy::ColorEachCharacter(None),
        );

        println!("ansi_styled_string: {}", ansi_styled_string);
        println!("ansi_styled_string: {:?}", ansi_styled_string);

        assert_eq2!(
            ansi_styled_string,
            "\u{1b}[38;2;0;0;0mH\u{1b}[0m\u{1b}[38;2;0;0;0mE\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;23;23;23mL\u{1b}[0m\u{1b}[38;2;46;46;46mO\u{1b}[0m\u{1b}[38;2;46;46;46m \u{1b}[0m\u{1b}[38;2;70;70;70mW\u{1b}[0m\u{1b}[38;2;70;70;70mO\u{1b}[0m\u{1b}[38;2;93;93;93mR\u{1b}[0m\u{1b}[38;2;93;93;93mL\u{1b}[0m\u{1b}[38;2;116;116;116mD\u{1b}[0m"
        );

        global_color_support::clear_override()
    }
}
