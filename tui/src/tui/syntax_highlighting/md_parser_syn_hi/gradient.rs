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

use crate::*;

impl ColorWheel {
    /// More info on gradients: <https://uigradients.com/>.
    pub fn from_heading_data(heading_data: &HeadingData) -> Self {
        match heading_data.level {
            HeadingLevel::Heading1 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from([/* cyan */ "#12c2e9", /* purple */ "#c471ed"].map(String::from)),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::DarkRedToDarkMagenta,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            HeadingLevel::Heading2 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from([/* purple */ "#c471ed", /* red */ "#f64f59"].map(String::from)),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::RedToBrightPink,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            HeadingLevel::Heading3 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from([/* red */ "#b92b27", /* blue */ "#1565C0"].map(String::from)),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::OrangeToNeonPink,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            HeadingLevel::Heading4 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(
                        [/* pink */ "#FF0099", /* dark purple */ "#493240"].map(String::from),
                    ),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::LightYellowToWhite,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            HeadingLevel::Heading5 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from([/* green */ "#00F260", /* blue */ "#0575E6"].map(String::from)),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::MediumGreenToMediumBlue,
                    ColorWheelSpeed::Medium,
                ),
            ]),
            HeadingLevel::Heading6 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from([/* red */ "#b21f1f", /* yellow */ "#fdbb2d"].map(String::from)),
                    ColorWheelSpeed::Medium,
                    20,
                ),
                ColorWheelConfig::Ansi256(
                    Ansi256GradientIndex::GreenToBlue,
                    ColorWheelSpeed::Medium,
                ),
            ]),
        }
    }
}
