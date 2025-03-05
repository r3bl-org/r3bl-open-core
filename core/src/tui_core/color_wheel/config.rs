/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
use sizing::VecSteps;
use smallstr::SmallString;
use smallvec::SmallVec;

use super::{Lolcat, LolcatBuilder, Seed};
use crate::{Ansi256GradientIndex, Colorize, TuiColor};

/// These are sized to allow for stack allocation rather than heap allocation. If for some
/// reason these are exceeded, then they will [smallvec::SmallVec::spilled] over into the
/// heap.
pub(in crate::tui_core) mod sizing {
    use super::*;

    pub type StringHexColor = SmallString<[u8; MAX_HEX_COLOR_STRING_SIZE]>;
    const MAX_HEX_COLOR_STRING_SIZE: usize = 8;

    pub type VecStops = SmallVec<[StringHexColor; MAX_STOPS_SIZE]>;
    const MAX_STOPS_SIZE: usize = 16;

    pub type VecSteps = SmallVec<[TuiColor; defaults::Defaults::Steps as usize]>;
}

pub mod defaults {
    use super::sizing::*;

    #[repr(u8)]
    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    pub enum Defaults {
        Steps = 64,
    }

    /// Provides an owned copy of the default gradient stops.
    pub fn get_default_gradient_stops() -> VecStops {
        let size_hint = DEFAULT_GRADIENT_STOPS.len();
        let mut stops = VecStops::with_capacity(size_hint);
        for &stop in DEFAULT_GRADIENT_STOPS.iter() {
            stops.push(stop.into());
        }
        stops
    }

    /// Use [get_default_gradient_stops] to get the default gradient stops.
    /// More info: <https://www.colorhexa.com/>
    const DEFAULT_GRADIENT_STOPS: [&str; 3] = [
        /* cyan */ "#00ffff", /* magenta */ "#ff00ff",
        /* blue */ "#0000ff",
    ];
}

/// For RGB colors:
/// 1. The stops are the colors that will be used to create the gradient.
/// 2. The speed is how fast the color wheel will rotate.
/// 3. The steps are the number of colors that will be generated. The larger this number is the
///    smoother the transition will be between each color. 100 is a good number to start with.
#[derive(Clone, PartialEq, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ColorWheelConfig {
    Rgb(
        /* stops */ sizing::VecStops,
        /* speed */ ColorWheelSpeed,
        /* steps */ u8,
    ),
    RgbRandom(/* speed */ ColorWheelSpeed),
    Ansi256(Ansi256GradientIndex, ColorWheelSpeed),
    Lolcat(LolcatBuilder),
}

impl ColorWheelConfig {
    pub fn config_contains_bg_lolcat(configs: &[ColorWheelConfig]) -> bool {
        configs.iter().any(|config| {
            matches!(
                config,
                ColorWheelConfig::Lolcat(LolcatBuilder {
                    colorization_strategy: Colorize::BothBackgroundAndForeground,
                    ..
                })
            )
        })
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

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ColorWheelDirection {
    Forward,
    Reverse,
}

#[repr(u8)]
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ColorWheelSpeed {
    Slow = 10,
    Medium = 5,
    Fast = 2,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, PartialEq, Debug)]
pub enum GradientKind {
    ColorWheel(VecSteps),
    Lolcat(Lolcat),
    NotCalculatedYet,
}

/// Gradient has to be generated before this will be anything other than
/// [GradientLengthKind::NotCalculatedYet].
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GradientLengthKind {
    ColorWheel(usize),
    Lolcat(Seed),
    NotCalculatedYet,
}
