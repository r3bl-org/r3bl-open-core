// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ANSIBasicColor, ASTColor, AnsiValue, ColorSupport, RgbValue, TransformColor,
            TuiColor, global_color_support};

/// Respect the color support of the terminal and downgrade the color if needed. This
/// really only applies to the [`TuiColor::Rgb`] variant.
impl From<TuiColor> for crossterm::style::Color {
    fn from(from_tui_color: TuiColor) -> Self {
        match from_tui_color {
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
}

fn convert_rgb_to_ansi_grayscale(r: u8, g: u8, b: u8) -> crossterm::style::Color {
    let ansi = ASTColor::Rgb((r, g, b).into()).as_grayscale();
    crossterm::style::Color::AnsiValue(ansi.index)
}
