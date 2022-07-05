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
use crossterm::style::*;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// Wrapper for [Color]. This is used to serialize and deserialize [Color]s. And it [Deref]s to
/// [Color] for interchangeable use w/ [Color].
///
/// Docs:
/// 1. https://serde.rs/remote-derive.html
/// 2. https://riptutorial.com/rust/example/20152/implement-serialize-and-deserialize-for-a-type-in-a-different-crate
#[derive(Serialize, Deserialize)]
#[serde(remote = "Color")]
enum ColorDef {
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

  /// An RGB color. See [RGB color model](https://en.wikipedia.org/wiki/RGB_color_model) for more info.
  ///
  /// Most UNIX terminals and Windows 10 supported only.
  /// See [Platform-specific notes](enum.Color.html#platform-specific-notes) for more info.
  Rgb { r: u8, g: u8, b: u8 },

  /// An ANSI color. See [256 colors - cheat sheet](https://jonasjacek.github.io/colors/) for more info.
  ///
  /// Most UNIX terminals and Windows 10 supported only.
  /// See [Platform-specific notes](enum.Color.html#platform-specific-notes) for more info.
  AnsiValue(u8),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct TWColor {
  #[serde(with = "ColorDef")]
  color: Color,
}

impl Deref for TWColor {
  type Target = Color;
  fn deref(&self) -> &Self::Target {
    &self.color
  }
}

impl From<Color> for TWColor {
  fn from(color: Color) -> Self {
    TWColor { color }
  }
}

impl Debug for TWColor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self.color {
      Color::Rgb { r, g, b } => f.write_fmt(format_args!("{},{},{}", r, g, b)),
      color => write!(f, "{:?}", color),
    }
  }
}
