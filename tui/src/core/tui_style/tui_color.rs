// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ASTColor, AnsiValue, RgbValue, TransformColor, convert_rgb_into_ansi256};
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
    Black,
    White,
    Gray,
    DarkGray,
    Red,
    DarkRed,
    Green,
    DarkGreen,
    Yellow,
    DarkYellow,
    Blue,
    DarkBlue,
    Magenta,
    DarkMagenta,
    Cyan,
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
        /// Convert a [`AnsiValue`] to [`TuiColor`].
        ///
        /// SGR codes (0-107) are converted to [`TuiColor::Basic`] for 16-color support.
        /// Other values (108-255) are treated as 256-color palette indices and become
        /// [`TuiColor::Ansi`].
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

mod basic_color_conversions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ANSIBasicColor> for RgbValue {
        fn from(basic: ANSIBasicColor) -> Self {
            match basic {
                ANSIBasicColor::Black => RgbValue {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
                ANSIBasicColor::White => RgbValue {
                    red: 255,
                    green: 255,
                    blue: 255,
                },
                ANSIBasicColor::Gray => RgbValue {
                    red: 128,
                    green: 128,
                    blue: 128,
                },
                ANSIBasicColor::Red => RgbValue {
                    red: 255,
                    green: 0,
                    blue: 0,
                },
                ANSIBasicColor::Green => RgbValue {
                    red: 0,
                    green: 255,
                    blue: 0,
                },
                ANSIBasicColor::Blue => RgbValue {
                    red: 0,
                    green: 0,
                    blue: 255,
                },
                ANSIBasicColor::Yellow => RgbValue {
                    red: 255,
                    green: 255,
                    blue: 0,
                },
                ANSIBasicColor::Cyan => RgbValue {
                    red: 0,
                    green: 255,
                    blue: 255,
                },
                ANSIBasicColor::Magenta => RgbValue {
                    red: 255,
                    green: 0,
                    blue: 255,
                },
                ANSIBasicColor::DarkGray => RgbValue {
                    red: 64,
                    green: 64,
                    blue: 64,
                },
                ANSIBasicColor::DarkRed => RgbValue {
                    red: 128,
                    green: 0,
                    blue: 0,
                },
                ANSIBasicColor::DarkGreen => RgbValue {
                    red: 0,
                    green: 128,
                    blue: 0,
                },
                ANSIBasicColor::DarkBlue => RgbValue {
                    red: 0,
                    green: 0,
                    blue: 128,
                },
                ANSIBasicColor::DarkYellow => RgbValue {
                    red: 128,
                    green: 128,
                    blue: 0,
                },
                ANSIBasicColor::DarkMagenta => RgbValue {
                    red: 128,
                    green: 0,
                    blue: 128,
                },
                ANSIBasicColor::DarkCyan => RgbValue {
                    red: 0,
                    green: 128,
                    blue: 128,
                },
            }
        }
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
                    let rgb: RgbValue = basic.into();
                    ASTColor::Rgb(rgb)
                }
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<RgbValue> for AnsiValue {
        fn from(rgb_value: RgbValue) -> Self {
            let ansi_value = convert_rgb_into_ansi256(rgb_value);
            Self::new(ansi_value.index)
        }
    }

    impl From<AnsiValue> for RgbValue {
        fn from(ansi_value: AnsiValue) -> Self {
            let rgb_color = ansi_value.as_rgb();
            Self::from_u8(rgb_color.red, rgb_color.green, rgb_color.blue)
        }
    }

    impl From<TuiColor> for RgbValue {
        fn from(tui_color: TuiColor) -> Self {
            match tui_color {
                TuiColor::Rgb(rgb) => rgb,
                TuiColor::Ansi(ansi) => ansi.as_rgb(),
                TuiColor::Basic(basic) => basic.into(),
            }
        }
    }
}

