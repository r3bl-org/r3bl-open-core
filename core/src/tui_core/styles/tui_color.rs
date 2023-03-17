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

use get_size::GetSize;
use serde::{Deserialize, Serialize};

use crate::*;

#[macro_export]
macro_rules! color {
    (
        $arg_r : expr,
        $arg_g : expr,
        $arg_b : expr
    ) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8($arg_r, $arg_g, $arg_b))
    };

    (
        $arg_value : expr
    ) => {
        $crate::TuiColor::AnsiValue($arg_value)
    };

    (@reset) => {
        $crate::TuiColor::Reset
    };

    (@black) => {
        $crate::TuiColor::Black
    };

    (@dark_grey) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkGrey)
    };

    (@red) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Red)
    };

    (@dark_red) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkRed)
    };

    (@green) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Green)
    };

    (@dark_green) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkGreen)
    };

    (@yellow) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Yellow)
    };

    (@dark_yellow) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkYellow)
    };

    (@blue) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Blue)
    };

    (@dark_blue) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkBlue)
    };

    (@magenta) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Magenta)
    };

    (@dark_magenta) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkMagenta)
    };

    (@cyan) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Cyan)
    };

    (@dark_cyan) => {
        $crate::TuiColor::Basic(ANSIBasicColor::DarkCyan)
    };

    (@white) => {
        $crate::TuiColor::Basic(ANSIBasicColor::White)
    };

    (@grey) => {
        $crate::TuiColor::Basic(ANSIBasicColor::Grey)
    };
}

/// Please use the macro [color] to create a new [TuiColor] instances, instead of directly
/// manipulating this struct.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Copy, Hash, GetSize)]
pub enum TuiColor {
    /// Resets the terminal color.
    Reset,
    /// ANSI 16 basic colors.
    Basic(ANSIBasicColor),
    /// An RGB color. See [RGB color model](https://en.wikipedia.org/wiki/RGB_color_model) for more
    /// info.
    ///
    /// Most UNIX terminals and Windows 10 supported only. See [Platform-specific
    /// notes](enum.Color.html#platform-specific-notes) for more info.
    Rgb(RgbValue),
    /// An ANSI color. See [256 colors - cheat sheet](https://jonasjacek.github.io/colors/) for more
    /// info.
    ///
    /// Most UNIX terminals and Windows 10 supported only. See [Platform-specific
    /// notes](enum.Color.html#platform-specific-notes) for more info.
    Ansi(u8),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Copy, Hash, GetSize)]
pub enum ANSIBasicColor {
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
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, GetSize, Copy, Debug)]
pub struct RgbValue {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Default for RgbValue {
    fn default() -> Self { Self::from_u8(255, 255, 255) }
}

mod rgb_values_impl {
    use super::*;

    impl RgbValue {
        pub fn from_u8(red: u8, green: u8, blue: u8) -> Self { Self { red, green, blue } }

        pub fn from_f32(red: f32, green: f32, blue: f32) -> Self {
            Self {
                red: (red * 255.0) as u8,
                green: (green * 255.0) as u8,
                blue: (blue * 255.0) as u8,
            }
        }

        pub fn try_from_hex_color(input: &str) -> CommonResult<RgbValue> {
            match parse_hex_color(input) {
                Ok((_, color)) => Ok(color),
                Err(_) => {
                    CommonError::new_err_with_only_type(CommonErrorType::InvalidHexColorFormat)
                }
            }
        }

        pub fn from_hex(input: &str) -> RgbValue {
            match parse_hex_color(input) {
                Ok((_, color)) => color,
                Err(_) => {
                    panic!("Invalid hex color format: {}", input)
                }
            }
        }

