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

#[macro_export]
macro_rules! color {
  (
    $arg_r : expr,
    $arg_g : expr,
    $arg_b : expr
  ) => {
    r3bl_rs_utils_core::TuiColor::Rgb {
      r: $arg_r,
      g: $arg_g,
      b: $arg_b,
    }
  };
  (
    $arg_value : expr
  ) => {
    r3bl_rs_utils_core::TuiColor::AnsiValue($arg_value)
  };
  (@reset) => {
    r3bl_rs_utils_core::TuiColor::Reset
  };
  (@black) => {
    r3bl_rs_utils_core::TuiColor::Black
  };
  (@dark_grey) => {
    r3bl_rs_utils_core::TuiColor::DarkGrey
  };
  (@red) => {
    r3bl_rs_utils_core::TuiColor::Red
  };
  (@dark_red) => {
    r3bl_rs_utils_core::TuiColor::DarkRed
  };
  (@green) => {
    r3bl_rs_utils_core::TuiColor::Green
  };
  (@dark_green) => {
    r3bl_rs_utils_core::TuiColor::DarkGreen
  };
  (@yellow) => {
    r3bl_rs_utils_core::TuiColor::Yellow
  };
  (@dark_yellow) => {
    r3bl_rs_utils_core::TuiColor::DarkYellow
  };
  (@blue) => {
    r3bl_rs_utils_core::TuiColor::Blue
  };
  (@dark_blue) => {
    r3bl_rs_utils_core::TuiColor::DarkBlue
  };
  (@magenta) => {
    r3bl_rs_utils_core::TuiColor::Magenta
  };
  (@dark_magenta) => {
    r3bl_rs_utils_core::TuiColor::DarkMagenta
  };
  (@cyan) => {
    r3bl_rs_utils_core::TuiColor::Cyan
  };
  (@dark_cyan) => {
    r3bl_rs_utils_core::TuiColor::DarkCyan
  };
  (@white) => {
    r3bl_rs_utils_core::TuiColor::White
  };
  (@grey) => {
    r3bl_rs_utils_core::TuiColor::Grey
  };
}

/// Please use the macro [color] to create a new [TuiColor] instances, instead of directly
/// manipulating this struct.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Copy, Hash)]
pub enum TuiColor {
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

mod helpers {
  use super::*;

  impl Debug for TuiColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        TuiColor::Rgb { r, g, b } => f.write_fmt(format_args!("{r},{g},{b}")),
        TuiColor::AnsiValue(value) => f.write_fmt(format_args!("ansi_value({value})")),
        TuiColor::Reset => f.write_fmt(format_args!("reset")),
        TuiColor::Black => f.write_fmt(format_args!("black")),
        TuiColor::DarkGrey => f.write_fmt(format_args!("dark_grey")),
        TuiColor::Red => f.write_fmt(format_args!("red")),
        TuiColor::DarkRed => f.write_fmt(format_args!("dark_red")),
        TuiColor::Green => f.write_fmt(format_args!("green")),
        TuiColor::DarkGreen => f.write_fmt(format_args!("dark_green")),
        TuiColor::Yellow => f.write_fmt(format_args!("yellow")),
        TuiColor::DarkYellow => f.write_fmt(format_args!("dark_yellow")),
        TuiColor::Blue => f.write_fmt(format_args!("blue")),
        TuiColor::DarkBlue => f.write_fmt(format_args!("dark_blue")),
        TuiColor::Magenta => f.write_fmt(format_args!("magenta")),
        TuiColor::DarkMagenta => f.write_fmt(format_args!("dark_magenta")),
        TuiColor::Cyan => f.write_fmt(format_args!("cyan")),
        TuiColor::DarkCyan => f.write_fmt(format_args!("dark_cyan")),
        TuiColor::White => f.write_fmt(format_args!("white")),
        TuiColor::Grey => f.write_fmt(format_args!("grey")),
      }
    }
  }
}
