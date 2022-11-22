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

use r3bl_rs_utils_core::TuiColor;

pub fn from_crossterm_color(other: crossterm::style::Color) -> TuiColor {
    match other {
        crossterm::style::Color::Reset => TuiColor::Reset,
        crossterm::style::Color::Black => TuiColor::Black,
        crossterm::style::Color::DarkGrey => TuiColor::DarkGrey,
        crossterm::style::Color::Red => TuiColor::Red,
        crossterm::style::Color::DarkRed => TuiColor::DarkRed,
        crossterm::style::Color::Green => TuiColor::Green,
        crossterm::style::Color::DarkGreen => TuiColor::DarkGreen,
        crossterm::style::Color::Yellow => TuiColor::Yellow,
        crossterm::style::Color::DarkYellow => TuiColor::DarkYellow,
        crossterm::style::Color::Blue => TuiColor::Blue,
        crossterm::style::Color::DarkBlue => TuiColor::DarkBlue,
        crossterm::style::Color::Magenta => TuiColor::Magenta,
        crossterm::style::Color::DarkMagenta => TuiColor::DarkMagenta,
        crossterm::style::Color::Cyan => TuiColor::Cyan,
        crossterm::style::Color::DarkCyan => TuiColor::DarkCyan,
        crossterm::style::Color::White => TuiColor::White,
        crossterm::style::Color::Grey => TuiColor::Grey,
        crossterm::style::Color::Rgb { r, g, b } => TuiColor::Rgb { r, g, b },
        crossterm::style::Color::AnsiValue(u8) => TuiColor::AnsiValue(u8),
    }
}

pub fn to_crossterm_color(other: TuiColor) -> crossterm::style::Color {
    match other {
        TuiColor::Reset => crossterm::style::Color::Reset,
        TuiColor::Black => crossterm::style::Color::Black,
        TuiColor::DarkGrey => crossterm::style::Color::DarkGrey,
        TuiColor::Red => crossterm::style::Color::Red,
        TuiColor::DarkRed => crossterm::style::Color::DarkRed,
        TuiColor::Green => crossterm::style::Color::Green,
        TuiColor::DarkGreen => crossterm::style::Color::DarkGreen,
        TuiColor::Yellow => crossterm::style::Color::Yellow,
        TuiColor::DarkYellow => crossterm::style::Color::DarkYellow,
        TuiColor::Blue => crossterm::style::Color::Blue,
        TuiColor::DarkBlue => crossterm::style::Color::DarkBlue,
        TuiColor::Magenta => crossterm::style::Color::Magenta,
        TuiColor::DarkMagenta => crossterm::style::Color::DarkMagenta,
        TuiColor::Cyan => crossterm::style::Color::Cyan,
        TuiColor::DarkCyan => crossterm::style::Color::DarkCyan,
        TuiColor::White => crossterm::style::Color::White,
        TuiColor::Grey => crossterm::style::Color::Grey,
        TuiColor::Rgb { r, g, b } => crossterm::style::Color::Rgb { r, g, b },
        TuiColor::AnsiValue(u8) => crossterm::style::Color::AnsiValue(u8),
    }
}
