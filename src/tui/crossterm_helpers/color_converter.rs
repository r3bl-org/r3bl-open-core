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

use r3bl_rs_utils_core::TWColor;

pub fn from_crossterm_color(other: crossterm::style::Color) -> TWColor {
  match other {
    crossterm::style::Color::Reset => TWColor::Reset,
    crossterm::style::Color::Black => TWColor::Black,
    crossterm::style::Color::DarkGrey => TWColor::DarkGrey,
    crossterm::style::Color::Red => TWColor::Red,
    crossterm::style::Color::DarkRed => TWColor::DarkRed,
    crossterm::style::Color::Green => TWColor::Green,
    crossterm::style::Color::DarkGreen => TWColor::DarkGreen,
    crossterm::style::Color::Yellow => TWColor::Yellow,
    crossterm::style::Color::DarkYellow => TWColor::DarkYellow,
    crossterm::style::Color::Blue => TWColor::Blue,
    crossterm::style::Color::DarkBlue => TWColor::DarkBlue,
    crossterm::style::Color::Magenta => TWColor::Magenta,
    crossterm::style::Color::DarkMagenta => TWColor::DarkMagenta,
    crossterm::style::Color::Cyan => TWColor::Cyan,
    crossterm::style::Color::DarkCyan => TWColor::DarkCyan,
    crossterm::style::Color::White => TWColor::White,
    crossterm::style::Color::Grey => TWColor::Grey,
    crossterm::style::Color::Rgb { r, g, b } => TWColor::Rgb { r, g, b },
    crossterm::style::Color::AnsiValue(u8) => TWColor::AnsiValue(u8),
  }
}

pub fn to_crossterm_color(other: TWColor) -> crossterm::style::Color {
  match other {
    TWColor::Reset => crossterm::style::Color::Reset,
    TWColor::Black => crossterm::style::Color::Black,
    TWColor::DarkGrey => crossterm::style::Color::DarkGrey,
    TWColor::Red => crossterm::style::Color::Red,
    TWColor::DarkRed => crossterm::style::Color::DarkRed,
    TWColor::Green => crossterm::style::Color::Green,
    TWColor::DarkGreen => crossterm::style::Color::DarkGreen,
    TWColor::Yellow => crossterm::style::Color::Yellow,
    TWColor::DarkYellow => crossterm::style::Color::DarkYellow,
    TWColor::Blue => crossterm::style::Color::Blue,
    TWColor::DarkBlue => crossterm::style::Color::DarkBlue,
    TWColor::Magenta => crossterm::style::Color::Magenta,
    TWColor::DarkMagenta => crossterm::style::Color::DarkMagenta,
    TWColor::Cyan => crossterm::style::Color::Cyan,
    TWColor::DarkCyan => crossterm::style::Color::DarkCyan,
    TWColor::White => crossterm::style::Color::White,
    TWColor::Grey => crossterm::style::Color::Grey,
    TWColor::Rgb { r, g, b } => crossterm::style::Color::Rgb { r, g, b },
    TWColor::AnsiValue(u8) => crossterm::style::Color::AnsiValue(u8),
  }
}
