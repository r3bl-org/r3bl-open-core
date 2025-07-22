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

//! These are the colors use to highlight the MD document. These are all sensitive to
//! [`ColorSupport`] constraints. You can find ANSI colors
//! [here](https://www.ditig.com/256-colors-cheat-sheet).

use smallvec::smallvec;

use crate::{global_color_support, new_style, tui_color, Ansi256GradientIndex,
            ColorSupport, ColorWheel, ColorWheelConfig, ColorWheelSpeed, HeadingData,
            TuiStyle};

/// This style is for any selected range in the document.
#[must_use]
pub fn get_selection_style() -> TuiStyle {
    let foreground = tui_color!(hex "#dddddd");
    let background = tui_color!(hex "#ff00ff");
    new_style!(
        color_fg: {foreground}
        color_bg: {background}
    )
}

/// This style is for the foreground text of the entire document. This is the default
/// style. It is overridden by other styles like bold, italic, etc. below.
#[must_use]
pub fn get_foreground_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#c1b3d0"),
                ColorSupport::Ansi256 => tui_color!(ansi 244), // Grey50.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(white),
            }
        }
    )
}

#[allow(clippy::doc_markdown)]
/// This style is for things like `[`, `]`, `*`, "`", etc. They are dimmed so that they
/// don't distract from the main content they are wrapping like a link or inline code
/// block, etc.
#[must_use]
pub fn get_foreground_dim_style() -> TuiStyle {
    get_foreground_style()
        + new_style!(
            dim
            color_fg: {tui_color!(hex "#5f5f5f")}
        )
}

/// This is just for the bold content, not the enclosing `**`.
#[must_use]
pub fn get_bold_style() -> TuiStyle {
    new_style! (
        bold
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#dacd24"),
                ColorSupport::Ansi256 => tui_color!(ansi 226), // Yellow1.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(yellow),
            }
        }
    )
}

/// This is just for the bold content, not the enclosing `*`.
#[must_use]
pub fn get_italic_style() -> TuiStyle {
    new_style!(
        italic
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#a59e3a"),
                ColorSupport::Ansi256 => tui_color!(ansi 208), // DarkOrange.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(dark_yellow),
            }
        }
    )
}

/// This is just for the bold content, not the enclosing `***`.
#[must_use]
pub fn get_bold_italic_style() -> TuiStyle {
    new_style!(
        bold italic
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#dacd24"),
                ColorSupport::Ansi256 => tui_color!(ansi 184), // Yellow3.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(yellow),
            }
        }
    )
}

#[allow(clippy::doc_markdown)]
/// This is just for the bold content, not the enclosing "`".
#[must_use]
pub fn get_inline_code_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect(){
                ColorSupport::Truecolor => tui_color!(hex "#ce55b7"),
                ColorSupport::Ansi256 => tui_color!(ansi 169), // HotPink2.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(magenta),
            }
        }
    )
}

/// This is just for the link text not the enclosing `[` and `]`.
#[must_use]
pub fn get_link_text_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#4f86ed"),
                ColorSupport::Ansi256 => tui_color!(ansi 33), // DodgerBlue1.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(blue),
            }
        }
    )
}

/// This is just for the link url not the enclosing `(` and `)`.
#[must_use]
pub fn get_link_url_style() -> TuiStyle {
    new_style!(
        underline
        color_fg:{
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#16adf3"),
                ColorSupport::Ansi256 => tui_color!(ansi 39), // DeepSkyBlue1.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(blue),
            }
        }
    )
}

/// This is for the entire checkbox span (checked).
#[must_use]
pub fn get_checkbox_checked_style() -> TuiStyle {
    new_style!(
        bold dim
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Grayscale => tui_color!(dark_magenta),
                _ => tui_color!(hex "#14a45b"),
            }
        }
    )
}

/// This is for the entire checkbox span (unchecked).
#[must_use]
pub fn get_checkbox_unchecked_style() -> TuiStyle {
    new_style!(
        bold
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Grayscale => tui_color!(green),
                _ => tui_color!(hex "#e1ff2f")
            }
        }
    )
}

