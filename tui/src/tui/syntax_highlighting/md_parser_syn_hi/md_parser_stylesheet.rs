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

//! These are the colors use to highlight the MD document. These are all sensitive to
//! [ColorSupport] constraints. You can find ANSI colors
//! [here](https://www.ditig.com/256-colors-cheat-sheet).

use r3bl_ansi_color::{global_color_support, ColorSupport};
use r3bl_rs_utils_core::{ANSIBasicColor,
                         Ansi256GradientIndex,
                         AnsiValue,
                         ColorWheel,
                         ColorWheelConfig,
                         ColorWheelSpeed,
                         RgbValue,
                         TuiColor,
                         TuiStyle};
use r3bl_macro::tui_style;

use crate::HeadingData;

/// This style is for any selected range in the document.
pub fn get_selection_style() -> TuiStyle {
    let color_fg = TuiColor::Rgb(RgbValue::from_hex("#dddddd"));
    let color_bg = TuiColor::Rgb(RgbValue::from_hex("#ff00ff"));
    tui_style! {
        color_fg: color_fg
        color_bg: color_bg
    }
}

/// This style is for the foreground text of the entire document. This is the default
/// style. It is overridden by other styles like bold, italic, etc. below.
pub fn get_foreground_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#c1b3d0")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(244)), // Grey50.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::White),
            _ => TuiColor::Basic(ANSIBasicColor::White),
        }
    }
}

/// This style is for things like `[`, `]`, `*`, "`", etc. They are dimmed so that they
/// don't distract from the main content they are wrapping like a link or inline code
/// block, etc.
pub fn get_foreground_dim_style() -> TuiStyle {
    get_foreground_style()
        + tui_style! {
            attrib: [dim]
            color_fg: TuiColor::Rgb(RgbValue::from_hex("#5f5f5f"))
        }
}

/// This is just for the bold content, not the enclosing `**`.
pub fn get_bold_style() -> TuiStyle {
    tui_style! {
        attrib: [bold]
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#dacd24")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(226)), // Yellow1.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow),
            _ => TuiColor::Basic(ANSIBasicColor::Yellow),
        }
    }
}

/// This is just for the bold content, not the enclosing `*`.
pub fn get_italic_style() -> TuiStyle {
    tui_style! {
        attrib: [italic]
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#a59e3a")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(208)), // DarkOrange.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkYellow),
            _ => TuiColor::Basic(ANSIBasicColor::DarkYellow),
        }
    }
}

/// This is just for the bold content, not the enclosing `***`.
pub fn get_bold_italic_style() -> TuiStyle {
    tui_style! {
        attrib: [bold, italic]
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#dacd24")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(184)), // Yellow3.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow),
            _ => TuiColor::Basic(ANSIBasicColor::Yellow),
        }
    }
}

/// This is just for the bold content, not the enclosing "`".
pub fn get_inline_code_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect(){
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ce55b7")),
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Magenta),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(169)), // HotPink2.
            _ => TuiColor::Basic(ANSIBasicColor::Magenta),
        }
    }
}

/// This is just for the link text not the enclosing `[` and `]`.
pub fn get_link_text_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4f86ed")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(33)), // DodgerBlue1.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Blue),
            _ => TuiColor::Basic(ANSIBasicColor::Blue),
        }
    }
}

/// This is just for the link url not the enclosing `(` and `)`.
pub fn get_link_url_style() -> TuiStyle {
    tui_style! {
        attrib: [underline]
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#16adf3")),
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(39)), // DeepSkyBlue1.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Blue),
            _ => TuiColor::Basic(ANSIBasicColor::Blue),
        }
    }
}

/// This is for the entire checkbox span (checked).
pub fn get_checkbox_checked_style() -> TuiStyle {
    tui_style! {
        attrib: [bold, dim]
        color_fg: match global_color_support::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
            _ => TuiColor::Rgb(RgbValue::from_hex("#14a45b")),
        }
    }
}

