// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{TuiColor, degrade_color, global_color_support};
use crossterm::style::Color;

/// Respect the color support of the terminal and downgrade the color if needed.
///
/// This converter uses the [`degrade_color`] function to ensure consistent color
/// degradation across the codebase. The degradation algorithm respects terminal
/// capabilities and handles all color variants (Basic ANSI, Extended ANSI, and RGB).
///
/// # Color Degradation
///
/// - **Truecolor terminals**: Colors are passed through unchanged
/// - **Ansi256 terminals**: RGB colors are converted to nearest ANSI256 palette color
/// - **Grayscale terminals**: All colors are converted to grayscale equivalents
/// - **`NoColor` terminals**: All colors are converted to terminal default
///
/// For detailed information about the degradation algorithm, see [`degrade_color`].
impl From<TuiColor> for crossterm::style::Color {
    fn from(color: TuiColor) -> Self {
        let color_support = global_color_support::detect();
        let degraded_color = degrade_color(color, color_support);

        // Convert the degraded_color TuiColor to crossterm format
        match degraded_color {
            TuiColor::Ansi(ansi) if ansi.is_basic() => {
                // Basic colors (palette indices 0-15) - map to crossterm colors
                match ansi.index {
                    0 => Color::Black,
                    1 => Color::DarkRed,
                    2 => Color::DarkGreen,
                    3 => Color::DarkYellow,
                    4 => Color::DarkBlue,
                    5 => Color::DarkMagenta,
                    6 => Color::DarkCyan,
                    7 => Color::White,
                    8 => Color::DarkGrey,
                    9 => Color::Red,
                    10 => Color::Green,
                    11 => Color::Yellow,
                    12 => Color::Blue,
                    13 => Color::Magenta,
                    14 => Color::Cyan,
                    15 => Color::Grey,
                    _ => unreachable!("index < 16 is guaranteed"),
                }
            }

            TuiColor::Ansi(ansi) => {
                // Extended colors (indices 16-255)
                Color::AnsiValue(ansi.index)
            }

            TuiColor::Rgb(rgb) => {
                // Truecolor (this should only happen on Truecolor terminals)
                Color::Rgb {
                    r: rgb.red,
                    g: rgb.green,
                    b: rgb.blue,
                }
            }
        }
    }
}

/// Convert from [`crossterm::style::Color`] to [`TuiColor`].
impl From<crossterm::style::Color> for TuiColor {
    fn from(crossterm_color: crossterm::style::Color) -> Self {
        match crossterm_color {
            Color::Rgb { r, g, b } => TuiColor::Rgb((r, g, b).into()),
            Color::AnsiValue(val) => TuiColor::Ansi(val.into()),
            // Map standard crossterm colors to ANSI basic colors (0-15)
            Color::Black | Color::Reset => TuiColor::Ansi(0.into()),
            Color::Red => TuiColor::Ansi(1.into()),
            Color::Green => TuiColor::Ansi(2.into()),
            Color::Yellow => TuiColor::Ansi(3.into()),
            Color::Blue => TuiColor::Ansi(4.into()),
            Color::Magenta => TuiColor::Ansi(5.into()),
            Color::Cyan => TuiColor::Ansi(6.into()),
            Color::White => TuiColor::Ansi(7.into()),
            Color::DarkGrey => TuiColor::Ansi(8.into()),
            Color::DarkRed => TuiColor::Ansi(9.into()),
            Color::DarkGreen => TuiColor::Ansi(10.into()),
            Color::DarkYellow => TuiColor::Ansi(11.into()),
            Color::DarkBlue => TuiColor::Ansi(12.into()),
            Color::DarkMagenta => TuiColor::Ansi(13.into()),
            Color::DarkCyan => TuiColor::Ansi(14.into()),
            Color::Grey => TuiColor::Ansi(15.into()),
        }
    }
}
