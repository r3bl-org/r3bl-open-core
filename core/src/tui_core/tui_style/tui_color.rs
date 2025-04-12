/*
 *   Copyright (c) 2022-2025 R3BL LLC
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

use super::parse_hex_color;
use crate::{ANSI_COLOR_PALETTE,
            ASTColor,
            TransformColor,
            color_utils,
            common::{CommonError, CommonErrorType, CommonResult},
            convert_rgb_into_ansi256};

#[macro_export]
macro_rules! tui_color {
    (medium_gray) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(193, 193, 193))
    };

    (light_cyan) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(190, 253, 249))
    };

    (light_purple) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(219, 202, 232))
    };

    (deep_purple) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(62, 14, 74))
    };

    (soft_pink) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(255, 181, 234))
    };

    (hot_pink) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(255, 0, 214))
    };

    (light_yellow_green) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(229, 239, 123))
    };

    (light_cyan) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(209, 244, 255))
    };

    (light_gray) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(241, 241, 241))
    };

    (dark_teal) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(6, 41, 52))
    };

    (bright_cyan) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(19, 227, 255))
    };

    (dark_purple) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(51, 32, 66))
    };

    (sky_blue) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(117, 215, 236))
    };

    (lavender) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(203, 170, 250))
    };

    (pink) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(195, 106, 138))
    };

    (dark_pink) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(203, 85, 121))
    };

    (dark_lizard_green) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(10, 122, 0))
    };

    (lizard_green) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(20, 244, 0))
    };

    (slate_gray) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(94, 103, 111))
    };

    (silver_metallic) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(213, 217, 220))
    };

    (frozen_blue) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(171, 204, 242))
    };

    (moonlight_blue) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(31, 36, 46))
    };

    (night_blue) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(14, 17, 23))
    };

    (guards_red) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(200, 1, 1))
    };

    (orange) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8(255, 132, 18))
    };

    (reset) => {
        $crate::TuiColor::Reset
    };

    (black) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Black)
    };

    (dark_gray) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkGray)
    };

    (red) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Red)
    };

    (dark_red) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkRed)
    };

    (green) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Green)
    };

    (dark_green) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkGreen)
    };

    (yellow) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Yellow)
    };

    (dark_yellow) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkYellow)
    };

    (blue) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Blue)
    };

    (dark_blue) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkBlue)
    };

    (magenta) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Magenta)
    };

    (dark_magenta) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkMagenta)
    };

    (cyan) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Cyan)
    };

    (dark_cyan) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::DarkCyan)
    };

    (white) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::White)
    };

    (gray) => {
        $crate::TuiColor::Basic($crate::ANSIBasicColor::Gray)
    };

    (
        hex $arg_hex : expr
    ) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_hex($arg_hex))
    };

    (
        ansi $arg_value : expr
    ) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new($arg_value))
    };

    (
        $arg_r : expr,
        $arg_g : expr,
        $arg_b : expr
        $(,)? /* optional trailing comma */
    ) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8($arg_r, $arg_g, $arg_b))
    };
}

/// Please use the macro [crate::tui_color!] to create a new [TuiColor] instances, instead
/// of directly manipulating this struct.
///
/// A [TuiColor] can be `RgbValue`, `AnsiValue`, or `ANSIBasicColor`.
/// - It is safe to use just `RgbValue` since the library will degrade gracefully to ANSI
///   256 or grayscale based on terminal emulator capabilities at runtime, which are
///   provided by
///   [`to_crossterm_color()`](https://docs.rs/r3bl_tui/latest/r3bl_tui/tui/terminal_lib_backends/color_converter/fn.to_crossterm_color.html)
///   and
///   [`ColorSupport`](https://docs.rs/r3bl_tui/latest/r3bl_tui/tui/color_wheel/detect_color_support/enum.ColorSupport.html).
/// - If a color is specified as `AnsiValue` or `ANSIBasicColor` then it will not be
///   downgraded.
#[derive(Clone, PartialEq, Eq, Copy, Hash)]
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
    Ansi(AnsiValue),
}

#[derive(Clone, PartialEq, Eq, Copy, Hash)]
pub enum ANSIBasicColor {
    /// Black color.
    Black,

    /// White color.
    White,

    /// Gray color.
    Gray,

