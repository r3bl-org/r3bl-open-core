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

#[macro_export]
macro_rules! color {
  ($r:expr, $g:expr, $b:expr) => {
    TWColor::Rgb {
      r: $r,
      g: $g,
      b: $b,
    }
  };
  ($value:expr) => {
    TWColor::AnsiValue($value)
  };
  (@reset) => {
    TWColor::Reset
  };
  (@black) => {
    TWColor::Black
  };
  (@dark_grey) => {
    TWColor::DarkGrey
  };
  (@red) => {
    TWColor::Red
  };
  (@dark_red) => {
    TWColor::DarkRed
  };
  (@green) => {
    TWColor::Green
  };
  (@dark_green) => {
    TWColor::DarkGreen
  };
  (@yellow) => {
    TWColor::Yellow
  };
  (@dark_yellow) => {
    TWColor::DarkYellow
  };
  (@blue) => {
    TWColor::Blue
  };
  (@dark_blue) => {
    TWColor::DarkBlue
  };
  (@magenta) => {
    TWColor::Magenta
  };
  (@dark_magenta) => {
    TWColor::DarkMagenta
  };
  (@cyan) => {
    TWColor::Cyan
  };
  (@dark_cyan) => {
    TWColor::DarkCyan
  };
  (@white) => {
    TWColor::White
  };
  (@grey) => {
    TWColor::Grey
  };
}

mod helpers {
  use super::*;

  impl Debug for TWColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        TWColor::Rgb { r, g, b } => f.write_fmt(format_args!("{},{},{}", r, g, b)),
        color => write!(f, "{:?}", color),
      }
    }
  }
}