mod impl_debug {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;
    use test_case::test_case;

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
    }

    #[test]
    fn test_ansi_colors() {
        let color = tui_color!(ansi 42);
        assert_eq2!(color, TuiColor::Ansi(AnsiValue::new(42)));
    }

    #[test]
    fn test_rgb_passthrough() {
        assert_eq2!(
            RgbValue::from(TuiColor::Rgb(RgbValue::from_u8(1, 2, 3))),
            RgbValue {
                red: 1,
                green: 2,
                blue: 3
            }
        );
    }

    #[test_case(ANSIBasicColor::Black, RgbValue { red: 0, green: 0, blue: 0 })]
    #[test_case(ANSIBasicColor::White, RgbValue { red: 255, green: 255, blue: 255 })]
    #[test_case(ANSIBasicColor::Gray, RgbValue { red: 128, green: 128, blue: 128 })]
    #[test_case(ANSIBasicColor::Red, RgbValue { red: 255, green: 0, blue: 0 })]
    #[test_case(ANSIBasicColor::Green, RgbValue { red: 0, green: 255, blue: 0 })]
    fn test_basic_color_to_rgb(color: ANSIBasicColor, expected: RgbValue) {
        assert_eq2!(RgbValue::from(TuiColor::Basic(color)), expected);
    }

    #[test_case(ANSIBasicColor::Black, RgbValue { red: 0, green: 0, blue: 0 })]
    #[test_case(ANSIBasicColor::DarkGray, RgbValue { red: 64, green: 64, blue: 64 })]
    #[test_case(ANSIBasicColor::Red, RgbValue { red: 255, green: 0, blue: 0 })]
    #[test_case(ANSIBasicColor::DarkRed, RgbValue { red: 128, green: 0, blue: 0 })]
    #[test_case(ANSIBasicColor::Green, RgbValue { red: 0, green: 255, blue: 0 })]
    #[test_case(ANSIBasicColor::DarkGreen, RgbValue { red: 0, green: 128, blue: 0 })]
    #[test_case(ANSIBasicColor::Yellow, RgbValue { red: 255, green: 255, blue: 0 })]
    #[test_case(ANSIBasicColor::DarkYellow, RgbValue { red: 128, green: 128, blue: 0 })]
    #[test_case(ANSIBasicColor::Blue, RgbValue { red: 0, green: 0, blue: 255 })]
    #[test_case(ANSIBasicColor::DarkBlue, RgbValue { red: 0, green: 0, blue: 128 })]
    #[test_case(ANSIBasicColor::Magenta, RgbValue { red: 255, green: 0, blue: 255 })]
    #[test_case(ANSIBasicColor::DarkMagenta, RgbValue { red: 128, green: 0, blue: 128 })]
    #[test_case(ANSIBasicColor::Cyan, RgbValue { red: 0, green: 255, blue: 255 })]
    #[test_case(ANSIBasicColor::DarkCyan, RgbValue { red: 0, green: 128, blue: 128 })]
    #[test_case(ANSIBasicColor::White, RgbValue { red: 255, green: 255, blue: 255 })]
    #[test_case(ANSIBasicColor::Gray, RgbValue { red: 128, green: 128, blue: 128 })]
    fn test_basic_colors_macro(color: ANSIBasicColor, expected: RgbValue) {
        assert_eq2!(RgbValue::from(TuiColor::Basic(color)), expected);
    }

    #[test]
    fn test_custom_colors_macro() {
        assert_eq2!(
            RgbValue::from(tui_color!(lizard_green)),
            RgbValue {
                red: 20,
                green: 244,
                blue: 0
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(slate_gray)),
            RgbValue {
                red: 94,
                green: 103,
                blue: 111
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(silver_metallic)),
            RgbValue {
                red: 213,
                green: 217,
                blue: 220
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(frozen_blue)),
            RgbValue {
                red: 171,
                green: 204,
                blue: 242
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(moonlight_blue)),
            RgbValue {
                red: 31,
                green: 36,
                blue: 46
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(night_blue)),
            RgbValue {
                red: 14,
                green: 17,
                blue: 23
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(guards_red)),
            RgbValue {
                red: 200,
                green: 1,
                blue: 1
            }
        );

        assert_eq2!(
            RgbValue::from(tui_color!(orange)),
            RgbValue {
                red: 255,
                green: 132,
                blue: 18
            }
        );
    }

    #[test]
    fn test_ansi_to_rgb_conversion() {
        // ANSI color 42 is rgb(0, 215, 135) per the 256-color palette
        let ansi_color = tui_color!(ansi 42);
        let rgb: RgbValue = ansi_color.into();
        assert_eq2!(rgb, RgbValue::from_u8(0, 215, 135));
    }
}