    /// Dark gray color.
    DarkGray,

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
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub struct RgbValue {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

mod rgb_value_impl_block {
    use super::*;

    impl From<(u8, u8, u8)> for RgbValue {
        fn from((red, green, blue): (u8, u8, u8)) -> Self {
            Self::from_u8(red, green, blue)
        }
    }

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
                Err(_) => CommonError::new_error_result_with_only_type(
                    CommonErrorType::InvalidHexColorFormat,
                ),
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
                        ANSIBasicColor::Gray => Ok(RgbValue {
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
                        ANSIBasicColor::DarkGray => Ok(RgbValue {
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

                _ => CommonError::new_error_result_with_only_type(
                    CommonErrorType::InvalidValue,
                ),
            }
        }
    }

    impl TransformColor for RgbValue {
        fn as_rgb(&self) -> RgbValue { *self }

        fn as_ansi(&self) -> AnsiValue { convert_rgb_into_ansi256(*self) }

        fn as_grayscale(&self) -> AnsiValue {
            convert_rgb_into_ansi256(*self).as_grayscale()
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub struct AnsiValue {
    pub index: u8,
}

mod ansi_value_impl_block {
    use super::*;

    impl From<u8> for AnsiValue {
        fn from(index: u8) -> Self { Self { index } }
    }

    impl TransformColor for AnsiValue {
        fn as_grayscale(&self) -> AnsiValue {
            let index = self.index as usize;
            let rgb = ANSI_COLOR_PALETTE[index];
            let rgb = RgbValue::from(rgb);
            let rgb = color_utils::convert_grayscale((rgb.red, rgb.green, rgb.blue));
            convert_rgb_into_ansi256(RgbValue {
                red: rgb.0,
                green: rgb.1,
                blue: rgb.2,
            })
        }

        fn as_rgb(&self) -> RgbValue {
            let index = self.index as usize;
            ANSI_COLOR_PALETTE[index].into()
        }

        fn as_ansi(&self) -> AnsiValue { *self }
    }
}

mod construct {
    use super::*;

    impl Default for RgbValue {
        fn default() -> Self { Self::from_u8(255, 255, 255) }
    }

    impl AnsiValue {
        pub fn new(color: u8) -> Self { Self { index: color } }
    }
}

/// This is useful when you want to mix and match the two crates. For example, you can use
/// a nice color from `tui_color!(lizard_green)` and then convert it to an ASTColor using
/// `ASTColor::from(tui_color)`. So you're no longer limited to the basic colors when
/// using `ASTColor` in your code (which happens when generating colorized log output).
mod convert_to_ast_color {
    use super::*;

    impl From<TuiColor> for ASTColor {
        fn from(tui_color: TuiColor) -> Self {
            match tui_color {
                TuiColor::Rgb(rgb) => ASTColor::Rgb(rgb),
                TuiColor::Ansi(ansi) => ASTColor::Ansi(ansi),
                TuiColor::Basic(basic) => {
                    let rgb = RgbValue::try_from_tui_color(TuiColor::Basic(basic))
                        .unwrap_or_default();
                    ASTColor::Rgb(rgb)
                }
                TuiColor::Reset => ASTColor::default(),
            }
        }
    }

    impl From<ASTColor> for TuiColor {
        fn from(ast_color: ASTColor) -> Self {
            match ast_color {
                ASTColor::Rgb(rgb) => TuiColor::Rgb(rgb),
                ASTColor::Ansi(ansi) => TuiColor::Ansi(ansi),
            }
        }
    }
}

/// This is useful when you want to go between different variants of the [TuiColor] enum.
mod convert_between_variants {
    use super::*;

    impl From<RgbValue> for AnsiValue {
        fn from(rgb_value: RgbValue) -> Self {
            let rgb_color = crate::RgbValue {
                red: rgb_value.red,
                green: rgb_value.green,
                blue: rgb_value.blue,
            };
            let ansi_color = crate::convert_rgb_into_ansi256(rgb_color).index;
            Self::new(ansi_color)
        }
    }

    impl From<AnsiValue> for RgbValue {
        fn from(ansi_value: AnsiValue) -> Self {
            let rgb_color = crate::AnsiValue {
                index: ansi_value.index,
            }
            .as_rgb();
            let (red, green, blue) = (rgb_color.red, rgb_color.green, rgb_color.blue);
            Self::from_u8(red, green, blue)
        }
    }

    impl From<TuiColor> for RgbValue {
        fn from(tui_color: TuiColor) -> Self {
            match tui_color {
                TuiColor::Rgb(rgb) => rgb,
                TuiColor::Ansi(ansi) => RgbValue::from(ansi),
                TuiColor::Basic(basic) => {
                    RgbValue::try_from_tui_color(TuiColor::Basic(basic))
                        .unwrap_or_default()
                }
                TuiColor::Reset => RgbValue::default(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_convert_tui_color_to_ast_color() {
        {
            let tui_color = tui_color!(255, 0, 0);
            let expected_color = ASTColor::Rgb((255, 0, 0).into());
            let converted_color = ASTColor::from(tui_color);
            assert_eq!(converted_color, expected_color);
        }
        {
            let tui_color = tui_color!(ansi 42);
            let expected_color = ASTColor::Ansi(42.into());
            let converted_color = ASTColor::from(tui_color);
            assert_eq!(converted_color, expected_color);
        }
        {
            let tui_color = tui_color!(red);
            let expected_color = ASTColor::Rgb((255, 0, 0).into());
            let converted_color = ASTColor::from(tui_color);
            assert_eq!(converted_color, expected_color);
        }
        {
            let tui_color = tui_color!(reset);
            let expected_color = ASTColor::Rgb((0, 0, 0).into());
            let converted_color = ASTColor::from(tui_color);
            assert_eq!(converted_color, expected_color);
        }
    }

    /// https://www.ditig.com/256-colors-cheat-sheet
    /// ANSI: 57 BlueViolet
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_ansi_to_rgb() {
        let ansi = AnsiValue::new(57);
        let rgb = RgbValue::from(ansi);
        assert_eq2!(rgb, RgbValue::from_u8(95, 0, 255))
    }

    /// https://www.ditig.com/256-colors-cheat-sheet
    /// ANSI: 57 BlueViolet
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_rgb_to_ansi() {
        let rgb = RgbValue::from_u8(95, 0, 255);
        let ansi = AnsiValue::from(rgb);
        assert_eq2!(ansi, AnsiValue::new(57))
    }

    #[test]
    fn test_ansi_colors() {
        let color = tui_color!(ansi 42);
        assert_eq2!(color, TuiColor::Ansi(AnsiValue::new(42)));
    }

    #[test]
    fn test_new() {
        let value = RgbValue::from_u8(1, 2, 3);
        assert_eq2!((value.red, value.green, value.blue), (1, 2, 3));
    }

    #[test]
    fn test_try_from_hex_color() {
        // Valid.
        {
            let hex_color = "#ff0000";
            let value = RgbValue::try_from_hex_color(hex_color).unwrap();
            assert_eq2!((value.red, value.green, value.blue), (255, 0, 0));
        }

        // Invalid.
        {
            let hex_color = "#ff000";
            let value = RgbValue::try_from_hex_color(hex_color);
            assert!(value.is_err());
        }

        // Using macro.
        {
            let hex_color = "#ff0000";
            let value = tui_color!(hex hex_color);
            assert_eq2!(value, tui_color!(255, 0, 0));
        }
    }

    #[test]
    fn test_try_from_tui_color() {
        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Rgb(RgbValue::from_u8(1, 2, 3)))
                .unwrap(),
            RgbValue {
                red: 1,
                green: 2,
                blue: 3
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Black)).unwrap(),
            RgbValue {
                red: 0,
                green: 0,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::White)).unwrap(),
            RgbValue {
                red: 255,
                green: 255,
                blue: 255
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Gray)).unwrap(),
            RgbValue {
                red: 128,
                green: 128,
                blue: 128
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Red)).unwrap(),
            RgbValue {
                red: 255,
                green: 0,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(TuiColor::Basic(ANSIBasicColor::Green)).unwrap(),
            RgbValue {
                red: 0,
                green: 255,
                blue: 0
            }
        );
    }

    #[test]
    fn test_from_color_macro() {
        let black = tui_color!(black);
        let dark_gray = tui_color!(dark_gray);
        let red = tui_color!(red);
        let dark_red = tui_color!(dark_red);
        let green = tui_color!(green);
        let dark_green = tui_color!(dark_green);
        let yellow = tui_color!(yellow);
        let dark_yellow = tui_color!(dark_yellow);
        let blue = tui_color!(blue);
        let dark_blue = tui_color!(dark_blue);
        let magenta = tui_color!(magenta);
        let dark_magenta = tui_color!(dark_magenta);
        let cyan = tui_color!(cyan);
        let dark_cyan = tui_color!(dark_cyan);
        let white = tui_color!(white);
        let gray = tui_color!(gray);
        let reset = tui_color!(reset);

        let lizard_green = tui_color!(lizard_green);
        let slate_gray = tui_color!(slate_gray);
        let silver_metallic = tui_color!(silver_metallic);
        let frozen_blue = tui_color!(frozen_blue);
        let moonlight_blue = tui_color!(moonlight_blue);
        let night_blue = tui_color!(night_blue);
        let guards_red = tui_color!(guards_red);
        let orange = tui_color!(orange);

        assert_eq2!(
            RgbValue::try_from_tui_color(black).unwrap(),
            RgbValue {
                red: 0,
                green: 0,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_gray).unwrap(),
            RgbValue {
                red: 64,
                green: 64,
                blue: 64
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(red).unwrap(),
            RgbValue {
                red: 255,
                green: 0,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_red).unwrap(),
            RgbValue {
                red: 128,
                green: 0,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(green).unwrap(),
            RgbValue {
                red: 0,
                green: 255,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_green).unwrap(),
            RgbValue {
                red: 0,
                green: 128,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(yellow).unwrap(),
            RgbValue {
                red: 255,
                green: 255,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_yellow).unwrap(),
            RgbValue {
                red: 128,
                green: 128,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(blue).unwrap(),
            RgbValue {
                red: 0,
                green: 0,
                blue: 255
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_blue).unwrap(),
            RgbValue {
                red: 0,
                green: 0,
                blue: 128
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(magenta).unwrap(),
            RgbValue {
                red: 255,
                green: 0,
                blue: 255
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_magenta).unwrap(),
            RgbValue {
                red: 128,
                green: 0,
                blue: 128
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(cyan).unwrap(),
            RgbValue {
                red: 0,
                green: 255,
                blue: 255
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(dark_cyan).unwrap(),
            RgbValue {
                red: 0,
                green: 128,
                blue: 128
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(white).unwrap(),
            RgbValue {
                red: 255,
                green: 255,
                blue: 255
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(gray).unwrap(),
            RgbValue {
                red: 128,
                green: 128,
                blue: 128
            }
        );

        assert!(RgbValue::try_from_tui_color(reset).is_err());

        assert_eq2!(
            RgbValue::try_from_tui_color(lizard_green).unwrap(),
            RgbValue {
                red: 20,
                green: 244,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(slate_gray).unwrap(),
            RgbValue {
                red: 94,
                green: 103,
                blue: 111
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(silver_metallic).unwrap(),
            RgbValue {
                red: 213,
                green: 217,
                blue: 220
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(frozen_blue).unwrap(),
            RgbValue {
                red: 171,
                green: 204,
                blue: 242
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(moonlight_blue).unwrap(),
            RgbValue {
                red: 31,
                green: 36,
                blue: 46
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(night_blue).unwrap(),
            RgbValue {
                red: 14,
                green: 17,
                blue: 23
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(guards_red).unwrap(),
            RgbValue {
                red: 200,
                green: 1,
                blue: 1
            }
        );

        assert_eq2!(
            RgbValue::try_from_tui_color(orange).unwrap(),
            RgbValue {
                red: 255,
                green: 132,
                blue: 18
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
                    write!(f, "{red},{green},{blue}")
                }
                TuiColor::Ansi(ansi_value) => {
                    write!(f, "ansi_value({})", ansi_value.index)
                }
                TuiColor::Reset => write!(f, "reset"),
                TuiColor::Basic(basic_color) => match basic_color {
                    ANSIBasicColor::Black => write!(f, "black"),
                    ANSIBasicColor::DarkGray => write!(f, "dark_gray"),
                    ANSIBasicColor::Red => write!(f, "red"),
                    ANSIBasicColor::DarkRed => write!(f, "dark_red"),
                    ANSIBasicColor::Green => write!(f, "green"),
                    ANSIBasicColor::DarkGreen => write!(f, "dark_green"),
                    ANSIBasicColor::Yellow => write!(f, "yellow"),
                    ANSIBasicColor::DarkYellow => {
                        write!(f, "dark_yellow")
                    }
                    ANSIBasicColor::Blue => write!(f, "blue"),
                    ANSIBasicColor::DarkBlue => write!(f, "dark_blue"),
                    ANSIBasicColor::Magenta => write!(f, "magenta"),
                    ANSIBasicColor::DarkMagenta => {
                        write!(f, "dark_magenta")
                    }
                    ANSIBasicColor::Cyan => write!(f, "cyan"),
                    ANSIBasicColor::DarkCyan => write!(f, "dark_cyan"),
                    ANSIBasicColor::White => write!(f, "white"),
                    ANSIBasicColor::Gray => write!(f, "gray"),
                },
            }
        }
    }
}
