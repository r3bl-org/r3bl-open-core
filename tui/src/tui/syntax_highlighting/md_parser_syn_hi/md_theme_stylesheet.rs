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

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;

use crate::*;

/// These are the colors use to highlight the MD document. These are all sensitive to [ColorSupport]
/// constraints. You can find ANSI colors [here](https://www.ditig.com/256-colors-cheat-sheet).
pub fn get_foreground_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::White),
            ColorSupport::Ansi256 => TuiColor::Ansi(244), // Grey50.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#c1b3d0")),
        }
    }
}

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

pub fn get_inline_code_style() -> Style {
    style! {
        attrib: [bold]
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Magenta),
            ColorSupport::Ansi256 => TuiColor::Ansi(165), // Magenta2.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ce55b7")),
        }
    }
}

pub fn get_link_base_style() -> Style {
    get_foreground_style()
        + style! {
            attrib: [dim]
        }
}

pub fn get_link_text_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Blue),
            ColorSupport::Ansi256 => TuiColor::Ansi(33), // DodgerBlue1.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#4f86ed")),
        }
    }
}

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

pub fn get_list_content_style() -> Style {
    style! {
        color_fg: match ColorSupport::detect() {
            ColorSupport::Grayscale => TuiColor::Basic(ANSIBasicColor::Cyan), // There is no equivalent.
            ColorSupport::Ansi256 => TuiColor::Ansi(87), // DarkSlateGray2. There is no equivalent.
            ColorSupport::Truecolor => TuiColor::Rgb(RgbValue::from_hex("#ad83da")), // Very soft violet.
        }
    }
}

pub fn get_list_bullet_style() -> Style {
    get_list_content_style()
        + style! {
            attrib: [dim]
        }
}