/// This is for the bullet or numbered bullet of a list item, not the content.
#[must_use]
pub fn get_list_bullet_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Grayscale => tui_color!(yellow), // There is no equivalent.
                _ => tui_color!(hex "#f8f8a6"), // Pale yellow.
            }
        }
    )
}

#[must_use]
pub fn get_code_block_lang_style() -> TuiStyle {
    get_inline_code_style() + new_style!(italic)
}

#[must_use]
pub fn get_code_block_content_style() -> TuiStyle { get_inline_code_style() }

/// - Bg color: #4f86ed
/// - Fg color: black
#[must_use]
pub fn get_metadata_title_marker_style() -> TuiStyle {
    new_style!(
        color_fg: {tui_color!(black)}
        color_bg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#4f86ed"), // Soft blue.
                ColorSupport::Ansi256 => tui_color!(ansi 39), // DeepSkyBlue1.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(cyan), // There is no equivalent.
            }
        }
    )
}

/// - Fg color: #4fcbd4
/// - Bg color: #444444
#[must_use]
pub fn get_metadata_title_value_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#4fcbd4"), // Moderate cyan.
                ColorSupport::Ansi256 => tui_color!(ansi 51), // Cyan1.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(cyan),
            }
        }
        color_bg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#444444"), // Very dark gray.
                ColorSupport::Ansi256 => tui_color!(ansi 238), // Grey27.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(dark_gray),
            }
        }
    )
}

/// - Bg color: #ad83da
/// - Fg color: black
#[must_use]
pub fn get_metadata_tags_marker_style() -> TuiStyle {
    new_style!(
        color_fg: {tui_color!(black)}
        color_bg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#ad83da"), // Very soft violet.
                ColorSupport::Ansi256 => tui_color!(ansi 133), // MediumOrchid3. There is no equivalent.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(yellow), // There is no equivalent.
            }
        }
    )
}

/// - Fg color: #e2a1e3
/// - Bg color: #303030
#[must_use]
pub fn get_metadata_tags_values_style() -> TuiStyle {
    new_style!(
        color_fg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#e2a1e3"), // Soft violet.
                ColorSupport::Ansi256 => tui_color!(ansi 45), // Turquoise2
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(cyan) // There is no equivalent.
            }
        }
        color_bg: {
            match global_color_support::detect() {
                ColorSupport::Truecolor => tui_color!(hex "#303030"), // Very dark gray.
                ColorSupport::Ansi256 => tui_color!(ansi 236), // Grey19.
                ColorSupport::Grayscale | ColorSupport::NoColor => tui_color!(dark_gray)
            }
        }
    )
}

const SPEED: ColorWheelSpeed = ColorWheelSpeed::Medium;
const ANSI_SPEED: ColorWheelSpeed = ColorWheelSpeed::Slow;
const STEPS: u8 = 20;

/// Currently unique coloring of up to 6 heading levels are supported.
/// More info on gradients: <https://uigradients.com/>.
#[must_use]
pub fn create_color_wheel_from_heading_data(
    heading_data: &HeadingData<'_>,
) -> ColorWheel {
    match heading_data.level.level {
        1 => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#01fa22".into(), "#00eef2".into()],
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::LightYellowToWhite,
                ANSI_SPEED,
            ),
        ]),

        2 => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#fff200".into(), "#de211b".into()],
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::GreenToBlue, ANSI_SPEED),
        ]),

        3 => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#00dbde".into(), "#fc00ff".into()],
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::OrangeToNeonPink, ANSI_SPEED),
        ]),

        4 => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#ff28a9".into(), "#bd60eb".into()],
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(
                Ansi256GradientIndex::LightOrangeToLightPurple,
                ANSI_SPEED,
            ),
        ]),

        5 => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#ff6a00".into(), "#ee0979".into()],
                SPEED,
                STEPS,
            ),
            ColorWheelConfig::Ansi256(Ansi256GradientIndex::RustToPurple, ANSI_SPEED),
        ]),

        _ => ColorWheel::new(smallvec![
            ColorWheelConfig::Rgb(
                smallvec!["#8470ba".into(), "#12c2e9".into()],
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
