// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::parse_hex_color;
use crate::{ASTColor, LossyConvertToByte, TransformColor,
            ansi_constants::ANSI_COLOR_PALETTE,
            color_utils,
            common::{CommonError, CommonErrorType, CommonResult},
            convert_rgb_into_ansi256};
use core::fmt::Debug;

/// Creates a [`TuiColor`] instance using various convenient syntaxes.
///
/// # Usage
///
/// ```rust
/// use r3bl_tui::tui_color;
///
/// // Named colors
/// let red = tui_color!(red);
/// let lizard_green = tui_color!(lizard_green);
///
/// // RGB values
/// let custom = tui_color!(255, 128, 0);
///
/// // ANSI color codes
/// let ansi = tui_color!(ansi 42);
///
/// // Hex colors (note: will panic on invalid format)
/// let hex = tui_color!(hex "#ff8000");
/// ```
///
/// # Panics
///
/// The `hex` variant will panic if the provided hex color string is not in a valid
/// format. Valid formats include: `#RGB`, `#RRGGBB`. Examples of invalid formats that
/// will panic:
/// - `#ff000` (5 characters instead of 6)
/// - `"gggggg"` (missing # prefix)
/// - `#zzzzzz` (invalid hex characters)
///
/// For fallible hex color parsing, use [`RgbValue::try_from_hex_color`] instead.
///
/// [`TuiColor`]: crate::TuiColor
/// [`RgbValue::try_from_hex_color`]: crate::RgbValue::try_from_hex_color
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

/// Please use the macro [`crate::tui_color`!] to create a new [`TuiColor`] instances,
/// instead of directly manipulating this struct.
///
/// A [`TuiColor`] can be [`RgbValue`], [`AnsiValue`], or [`ANSIBasicColor`].
/// - It is safe to use just `RgbValue` since the library will degrade gracefully to ANSI
///   256 or grayscale based on terminal emulator capabilities at runtime, as determined
///   by [`ColorSupport`].
/// - If a color is specified as [`AnsiValue`] or [`ANSIBasicColor`] then it will not be
///   downgraded.
///
/// [`TuiColor`]: crate::TuiColor
/// [`RgbValue`]: crate::RgbValue
/// [`AnsiValue`]: crate::AnsiValue
/// [`ANSIBasicColor`]: crate::ANSIBasicColor
/// [`ColorSupport`]: crate::ColorSupport
#[derive(Clone, PartialEq, Eq, Copy, Hash)]
pub enum TuiColor {
    /// Resets the terminal color.
    Reset,
    /// ANSI 16 basic colors.
    Basic(ANSIBasicColor),
    /// An RGB color. See [RGB color model] for more info.
    ///
    /// Most UNIX terminals and Windows 10 supported only.
    ///
    /// [RGB color model]: https://en.wikipedia.org/wiki/RGB_color_model
    Rgb(RgbValue),
    /// An ANSI color. See [256 colors - cheat sheet] for more info.
    ///
    /// Most UNIX terminals and Windows 10 supported only.
    ///
    /// [256 colors - cheat sheet]: https://jonasjacek.github.io/colors/
    Ansi(AnsiValue),
}

#[derive(Clone, PartialEq, Eq, Copy, Hash, Debug)]
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

mod convenience_conversions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ANSIBasicColor> for TuiColor {
        fn from(basic_color: ANSIBasicColor) -> Self { TuiColor::Basic(basic_color) }
    }

    impl From<RgbValue> for TuiColor {
        fn from(rgb_value: RgbValue) -> Self { TuiColor::Rgb(rgb_value) }
    }

    impl From<AnsiValue> for TuiColor {
        /// Convert a [`AnsiValue`] (256-color palette index or SGR code) to [`TuiColor`].
        ///
        /// This implementation handles two cases:
        ///
        /// 1. **SGR Color Codes (0-107)**: Basic ANSI color codes that are converted to
        ///    [`TuiColor::Basic`] variants for standard 16-color terminal support.
        ///    - Standard colors: 30-37 (foreground), 40-47 (background)
        ///    - Bright colors: 90-97 (bright foreground), 100-107 (bright background)
        ///
        /// 2. **Palette Indices (0-255)**: Other values are treated as 256-color palette
        ///    indices and wrapped in [`TuiColor::Ansi`].
        ///
        /// # Examples
        ///
        /// ```
        /// use r3bl_tui::{TuiColor, AnsiValue};
        ///
        /// // SGR code 31 (red foreground) → Basic red
        /// let color = TuiColor::from(AnsiValue::new(31));
        /// assert!(matches!(color, TuiColor::Basic(_)));
        ///
        /// // Palette index 196 (bright red in 256-color) → Ansi color
        /// let color = TuiColor::from(AnsiValue::new(196));
        /// assert!(matches!(color, TuiColor::Ansi(_)));
        /// ```
        ///
        /// [`AnsiValue`]: crate::AnsiValue
        /// [`TuiColor`]: crate::TuiColor
        /// [`TuiColor::Basic`]: crate::TuiColor::Basic
        /// [`TuiColor::Ansi`]: crate::TuiColor::Ansi
        fn from(ansi_value: AnsiValue) -> Self {
            match ansi_value.index {
                // Standard foreground colors (30-37)
                30 | 40 => TuiColor::Basic(ANSIBasicColor::Black),
                31 | 41 => TuiColor::Basic(ANSIBasicColor::DarkRed),
                32 | 42 => TuiColor::Basic(ANSIBasicColor::DarkGreen),
                33 | 43 => TuiColor::Basic(ANSIBasicColor::DarkYellow),
                34 | 44 => TuiColor::Basic(ANSIBasicColor::DarkBlue),
                35 | 45 => TuiColor::Basic(ANSIBasicColor::DarkMagenta),
                36 | 46 => TuiColor::Basic(ANSIBasicColor::DarkCyan),
                37 | 47 => TuiColor::Basic(ANSIBasicColor::Gray),

                // Bright colors (90-97, 100-107)
                90 | 100 => TuiColor::Basic(ANSIBasicColor::DarkGray),
                91 | 101 => TuiColor::Basic(ANSIBasicColor::Red),
                92 | 102 => TuiColor::Basic(ANSIBasicColor::Green),
                93 | 103 => TuiColor::Basic(ANSIBasicColor::Yellow),
                94 | 104 => TuiColor::Basic(ANSIBasicColor::Blue),
                95 | 105 => TuiColor::Basic(ANSIBasicColor::Magenta),
                96 | 106 => TuiColor::Basic(ANSIBasicColor::Cyan),
                97 | 107 => TuiColor::Basic(ANSIBasicColor::White),

                // All other values: treat as 256-color palette indices
                _ => TuiColor::Ansi(ansi_value),
            }
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Copy, Debug)]
pub struct RgbValue {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

mod rgb_value_impl_block {
    use super::{ANSIBasicColor, AnsiValue, CommonError, CommonErrorType, CommonResult,
                LossyConvertToByte, RgbValue, TransformColor, TuiColor,
                convert_rgb_into_ansi256, parse_hex_color};

