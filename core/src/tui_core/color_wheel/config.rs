/*
 *   Copyright (c) 2024 R3BL LLC
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

use r3bl_ansi_color::{ColorSupport, global_color_support};
use serde::{Deserialize, Serialize};

use super::{Lolcat, LolcatBuilder};
use crate::{Ansi256GradientIndex,
            MicroVecBackingStore,
            TinyStringBackingStore,
            TinyVecBackingStore,
            TuiColor};

/// For RGB colors:
/// 1. The stops are the colors that will be used to create the gradient.
/// 2. The speed is how fast the color wheel will rotate.
/// 3. The steps are the number of colors that will be generated. The larger this number is the
///    smoother the transition will be between each color. 100 is a good number to start with.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum ColorWheelConfig {
    Rgb(
        /* stops */ MicroVecBackingStore<TinyStringBackingStore>,
        /* speed */ ColorWheelSpeed,
        /* steps */ u8,
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

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ColorWheelDirection {
    Forward,
    Reverse,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ColorWheelSpeed {
    Slow = 10,
    Medium = 5,
    Fast = 2,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, size_of::SizeOf)]
pub enum GradientKind {
    ColorWheel(TinyVecBackingStore<TuiColor>),
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