        pub fn try_from_tui_color(color: TuiColor) -> CommonResult<Self> {
            match color {
                // RGB values.
                TuiColor::Rgb(it) => Ok(it),

                // ANSI Basic 16.
                TuiColor::Basic(basic_color) => {
                    match basic_color {
                        // ANSI values.
                        ANSIBasicColor::Black => Ok(RgbValue {
                            red: 0,
                            green: 0,
                            blue: 0,
                        }),
                        ANSIBasicColor::White => Ok(RgbValue {
                            red: 255,
                            green: 255,
                            blue: 255,
                        }),
                        ANSIBasicColor::Grey => Ok(RgbValue {
                            red: 128,
                            green: 128,
                            blue: 128,
                        }),
                        ANSIBasicColor::Red => Ok(RgbValue {
                            red: 255,
                            green: 0,
                            blue: 0,
                        }),
                        ANSIBasicColor::Green => Ok(RgbValue {
                            red: 0,
                            green: 255,
                            blue: 0,
                        }),
                        ANSIBasicColor::Blue => Ok(RgbValue {
                            red: 0,
                            green: 0,
                            blue: 255,
                        }),
                        ANSIBasicColor::Yellow => Ok(RgbValue {
                            red: 255,
                            green: 255,
                            blue: 0,
                        }),
                        ANSIBasicColor::Cyan => Ok(RgbValue {
                            red: 0,
                            green: 255,
                            blue: 255,
                        }),
                        ANSIBasicColor::Magenta => Ok(RgbValue {
                            red: 255,
                            green: 0,
                            blue: 255,
                        }),
                        ANSIBasicColor::DarkGrey => Ok(RgbValue {
                            red: 64,
                            green: 64,
                            blue: 64,
                        }),
                        ANSIBasicColor::DarkRed => Ok(RgbValue {
                            red: 128,
                            green: 0,
                            blue: 0,
                        }),
                        ANSIBasicColor::DarkGreen => Ok(RgbValue {
                            red: 0,
                            green: 128,
                            blue: 0,
                        }),
                        ANSIBasicColor::DarkBlue => Ok(RgbValue {
                            red: 0,
                            green: 0,
                            blue: 128,
                        }),
                        ANSIBasicColor::DarkYellow => Ok(RgbValue {
                            red: 128,
                            green: 128,
                            blue: 0,
                        }),
                        ANSIBasicColor::DarkMagenta => Ok(RgbValue {
                            red: 128,
                            green: 0,
                            blue: 128,
                        }),
                        ANSIBasicColor::DarkCyan => Ok(RgbValue {
                            red: 0,
                            green: 128,
                            blue: 128,
                        }),
                    }
                }

                _ => CommonError::new_err_with_only_type(CommonErrorType::InvalidRgbColor),
            }
        }
    }
}

#[cfg(test)]
mod test_rgb_value {
    use super::*;

    #[test]
    fn test_new() {
        let value = RgbValue::from_u8(1, 2, 3);
        assert_eq!((value.red, value.green, value.blue), (1, 2, 3));
    }

    #[test]
    fn test_try_from_hex_color() {
        // Valid.
        {
            let hex_color = "#ff0000";
            let value = RgbValue::try_from_hex_color(hex_color).unwrap();
            assert_eq!((value.red, value.green, value.blue), (255, 0, 0));
        }

        // Invalid.
        {
            let hex_color = "#ff000";
            let value = RgbValue::try_from_hex_color(hex_color);
            assert!(value.is_err());
        }
    }

    #[test]
    fn test_try_from_tui_color() {
        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Rgb(RgbValue::from_u8(1, 2, 3))).unwrap(),
            RgbValue {
                red: 1,
                green: 2,
                blue: 3
            }
        );

        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Black)).unwrap(),
            RgbValue {
                red: 0,
                green: 0,
                blue: 0
            }
        );

        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::White)).unwrap(),
            RgbValue {
                red: 255,
                green: 255,
                blue: 255
            }
        );

        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Grey)).unwrap(),
            RgbValue {
                red: 128,
                green: 128,
                blue: 128
            }
        );

        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Red)).unwrap(),
            RgbValue {
                red: 255,
                green: 0,
                blue: 0
            }
        );

        assert_eq!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Green)).unwrap(),
            RgbValue {
                red: 0,
                green: 255,
                blue: 0
            }
        );
    }
}

mod debug_helpers {
    use super::*;

    impl Debug for TuiColor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TuiColor::Rgb(RgbValue { red, green, blue }) => {
                    f.write_fmt(format_args!("{red},{green},{blue}"))
                }
                TuiColor::Ansi(value) => f.write_fmt(format_args!("ansi_value({value})")),
                TuiColor::Reset => f.write_fmt(format_args!("reset")),
                TuiColor::Basic(basic_color) => match basic_color {
                    ANSIBasicColor::Black => f.write_fmt(format_args!("black")),
                    ANSIBasicColor::DarkGrey => f.write_fmt(format_args!("dark_grey")),
                    ANSIBasicColor::Red => f.write_fmt(format_args!("red")),
                    ANSIBasicColor::DarkRed => f.write_fmt(format_args!("dark_red")),
                    ANSIBasicColor::Green => f.write_fmt(format_args!("green")),
                    ANSIBasicColor::DarkGreen => f.write_fmt(format_args!("dark_green")),
                    ANSIBasicColor::Yellow => f.write_fmt(format_args!("yellow")),
                    ANSIBasicColor::DarkYellow => f.write_fmt(format_args!("dark_yellow")),
                    ANSIBasicColor::Blue => f.write_fmt(format_args!("blue")),
                    ANSIBasicColor::DarkBlue => f.write_fmt(format_args!("dark_blue")),
                    ANSIBasicColor::Magenta => f.write_fmt(format_args!("magenta")),
                    ANSIBasicColor::DarkMagenta => f.write_fmt(format_args!("dark_magenta")),
                    ANSIBasicColor::Cyan => f.write_fmt(format_args!("cyan")),
                    ANSIBasicColor::DarkCyan => f.write_fmt(format_args!("dark_cyan")),
                    ANSIBasicColor::White => f.write_fmt(format_args!("white")),
                    ANSIBasicColor::Grey => f.write_fmt(format_args!("grey")),
                },
            }
        }
    }
}
