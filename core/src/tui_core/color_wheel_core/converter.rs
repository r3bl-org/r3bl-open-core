/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use crate::{ANSIBasicColor, ASTColor, RgbValue, TuiColor};

pub fn convert_tui_color_into_r3bl_ansi_color(color: TuiColor) -> ASTColor {
    match color {
        TuiColor::Rgb(RgbValue { red, green, blue }) => ASTColor::Rgb(red, green, blue),
        TuiColor::Ansi(ansi_value) => ASTColor::Ansi256(ansi_value.color),
        TuiColor::Basic(basic_color) => match basic_color {
            ANSIBasicColor::Black => ASTColor::Rgb(0, 0, 0),
            ANSIBasicColor::White => ASTColor::Rgb(255, 255, 255),
            ANSIBasicColor::Grey => ASTColor::Rgb(128, 128, 128),
            ANSIBasicColor::DarkGrey => ASTColor::Rgb(64, 64, 64),
            ANSIBasicColor::Red => ASTColor::Rgb(255, 0, 0),
            ANSIBasicColor::DarkRed => ASTColor::Rgb(128, 0, 0),
            ANSIBasicColor::Green => ASTColor::Rgb(0, 255, 0),
            ANSIBasicColor::DarkGreen => ASTColor::Rgb(0, 128, 0),
            ANSIBasicColor::Yellow => ASTColor::Rgb(255, 255, 0),
            ANSIBasicColor::DarkYellow => ASTColor::Rgb(128, 128, 0),
            ANSIBasicColor::Blue => ASTColor::Rgb(0, 0, 255),
            ANSIBasicColor::DarkBlue => ASTColor::Rgb(0, 0, 128),
            ANSIBasicColor::Magenta => ASTColor::Rgb(255, 0, 255),
            ANSIBasicColor::DarkMagenta => ASTColor::Rgb(128, 0, 128),
            ANSIBasicColor::Cyan => ASTColor::Rgb(0, 255, 255),
            ANSIBasicColor::DarkCyan => ASTColor::Rgb(0, 128, 128),
        },
        TuiColor::Reset => ASTColor::default(),
    }
}

#[cfg(test)]
mod tests_color_converter {
    use super::*;
    use crate::tui_color;

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_rgb() {
        let tui_color = tui_color!(255, 0, 0);
        let expected_color = ASTColor::Rgb(255, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_ansi() {
        let tui_color = tui_color!(ansi 42);
        let expected_color = ASTColor::Ansi256(42);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_basic() {
        let tui_color = tui_color!(red);
        let expected_color = ASTColor::Rgb(255, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }

    #[test]
    fn test_convert_tui_color_into_r3bl_ansi_color_reset() {
        let tui_color = tui_color!(reset);
        let expected_color = ASTColor::Rgb(0, 0, 0);
        let converted_color = convert_tui_color_into_r3bl_ansi_color(tui_color);
        assert_eq!(converted_color, expected_color);
    }
}
