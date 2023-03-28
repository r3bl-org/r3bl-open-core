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

//! These are the colors use to highlight the MD document. These are all sensitive to [ColorSupport]
//! constraints. You can find ANSI colors [here](https://www.ditig.com/256-colors-cheat-sheet).

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;

use crate::*;

/// This style is for the foreground text of the entire document. This is the default style. It is
/// overridden by other styles like bold, italic, etc. below.
pub fn get_foreground_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::White),
            ColorSupport::Ansi256 => TuiColor::Ansi(244), // Grey50.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#c1b3d0")),
        }
    }
}

/// This style is for things like `[`, `]`, `*`, "`", etc. They are dimmed so that they don't
/// distract from the main content they are wrapping like a link or inline code block, etc.
pub fn get_foreground_dim_style() -> Style {
    get_foreground_style()
        + style! {
            attrib: [dim]
        }
}

/// This is just for the bold content, not the enclosing `**`.
pub fn get_bold_style() -> Style {
    style! {
        attrib: [bold]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow),
            ColorSupport::Ansi256 => TuiColor::Ansi(184), // Yellow3.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#dacd24")),
        }
    }
}

/// This is just for the bold content, not the enclosing `*`.
pub fn get_italic_style() -> Style {
    style! {
        attrib: [italic]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkYellow),
            ColorSupport::Ansi256 => TuiColor::Ansi(166), // DarkOrange3.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#a59e3a")),
        }
    }
}

/// This is just for the bold content, not the enclosing `***`.
pub fn get_bold_italic_style() -> Style {
    style! {
        attrib: [bold, italic]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow),
            ColorSupport::Ansi256 => TuiColor::Ansi(184), // Yellow3.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#dacd24")),
        }
    }
}

/// This is just for the bold content, not the enclosing "`".
pub fn get_inline_code_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Magenta),
            ColorSupport::Ansi256 => TuiColor::Ansi(165), // Magenta2.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ce55b7")),
        }
    }
}

/// This is just for the link text not the enclosing `[` and `]`.
pub fn get_link_text_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Blue),
            ColorSupport::Ansi256 => TuiColor::Ansi(33), // DodgerBlue1.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4f86ed")),
        }
    }
}

/// This is just for the link url not the enclosing `(` and `)`.
pub fn get_link_url_style() -> Style {
    style! {
        attrib: [underline]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Blue),
            ColorSupport::Ansi256 => TuiColor::Ansi(39), // DeepSkyBlue1.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#16adf3")),
        }
    }
}

/// This is for the entire checkbox span (checked).
pub fn get_checkbox_checked_style() -> Style {
    style! {
        attrib: [bold, dim]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
            ColorSupport::Ansi256 => TuiColor::Ansi(91), // DarkMagenta.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#6f5170"))
        }
    }
}

/// This is for the entire checkbox span (unchecked).
pub fn get_checkbox_unchecked_style() -> Style {
    style! {
        attrib: [bold]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Green),
            ColorSupport::Ansi256 => TuiColor::Ansi(41), // SpringGreen3.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#5aab82"))
        }
    }
}

/// This is for the bullet or numbered bullet of a list item, not the content.
pub fn get_list_bullet_style() -> Style {
    pub fn get_list_content_style() -> Style {
        style! {
            color_fg: match ColorSupport::detect() {
                ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
                ColorSupport::Ansi256 => TuiColor::Ansi(87), // DarkSlateGray2. There is no equivalent.
                ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ad83da")), // Very soft violet.
            }
        }
    }

    get_list_content_style()
        + style! {
            attrib: [dim]
        }
}

pub fn get_code_block_lang_style() -> Style {
    get_inline_code_style()
        + style! {
            attrib: [italic]
        }
}

pub fn get_code_block_content_style() -> Style { get_inline_code_style() }

/// - Bg color: #4f86ed
/// - Fg color: black
pub fn get_metadata_title_marker_style() -> Style {
    style! {
        color_fg: TuiColor::Basic(ANSIBasicColor::Black)
        color_bg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
            ColorSupport::Ansi256 => TuiColor::Ansi(39), // DeepSkyBlue1.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4f86ed")), // Soft blue.
        }
    }
}

/// - Fg color: #4fcbd4
/// - Bg color: #444444
pub fn get_metadata_title_value_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan),
            ColorSupport::Ansi256 => TuiColor::Ansi(51), // Cyan1.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4fcbd4")), // Moderate cyan.
        }
        color_bg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkGrey),
            ColorSupport::Ansi256 => TuiColor::Ansi(238), // Grey27.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#444444")), // Very dark gray.
        }
    }
}

/// - Bg color: #ad83da
/// - Fg color: black
pub fn get_metadata_tags_marker_style() -> Style {
    style! {
        color_fg: TuiColor::Basic(ANSIBasicColor::Black)
        color_bg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow), // There is no equivalent.
            ColorSupport::Ansi256 => TuiColor::Ansi(133), // MediumOrchid3. There is no equivalent.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ad83da")), // Very soft violet.
        }
    }
}

/// - Fg color: #e2a1e3
/// - Bg color: #303030
pub fn get_metadata_tags_values_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
            ColorSupport::Ansi256 => TuiColor::Ansi(45), // Turquoise2
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#e2a1e3")), // Soft violet.
        }
        color_bg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkGrey),
            ColorSupport::Ansi256 => TuiColor::Ansi(236), // Grey19.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#303030")), // Very dark gray.
        }
    }
}

const SPEED: ColorWheelSpeed = ColorWheelSpeed::Medium;
const STEPS: usize = 20;

impl ColorWheel {
    /// More info on gradients: <https://uigradients.com/>.
    pub fn from_heading_data(heading_data: &HeadingData) -> Self {
        match heading_data.level {
            HeadingLevel::Heading1 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#01fa22", "#00eef2"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::LightYellowToWhite, SPEED),
            ]),

            HeadingLevel::Heading2 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#fff200", "#de211b"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::MediumGreenToMediumBlue, SPEED),
            ]),

            HeadingLevel::Heading3 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#00dbde", "#fc00ff"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::DarkRedToDarkMagenta, SPEED),
            ]),

            HeadingLevel::Heading4 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#ff28a9", "#bd60eb"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::RedToBrightPink, SPEED),
            ]),

            HeadingLevel::Heading5 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#ff6a00", "#ee0979"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::OrangeToNeonPink, SPEED),
            ]),

            HeadingLevel::Heading6 => ColorWheel::new(vec![
                ColorWheelConfig::Rgb(
                    Vec::from(["#8470ba", "#12c2e9"].map(String::from)),
                    SPEED,
                    STEPS,
                ),
                ColorWheelConfig::Ansi256(Ansi256GradientIndex::GreenToBlue, ColorWheelSpeed::Fast),
            ]),
        }
    }
}
