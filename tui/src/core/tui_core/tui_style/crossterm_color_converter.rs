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

use crate::{global_color_support,
            ANSIBasicColor,
            ASTColor,
            AnsiValue,
            ColorSupport,
            RgbValue,
            TransformColor,
            TuiColor};

#[rustfmt::skip]
pub fn convert_from_crossterm_color_to_tui_color(value: crossterm::style::Color) -> TuiColor {
    match value {
        // Basic colors.
        crossterm::style::Color::Reset       => TuiColor::Reset,
        crossterm::style::Color::Black       => TuiColor::Basic(ANSIBasicColor::Black),
        crossterm::style::Color::DarkGrey    => TuiColor::Basic(ANSIBasicColor::DarkGray),
        crossterm::style::Color::Red         => TuiColor::Basic(ANSIBasicColor::Red),
        crossterm::style::Color::DarkRed     => TuiColor::Basic(ANSIBasicColor::DarkRed),
        crossterm::style::Color::Green       => TuiColor::Basic(ANSIBasicColor::Green),
        crossterm::style::Color::DarkGreen   => TuiColor::Basic(ANSIBasicColor::DarkGreen),
        crossterm::style::Color::Yellow      => TuiColor::Basic(ANSIBasicColor::Yellow),
        crossterm::style::Color::DarkYellow  => TuiColor::Basic(ANSIBasicColor::DarkYellow),
        crossterm::style::Color::Blue        => TuiColor::Basic(ANSIBasicColor::Blue),
        crossterm::style::Color::DarkBlue    => TuiColor::Basic(ANSIBasicColor::DarkBlue),
        crossterm::style::Color::Magenta     => TuiColor::Basic(ANSIBasicColor::Magenta),
        crossterm::style::Color::DarkMagenta => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
        crossterm::style::Color::Cyan        => TuiColor::Basic(ANSIBasicColor::Cyan),
        crossterm::style::Color::DarkCyan    => TuiColor::Basic(ANSIBasicColor::DarkCyan),
        crossterm::style::Color::White       => TuiColor::Basic(ANSIBasicColor::White),
        crossterm::style::Color::Grey        => TuiColor::Basic(ANSIBasicColor::Gray),

        // RGB colors.
        crossterm::style::Color::Rgb { r, g, b } => TuiColor::Rgb(RgbValue {
            red: r,
            green: g,
            blue: b,
        }),

        // ANSI colors.
        crossterm::style::Color::AnsiValue(number) => {
            TuiColor::Ansi(AnsiValue::new(number))
        }
    }
}

