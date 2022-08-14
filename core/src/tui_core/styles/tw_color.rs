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

use core::fmt::Debug;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum TWColor {
  /// Resets the terminal color.
  Reset,

  /// Black color.
  Black,

  /// Dark grey color.
  DarkGrey,

  /// Light red color.
  Red,

  /// Dark red color.
  DarkRed,

  /// Light green color.
  Green,

  /// Dark green color.
  DarkGreen,

  /// Light yellow color.
  Yellow,

  /// Dark yellow color.
  DarkYellow,

  /// Light blue color.
  Blue,

  /// Dark blue color.
  DarkBlue,

  /// Light magenta color.
  Magenta,

  /// Dark magenta color.
  DarkMagenta,

  /// Light cyan color.
  Cyan,

  /// Dark cyan color.
  DarkCyan,

  /// White color.
  White,

  /// Grey color.
  Grey,

  /// An RGB color. See [RGB color model](https://en.wikipedia.org/wiki/RGB_color_model) for more
  /// info.
  ///
  /// Most UNIX terminals and Windows 10 supported only. See [Platform-specific
  /// notes](enum.Color.html#platform-specific-notes) for more info.
  Rgb { r: u8, g: u8, b: u8 },

  /// An ANSI color. See [256 colors - cheat sheet](https://jonasjacek.github.io/colors/) for more
  /// info.
  ///
  /// Most UNIX terminals and Windows 10 supported only. See [Platform-specific
  /// notes](enum.Color.html#platform-specific-notes) for more info.
  AnsiValue(u8),
}

/// Convert from [crossterm::style::color::Color] to [TWColor].
impl From<crossterm::style::Color> for TWColor {
  fn from(other: crossterm::style::Color) -> Self {
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
}

/// Convert from [TWColor] to [crossterm::style::color::Color].
impl From<TWColor> for crossterm::style::Color {
  fn from(other: TWColor) -> Self {
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
}

impl Debug for TWColor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TWColor::Rgb { r, g, b } => f.write_fmt(format_args!("{},{},{}", r, g, b)),
      color => write!(f, "{:?}", color),
    }
  }
}