    impl From<(u8, u8, u8)> for RgbValue {
        fn from((red, green, blue): (u8, u8, u8)) -> Self {
            Self::from_u8(red, green, blue)
        }
    }

    impl RgbValue {
        #[must_use]
        pub fn from_u8(red: u8, green: u8, blue: u8) -> Self { Self { red, green, blue } }

        #[must_use]
        pub fn from_f32(red: f32, green: f32, blue: f32) -> Self {
            Self {
                red: (red * 255.0).to_u8_lossy(),
                green: (green * 255.0).to_u8_lossy(),
                blue: (blue * 255.0).to_u8_lossy(),
            }
        }

        /// # Errors
        ///
        /// Returns an error if the input string is not a valid hex color format.
        pub fn try_from_hex_color(input: &str) -> CommonResult<RgbValue> {
            match parse_hex_color(input) {
                Ok((_, color)) => Ok(color),
                Err(_) => CommonError::new_error_result_with_only_type(
                    CommonErrorType::InvalidHexColorFormat,
                ),
            }
        }

        /// # Panics
        ///
        /// This function will panic if the input string is not a valid hex color format.
        #[must_use]
        pub fn from_hex(input: &str) -> RgbValue {
            #[allow(clippy::match_wild_err_arm)]
            match parse_hex_color(input) {
                Ok((_, color)) => color,
                Err(_) => {
                    panic!("Invalid hex color format: {input}")
                }
            }
        }

        /// # Errors
        ///
        /// Returns an error if the `TuiColor` is an index-based color that cannot be
        /// converted to RGB.
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
    use super::{ANSI_COLOR_PALETTE, AnsiValue, RgbValue, TransformColor, color_utils,
                convert_rgb_into_ansi256};

    impl From<u8> for AnsiValue {
        fn from(index: u8) -> Self { Self { index } }
    }

    impl From<u16> for AnsiValue {
        fn from(value: u16) -> Self {
            debug_assert!(
                value <= 255,
                "AnsiValue must represent a valid 256-color palette index (0-255), got {}",
                value
            );
            Self { index: value as u8 }
        }
    }

    impl From<i32> for AnsiValue {
        fn from(value: i32) -> Self {
            debug_assert!(
                value >= 0 && value <= 255,
                "AnsiValue must represent a valid 256-color palette index (0-255), got {}",
                value
            );
            Self { index: value as u8 }
        }
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
    use super::{AnsiValue, RgbValue};

    impl Default for RgbValue {
        fn default() -> Self { Self::from_u8(255, 255, 255) }
    }

    impl AnsiValue {
        #[must_use]
        pub fn new(color: u8) -> Self { Self { index: color } }
    }
}

/// This is useful when you want to mix and match the two crates. For example, you can use
/// a nice color from `tui_color!(lizard_green)` and then convert it to an [`ASTColor`]
/// using [`ASTColor::from`]. So you're no longer limited to the basic colors
/// when using [`ASTColor`] in your code (which happens when generating colorized log
/// output).
///
/// [`ASTColor`]: crate::ASTColor
/// [`ASTColor::from`]: crate::ASTColor::from
mod convert_to_ast_color {
    use super::{ASTColor, RgbValue, TuiColor};

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

/// This is useful when you want to go between different variants of the [`TuiColor`]
/// enum.
///
/// [`TuiColor`]: crate::TuiColor
mod convert_between_variants {
    use super::{AnsiValue, RgbValue, TransformColor, TuiColor};

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

    /// <https://www.ditig.com/256-colors-cheat-sheet>
    /// ANSI: 57 `BlueViolet`
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_ansi_to_rgb() {
        let ansi = AnsiValue::new(57);
        let rgb = RgbValue::from(ansi);
        assert_eq2!(rgb, RgbValue::from_u8(95, 0, 255));
    }

    /// <https://www.ditig.com/256-colors-cheat-sheet>
    /// ANSI: 57 `BlueViolet`
    /// RGB: #5f00ff rgb(95,0,255)
    #[test]
    fn test_rgb_to_ansi() {
        let rgb = RgbValue::from_u8(95, 0, 255);
        let ansi = AnsiValue::from(rgb);
        assert_eq2!(ansi, AnsiValue::new(57));
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

    #[allow(clippy::too_many_lines)]
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

mod debug_helper {
    use super::{ANSIBasicColor, Debug, RgbValue, TuiColor};

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