/// This is for the entire checkbox span (unchecked).
pub fn get_checkbox_unchecked_style() -> TuiStyle {
    tui_style! {
        attrib: [bold]
        color_fg: match global_color_support::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Green),
            _ => TuiColor::Rgb(RgbValue::from_hex("#e1ff2f"))
        }
    }
}

/// This is for the bullet or numbered bullet of a list item, not the content.
pub fn get_list_bullet_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow), // There is no equivalent.
            _ => TuiColor::Rgb(RgbValue::from_hex("#f8f8a6")), // Pale yellow.
        }
    }
}

pub fn get_code_block_lang_style() -> TuiStyle {
    get_inline_code_style()
        + tui_style! {
            attrib: [italic]
        }
}

pub fn get_code_block_content_style() -> TuiStyle { get_inline_code_style() }

/// - Bg color: #4f86ed
/// - Fg color: black
pub fn get_metadata_title_marker_style() -> TuiStyle {
    tui_style! {
        color_fg: TuiColor::Basic(ANSIBasicColor::Black)
        color_bg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4f86ed")), // Soft blue.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(39)), // DeepSkyBlue1.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
            _ => TuiColor::Basic(ANSIBasicColor::Cyan),
        }
    }
}

/// - Fg color: #4fcbd4
/// - Bg color: #444444
pub fn get_metadata_title_value_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4fcbd4")), // Moderate cyan.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(51)), // Cyan1.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan),
            _ => TuiColor::Basic(ANSIBasicColor::Cyan),
        }
        color_bg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#444444")), // Very dark gray.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(238)), // Grey27.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkGrey),
            _ => TuiColor::Basic(ANSIBasicColor::DarkGrey),
        }
    }
}

/// - Bg color: #ad83da
/// - Fg color: black
pub fn get_metadata_tags_marker_style() -> TuiStyle {
    tui_style! {
        color_fg: TuiColor::Basic(ANSIBasicColor::Black)
        color_bg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ad83da")), // Very soft violet.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(133)), // MediumOrchid3. There is no equivalent.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Yellow), // There is no equivalent.
            _ => TuiColor::Basic(ANSIBasicColor::Yellow),
        }
    }
}

/// - Fg color: #e2a1e3
/// - Bg color: #303030
pub fn get_metadata_tags_values_style() -> TuiStyle {
    tui_style! {
        color_fg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#e2a1e3")), // Soft violet.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(45)), // Turquoise2
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
            _ => TuiColor::Basic(ANSIBasicColor::Cyan),
        }
        color_bg: match global_color_support::detect() {
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#303030")), // Very dark gray.
            ColorSupport::Ansi256 => TuiColor::Ansi(AnsiValue::new(236)), // Grey19.
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::DarkGrey),
            _ => TuiColor::Basic(ANSIBasicColor::DarkGrey),
        }
    }
}

const SPEED: ColorWheelSpeed = ColorWheelSpeed::Medium;
const ANSI_SPEED: ColorWheelSpeed = ColorWheelSpeed::Slow;
const STEPS: usize = 20;

/// Currently unique coloring of up to 6 heading levels are supported.
/// More info on gradients: <https://uigradients.com/>.
pub fn create_color_wheel_from_heading_data(
    heading_data: &HeadingData<'_>,
) -> ColorWheel {
    match heading_data.heading_level.level {
        1 => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#01fa22", "#00eef2"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::LightYellowToWhite,
                ANSI_SPEED,
            ),
        ]),

        2 => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#fff200", "#de211b"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::GreenToBlue, ANSI_SPEED),
        ]),

        3 => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#00dbde", "#fc00ff"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::OrangeToNeonPink, ANSI_SPEED),
        ]),

        4 => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#ff28a9", "#bd60eb"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::LightOrangeToLightPurple,
                ANSI_SPEED,
            ),
        ]),

        5 => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#ff6a00", "#ee0979"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::RustToPurple, ANSI_SPEED),
        ]),

        _ => ColorWheel::new(vec![
            ColorWheelConfig::Rgb(
                Vec::from(["#8470ba", "#12c2e9"].map(String::from)),
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::DarkOliveGreenToDarkLavender,
                ANSI_SPEED,
            ),
        ]),
    }
}
