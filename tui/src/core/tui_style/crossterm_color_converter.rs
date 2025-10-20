// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnsiValue, ColorSupport, RgbValue, TransformColor, TuiColor,
            global_color_support};

/// Respect the color support of the terminal and downgrade the color if needed. This
/// really only applies to the [`TuiColor::Rgb`] variant.
///
/// Basic colors (indices 0-15) have special handling for grayscale degradation.
impl From<TuiColor> for crossterm::style::Color {
    fn from(color: TuiColor) -> Self {
        match color {
            TuiColor::Ansi(ansi) if ansi.index < 16 => {
                // Basic colors (palette indices 0-15).
                match global_color_support::detect() {
                    // Terminal has no color support - reset to terminal default
                    // (`ESC[0m`).
                    ColorSupport::NoColor => crossterm::style::Color::Reset,

                    // Convert to grayscale for low-color terminals.
                    ColorSupport::Grayscale => {
                        match ansi.index {
                            0 => crossterm::style::Color::Black,
                            1 => convert_rgb_to_ansi_grayscale(255, 0, 0), // red
                            2 => convert_rgb_to_ansi_grayscale(0, 255, 0), // green
                            3 => convert_rgb_to_ansi_grayscale(255, 255, 0), // yellow
                            4 => convert_rgb_to_ansi_grayscale(0, 0, 255), // blue
                            5 => convert_rgb_to_ansi_grayscale(255, 0, 255), // magenta
                            6 => convert_rgb_to_ansi_grayscale(0, 255, 255), // cyan
                            7 => crossterm::style::Color::White,
                            8 => convert_rgb_to_ansi_grayscale(128, 128, 128), /* dark_gray */
                            9 => convert_rgb_to_ansi_grayscale(128, 0, 0), // dark_red
                            10 => convert_rgb_to_ansi_grayscale(0, 128, 0), // dark_green
                            11 => convert_rgb_to_ansi_grayscale(128, 128, 0), /* dark_yellow */
                            12 => convert_rgb_to_ansi_grayscale(0, 0, 128), // dark_blue
                            13 => convert_rgb_to_ansi_grayscale(128, 0, 128), /* dark_magenta */
                            14 => convert_rgb_to_ansi_grayscale(0, 128, 128), /* dark_cyan */
                            15 => convert_rgb_to_ansi_grayscale(192, 192, 192), // gray
                            _ => unreachable!(
                                "index < 16 is guaranteed by the outer match guard"
                            ),
                        }
                    }

                    // Keep basic colors as is.
                    ColorSupport::Ansi256 | ColorSupport::Truecolor => match ansi.index {
                        0 => crossterm::style::Color::Black,
                        1 => crossterm::style::Color::DarkRed,
                        2 => crossterm::style::Color::DarkGreen,
                        3 => crossterm::style::Color::DarkYellow,
                        4 => crossterm::style::Color::DarkBlue,
                        5 => crossterm::style::Color::DarkMagenta,
                        6 => crossterm::style::Color::DarkCyan,
                        7 => crossterm::style::Color::White,
                        8 => crossterm::style::Color::DarkGrey,
                        9 => crossterm::style::Color::Red,
                        10 => crossterm::style::Color::Green,
                        11 => crossterm::style::Color::Yellow,
                        12 => crossterm::style::Color::Blue,
                        13 => crossterm::style::Color::Magenta,
                        14 => crossterm::style::Color::Cyan,
                        15 => crossterm::style::Color::Grey,
                        _ => unreachable!(
                            "index < 16 is guaranteed by the outer match guard"
                        ),
                    },
                }
            }

            TuiColor::Ansi(ansi) => {
                // 256-color palette or extended colors (indices 16-255).
                match global_color_support::detect() {
                    // Keep it as is.
                    ColorSupport::Truecolor | ColorSupport::Ansi256 => {
                        crossterm::style::Color::AnsiValue(ansi.index)
                    }

                    // Terminal has no color support - reset to terminal default.
                    ColorSupport::NoColor => crossterm::style::Color::Reset,

                    // Convert to grayscale.
                    ColorSupport::Grayscale => {
                        let ansi_grayscale_color = TuiColor::Ansi(ansi).as_grayscale();
                        crossterm::style::Color::AnsiValue(ansi_grayscale_color.index)
                    }
                }
            }

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

                    // Terminal has no color support - reset to terminal default.
                    ColorSupport::NoColor => crossterm::style::Color::Reset,

                    // Convert to grayscale.
                    ColorSupport::Grayscale => convert_rgb_to_ansi_grayscale(r, g, b),
                }
            }
        }
    }
}

fn convert_rgb_to_ansi_grayscale(r: u8, g: u8, b: u8) -> crossterm::style::Color {
    let ansi = TuiColor::Rgb((r, g, b).into()).as_grayscale();
    crossterm::style::Color::AnsiValue(ansi.index)
}
