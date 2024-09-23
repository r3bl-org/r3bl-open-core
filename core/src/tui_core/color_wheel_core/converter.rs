/*
 *   Copyright (c) 2024 R3BL LLC
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

use crate::{ANSIBasicColor, RgbValue, TuiColor};

pub fn convert_tui_color_into_r3bl_ansi_color(color: TuiColor) -> r3bl_ansi_color::Color {
    match color {
        TuiColor::Rgb(RgbValue { red, green, blue }) => {
            r3bl_ansi_color::Color::Rgb(red, green, blue)
        }
        TuiColor::Ansi(ansi_value) => r3bl_ansi_color::Color::Ansi256(ansi_value.color),
        TuiColor::Basic(basic_color) => match basic_color {
            ANSIBasicColor::Black => r3bl_ansi_color::Color::Rgb(0, 0, 0),
            ANSIBasicColor::White => r3bl_ansi_color::Color::Rgb(255, 255, 255),
            ANSIBasicColor::Grey => r3bl_ansi_color::Color::Rgb(128, 128, 128),
            ANSIBasicColor::DarkGrey => r3bl_ansi_color::Color::Rgb(64, 64, 64),
            ANSIBasicColor::Red => r3bl_ansi_color::Color::Rgb(255, 0, 0),
            ANSIBasicColor::DarkRed => r3bl_ansi_color::Color::Rgb(128, 0, 0),
            ANSIBasicColor::Green => r3bl_ansi_color::Color::Rgb(0, 255, 0),
            ANSIBasicColor::DarkGreen => r3bl_ansi_color::Color::Rgb(0, 128, 0),
            ANSIBasicColor::Yellow => r3bl_ansi_color::Color::Rgb(255, 255, 0),
            ANSIBasicColor::DarkYellow => r3bl_ansi_color::Color::Rgb(128, 128, 0),
            ANSIBasicColor::Blue => r3bl_ansi_color::Color::Rgb(0, 0, 255),
            ANSIBasicColor::DarkBlue => r3bl_ansi_color::Color::Rgb(0, 0, 128),
            ANSIBasicColor::Magenta => r3bl_ansi_color::Color::Rgb(255, 0, 255),
            ANSIBasicColor::DarkMagenta => r3bl_ansi_color::Color::Rgb(128, 0, 128),
            ANSIBasicColor::Cyan => r3bl_ansi_color::Color::Rgb(0, 255, 255),
            ANSIBasicColor::DarkCyan => r3bl_ansi_color::Color::Rgb(0, 128, 128),
        },
        TuiColor::Reset => r3bl_ansi_color::Color::Rgb(0, 0, 0),
    }
}

#[cfg(test)]
mod tests_color_converter {
    use crate::{ANSIBasicColor, AnsiValue, RgbValue, TuiColor};

    use super::*;

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_rgb() {
        let tui_color = TuiColor::Rgb(RgbValue {
            red: 255,
            green: 0,
            blue: 0,
        });
        let expected_color = r3bl_ansi_color::Color::Rgb(255, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_ansi() {
        let tui_color = TuiColor::Ansi(AnsiValue { color: 42 });
        let expected_color = r3bl_ansi_color::Color::Ansi256(42);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_basic() {
        let tui_color = TuiColor::Basic(ANSIBasicColor::Red);
        let expected_color = r3bl_ansi_color::Color::Rgb(255, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_reset() {
        let tui_color = TuiColor::Reset;
        let expected_color = r3bl_ansi_color::Color::Rgb(0, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }
}
