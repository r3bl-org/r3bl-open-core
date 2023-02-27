/*
 *   Copyright (c) 2022 R3BL LLC
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

pub fn from_crossterm_color(value: crossterm::style::Color) -> TuiColor {
    match value {
        crossterm::style::Color::Reset => TuiColor::Reset,
        crossterm::style::Color::Black => TuiColor::Basic(ANSIBasicColor::Black),
        crossterm::style::Color::DarkGrey => TuiColor::Basic(ANSIBasicColor::DarkGrey),
        crossterm::style::Color::Red => TuiColor::Basic(ANSIBasicColor::Red),
        crossterm::style::Color::DarkRed => TuiColor::Basic(ANSIBasicColor::DarkRed),
        crossterm::style::Color::Green => TuiColor::Basic(ANSIBasicColor::Green),
        crossterm::style::Color::DarkGreen => TuiColor::Basic(ANSIBasicColor::DarkGreen),
        crossterm::style::Color::Yellow => TuiColor::Basic(ANSIBasicColor::Yellow),
        crossterm::style::Color::DarkYellow => TuiColor::Basic(ANSIBasicColor::DarkYellow),
        crossterm::style::Color::Blue => TuiColor::Basic(ANSIBasicColor::Blue),
        crossterm::style::Color::DarkBlue => TuiColor::Basic(ANSIBasicColor::DarkBlue),
        crossterm::style::Color::Magenta => TuiColor::Basic(ANSIBasicColor::Magenta),
        crossterm::style::Color::DarkMagenta => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
        crossterm::style::Color::Cyan => TuiColor::Basic(ANSIBasicColor::Cyan),
        crossterm::style::Color::DarkCyan => TuiColor::Basic(ANSIBasicColor::DarkCyan),
        crossterm::style::Color::White => TuiColor::Basic(ANSIBasicColor::White),
        crossterm::style::Color::Grey => TuiColor::Basic(ANSIBasicColor::Grey),
        crossterm::style::Color::Rgb { r, g, b } => TuiColor::Rgb(RgbValue {
            red: r,
            green: g,
            blue: b,
        }),
        crossterm::style::Color::AnsiValue(u8) => TuiColor::Ansi(u8),
    }
}

pub fn to_crossterm_color(value: TuiColor) -> crossterm::style::Color {
    match value {
        TuiColor::Reset => crossterm::style::Color::Reset,
        TuiColor::Basic(basic_color) => match basic_color {
            ANSIBasicColor::Black => crossterm::style::Color::Black,
            ANSIBasicColor::DarkGrey => crossterm::style::Color::DarkGrey,
            ANSIBasicColor::Red => crossterm::style::Color::Red,
            ANSIBasicColor::DarkRed => crossterm::style::Color::DarkRed,
            ANSIBasicColor::Green => crossterm::style::Color::Green,
            ANSIBasicColor::DarkGreen => crossterm::style::Color::DarkGreen,
            ANSIBasicColor::Yellow => crossterm::style::Color::Yellow,
            ANSIBasicColor::DarkYellow => crossterm::style::Color::DarkYellow,
            ANSIBasicColor::Blue => crossterm::style::Color::Blue,
            ANSIBasicColor::DarkBlue => crossterm::style::Color::DarkBlue,
            ANSIBasicColor::Magenta => crossterm::style::Color::Magenta,
            ANSIBasicColor::DarkMagenta => crossterm::style::Color::DarkMagenta,
            ANSIBasicColor::Cyan => crossterm::style::Color::Cyan,
            ANSIBasicColor::DarkCyan => crossterm::style::Color::DarkCyan,
            ANSIBasicColor::White => crossterm::style::Color::White,
            ANSIBasicColor::Grey => crossterm::style::Color::Grey,
        },
        TuiColor::Rgb(RgbValue {
            red: r,
            green: g,
            blue: b,
        }) => crossterm::style::Color::Rgb { r, g, b },
        TuiColor::Ansi(u8) => crossterm::style::Color::AnsiValue(u8),
    }
}