/// Respect the color support of the terminal and downgrade the color if needed. This
/// really only applies to the [TuiColor::Rgb] variant.
pub fn convert_from_tui_color_to_crossterm_color(
    from_tui_color: TuiColor,
) -> crossterm::style::Color {
    match from_tui_color {
        TuiColor::Reset => crossterm::style::Color::Reset,

        TuiColor::Basic(from_basic_color) => match global_color_support::detect() {
            // Convert to grayscale.
            #[rustfmt::skip]
            ColorSupport::NoColor | ColorSupport::Grayscale => match from_basic_color {
                ANSIBasicColor::Black =>       crossterm::style::Color::Black,
                ANSIBasicColor::White =>       crossterm::style::Color::White,
                ANSIBasicColor::Gray =>        convert_rgb_to_ansi_grayscale(192, 192, 192),
                ANSIBasicColor::DarkGray =>    convert_rgb_to_ansi_grayscale(128, 128, 128),
                ANSIBasicColor::Red =>         convert_rgb_to_ansi_grayscale(255, 0,   0),
                ANSIBasicColor::DarkRed =>     convert_rgb_to_ansi_grayscale(128, 0,   0),
                ANSIBasicColor::Green =>       convert_rgb_to_ansi_grayscale(0,   255, 0),
                ANSIBasicColor::DarkGreen =>   convert_rgb_to_ansi_grayscale(0,   128, 0),
                ANSIBasicColor::Yellow =>      convert_rgb_to_ansi_grayscale(255, 255, 0),
                ANSIBasicColor::DarkYellow =>  convert_rgb_to_ansi_grayscale(128, 128, 0),
                ANSIBasicColor::Blue =>        convert_rgb_to_ansi_grayscale(0,   0,   255),
                ANSIBasicColor::DarkBlue =>    convert_rgb_to_ansi_grayscale(0,   0,   128),
                ANSIBasicColor::Magenta =>     convert_rgb_to_ansi_grayscale(255, 0,   255),
                ANSIBasicColor::DarkMagenta => convert_rgb_to_ansi_grayscale(128, 0,   128),
                ANSIBasicColor::Cyan =>        convert_rgb_to_ansi_grayscale(0,   255, 255),
                ANSIBasicColor::DarkCyan =>    convert_rgb_to_ansi_grayscale(0,   128, 128),
            },

            // Keep it as is.
            #[rustfmt::skip]
            ColorSupport::Ansi256 | ColorSupport::Truecolor => match from_basic_color {
                ANSIBasicColor::Black =>        crossterm::style::Color::Black,
                ANSIBasicColor::White =>        crossterm::style::Color::White,
                ANSIBasicColor::Gray =>         crossterm::style::Color::Grey,
                ANSIBasicColor::DarkGray =>     crossterm::style::Color::DarkGrey,
                ANSIBasicColor::Red =>          crossterm::style::Color::Red,
                ANSIBasicColor::DarkRed =>      crossterm::style::Color::DarkRed,
                ANSIBasicColor::Green =>        crossterm::style::Color::Green,
                ANSIBasicColor::DarkGreen =>    crossterm::style::Color::DarkGreen,
                ANSIBasicColor::Yellow =>       crossterm::style::Color::Yellow,
                ANSIBasicColor::DarkYellow =>   crossterm::style::Color::DarkYellow,
                ANSIBasicColor::Blue =>         crossterm::style::Color::Blue,
                ANSIBasicColor::DarkBlue =>     crossterm::style::Color::DarkBlue,
                ANSIBasicColor::Magenta =>      crossterm::style::Color::Magenta,
                ANSIBasicColor::DarkMagenta =>  crossterm::style::Color::DarkMagenta,
                ANSIBasicColor::Cyan =>         crossterm::style::Color::Cyan,
                ANSIBasicColor::DarkCyan =>     crossterm::style::Color::DarkCyan,
            },
        },

        TuiColor::Ansi(ansi) => {
            match global_color_support::detect() {
                // Keep it as is.
                ColorSupport::Truecolor | ColorSupport::Ansi256 => {
                    crossterm::style::Color::AnsiValue(ansi.index)
                }

                // Convert to grayscale.
                ColorSupport::Grayscale | ColorSupport::NoColor => {
                    let ansi_grayscale_color = ASTColor::Ansi(ansi).as_grayscale();
                    crossterm::style::Color::AnsiValue(ansi_grayscale_color.index)
                }
            }
        }

        // Downgrade the color if needed.
        TuiColor::Rgb(from_rgb_value) => {
            let RgbValue {
                red: r,
                green: g,
                blue: b,
            } = from_rgb_value;

            match global_color_support::detect() {
                // Keep it as is.
                ColorSupport::Truecolor => crossterm::style::Color::Rgb { r, g, b },

                // Convert to ANSI256.
                ColorSupport::Ansi256 => {
                    let ansi_value = AnsiValue::from(from_rgb_value).index;
                    crossterm::style::Color::AnsiValue(ansi_value)
                }

                // Convert to grayscale.
                ColorSupport::NoColor | ColorSupport::Grayscale => {
                    convert_rgb_to_ansi_grayscale(r, g, b)
                }
            }
        }
    }
}

fn convert_rgb_to_ansi_grayscale(r: u8, g: u8, b: u8) -> crossterm::style::Color {
    let ansi = ASTColor::Rgb((r, g, b).into()).as_grayscale();
    crossterm::style::Color::AnsiValue(ansi.index)
}
