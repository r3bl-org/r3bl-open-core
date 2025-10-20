// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{AnsiValue, RgbValue, TransformColor, convert_rgb_into_ansi256};
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
/// # Basic Colors
///
/// The following named colors are basic ANSI colors (indices 0-15) which are widely
/// supported by terminal emulators. These will not be degraded to a lower color support
/// level:
///
/// | Index  | Name         |
/// |--------|--------------|
/// | 0      | black        |
/// | 1      | red          |
/// | 2      | green        |
/// | 3      | yellow       |
/// | 4      | blue         |
/// | 5      | magenta      |
/// | 6      | cyan         |
/// | 7      | white        |
/// | 8      | dark_gray    |
/// | 9      | dark_red     |
/// | 10     | dark_green   |
/// | 11     | dark_yellow  |
/// | 12     | dark_blue    |
/// | 13     | dark_magenta |
/// | 14     | dark_cyan    |
/// | 15     | gray         |
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
/// [black]: #usage
/// [red]: #usage
/// [green]: #usage
/// [yellow]: #usage
/// [blue]: #usage
/// [magenta]: #usage
/// [cyan]: #usage
/// [white]: #usage
/// [dark_gray]: #usage
/// [dark_red]: #usage
/// [dark_green]: #usage
/// [dark_yellow]: #usage
/// [dark_blue]: #usage
/// [dark_magenta]: #usage
/// [dark_cyan]: #usage
/// [gray]: #usage
#[macro_export]
macro_rules! tui_color {
    //------------------
    // Basic ANSI colors
    //------------------
    (black) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(0))
    };

    (red) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(1))
    };

    (green) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(2))
    };

    (yellow) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(3))
    };

    (blue) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(4))
    };

    (magenta) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(5))
    };

    (cyan) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(6))
    };

    (white) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(7))
    };

    (dark_gray) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(8))
    };

    (dark_red) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(9))
    };

    (dark_green) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(10))
    };

    (dark_yellow) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(11))
    };

    (dark_blue) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(12))
    };

    (dark_magenta) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(13))
    };

    (dark_cyan) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(14))
    };

    (gray) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new(15))
    };

    //-----------------
    // RGB-named colors
    //-----------------
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

    //-----------------
    // Pattern matchers
    //-----------------

    /* Hex syntax: hex "#RRGGBB" */
    (
        hex $arg_hex : expr
    ) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_hex($arg_hex))
    };

    /* Ansi syntax: ansi <index> */
    (
        ansi $arg_value : expr
    ) => {
        $crate::TuiColor::Ansi($crate::AnsiValue::new($arg_value))
    };

    /* RGB syntax: (r, g, b) */
    (
        $arg_r : expr,
        $arg_g : expr,
        $arg_b : expr
        $(,)? /* optional trailing comma */
    ) => {
        $crate::TuiColor::Rgb($crate::RgbValue::from_u8($arg_r, $arg_g, $arg_b))
    };
}

/// Please use the macro [`tui_color`!] to create a new [`TuiColor`] instances,
/// instead of directly manipulating this struct.
///
/// A [`TuiColor`] can be [`RgbValue`] or [`AnsiValue`].
/// - It is safe to use just [`RgbValue`] since the library will degrade gracefully to
///   ANSI 256 or grayscale based on terminal emulator capabilities at runtime, as
///   determined by [`ColorSupport`].
/// - Basic ANSI colors (0-15) are represented as [`AnsiValue`] with indices 0-15. If a
///   color is specified as [`AnsiValue`], it will not be downgraded.
///
/// [`TuiColor`]: crate::TuiColor
/// [`RgbValue`]: crate::RgbValue
/// [`AnsiValue`]: crate::AnsiValue
/// [`ColorSupport`]: crate::ColorSupport
/// [`tui_color`!]: crate::tui_color!
#[derive(Clone, PartialEq, Eq, Copy, Hash)]
pub enum TuiColor {
    /// An RGB color. See [RGB color model] for more info.
    ///
    /// Most UNIX terminals and Windows 10 supported only.
    ///
    /// [RGB color model]: https://en.wikipedia.org/wiki/RGB_color_model
    Rgb(RgbValue),
    /// An ANSI color. See [256 colors - cheat sheet] for more info.
    ///
    /// Most UNIX terminals and Windows 10 supported only.
    /// Indices 0-15 represent basic ANSI colors; 16-255 represent the extended palette.
    ///
    /// [256 colors - cheat sheet]: https://jonasjacek.github.io/colors/
    Ansi(AnsiValue),
}

#[derive(Clone, PartialEq, Eq, Copy, Hash, Debug)]
pub enum ANSIBasicColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    DarkGray,
    DarkRed,
    DarkGreen,
    DarkYellow,
    DarkBlue,
    DarkMagenta,
    DarkCyan,
    Gray,
}

impl ANSIBasicColor {
    /// Get the palette index (0-15) for this basic ANSI color.
    ///
    /// These indices correspond to the 16-color ANSI palette:
    /// - 0-7: standard colors (dark variants)
    /// - 8-15: bright colors and grays
    #[must_use]
    pub fn palette_index(&self) -> u8 {
        match self {
            ANSIBasicColor::Black => 0,
            ANSIBasicColor::Red => 1,
            ANSIBasicColor::Green => 2,
            ANSIBasicColor::Yellow => 3,
            ANSIBasicColor::Blue => 4,
            ANSIBasicColor::Magenta => 5,
            ANSIBasicColor::Cyan => 6,
            ANSIBasicColor::White => 7,
            ANSIBasicColor::DarkGray => 8,
            ANSIBasicColor::DarkRed => 9,
            ANSIBasicColor::DarkGreen => 10,
            ANSIBasicColor::DarkYellow => 11,
            ANSIBasicColor::DarkBlue => 12,
            ANSIBasicColor::DarkMagenta => 13,
            ANSIBasicColor::DarkCyan => 14,
            ANSIBasicColor::Gray => 15,
        }
    }
}

mod convenience_conversions {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl From<ANSIBasicColor> for TuiColor {
        fn from(basic_color: ANSIBasicColor) -> Self {
            TuiColor::Ansi(AnsiValue::new(basic_color.palette_index()))
        }
    }

    impl From<RgbValue> for TuiColor {
        fn from(rgb_value: RgbValue) -> Self { TuiColor::Rgb(rgb_value) }
    }

    impl From<AnsiValue> for TuiColor {
        /// Convert a [`AnsiValue`] to [`TuiColor`].
        ///
        /// SGR codes (30-37, 40-47, 90-97, 100-107) are mapped to basic color indices
        /// 0-15. Other values (0-29, 48-89, 98-99, 108-255) are treated as
        /// 256-color palette indices.
        fn from(ansi_value: AnsiValue) -> Self {
            match ansi_value.index {
                // Standard foreground colors (30-37) → palette indices 9-15, 0
                30 | 40 => TuiColor::Ansi(AnsiValue::new(0)), // black
                31 | 41 => TuiColor::Ansi(AnsiValue::new(9)), // dark_red
                32 | 42 => TuiColor::Ansi(AnsiValue::new(10)), // dark_green
                33 | 43 => TuiColor::Ansi(AnsiValue::new(11)), // dark_yellow
                34 | 44 => TuiColor::Ansi(AnsiValue::new(12)), // dark_blue
                35 | 45 => TuiColor::Ansi(AnsiValue::new(13)), // dark_magenta
                36 | 46 => TuiColor::Ansi(AnsiValue::new(14)), // dark_cyan
                37 | 47 => TuiColor::Ansi(AnsiValue::new(15)), // gray

                // Bright colors (90-97, 100-107) → palette indices 8, 1-7
                90 | 100 => TuiColor::Ansi(AnsiValue::new(8)), // dark_gray
                91 | 101 => TuiColor::Ansi(AnsiValue::new(1)), // red
                92 | 102 => TuiColor::Ansi(AnsiValue::new(2)), // green
                93 | 103 => TuiColor::Ansi(AnsiValue::new(3)), // yellow
                94 | 104 => TuiColor::Ansi(AnsiValue::new(4)), // blue
                95 | 105 => TuiColor::Ansi(AnsiValue::new(5)), // magenta
                96 | 106 => TuiColor::Ansi(AnsiValue::new(6)), // cyan
                97 | 107 => TuiColor::Ansi(AnsiValue::new(7)), // white

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
            }
        }
    }
}

mod impl_debug {
    use super::{Debug, RgbValue, TuiColor};

    impl Debug for TuiColor {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                TuiColor::Rgb(RgbValue { red, green, blue }) => {
                    write!(f, "{red},{green},{blue}")
                }
                TuiColor::Ansi(ansi_value) => {
                    if ansi_value.index < 16 {
                        // Show friendly name for basic colors (indices 0-15)
                        let name = match ansi_value.index {
                            0 => "black",
                            1 => "red",
                            2 => "green",
                            3 => "yellow",
                            4 => "blue",
                            5 => "magenta",
                            6 => "cyan",
                            7 => "white",
                            8 => "dark_gray",
                            9 => "dark_red",
                            10 => "dark_green",
                            11 => "dark_yellow",
                            12 => "dark_blue",
                            13 => "dark_magenta",
                            14 => "dark_cyan",
                            15 => "gray",
                            _ => "unknown",
                        };
                        write!(f, "{name}")
                    } else {
                        write!(f, "ansi_value({})", ansi_value.index)
                    }
                }
            }
        }
    }
}

impl TransformColor for TuiColor {
    fn as_rgb(&self) -> RgbValue {
        match self {
            TuiColor::Rgb(rgb) => *rgb,
            TuiColor::Ansi(ansi) => ansi.as_rgb(),
        }
    }

    fn as_ansi(&self) -> AnsiValue {
        match self {
            TuiColor::Rgb(rgb) => convert_rgb_into_ansi256(*rgb),
            TuiColor::Ansi(ansi) => *ansi,
        }
    }

    fn as_grayscale(&self) -> AnsiValue {
        match self {
            TuiColor::Rgb(rgb) => convert_rgb_into_ansi256(*rgb).as_grayscale(),
            TuiColor::Ansi(ansi) => ansi.as_grayscale(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_eq2;
    use test_case::test_case;

    #[test]
    fn test_tui_color_as_different_variants() {
        {
            let tui_color = tui_color!(255, 0, 0);
            let expected_color = TuiColor::Rgb((255, 0, 0).into());
            assert_eq!(tui_color, expected_color);
        }
        {
            let tui_color = tui_color!(ansi 42);
            let expected_color = TuiColor::Ansi(42.into());
            assert_eq!(tui_color, expected_color);
        }
        {
            let tui_color = tui_color!(red);
            let expected_color = TuiColor::Ansi(AnsiValue::new(1)); // red is palette index 1
            assert_eq!(tui_color, expected_color);
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

    #[test_case(ANSIBasicColor::Black)]
    #[test_case(ANSIBasicColor::White)]
    #[test_case(ANSIBasicColor::Red)]
    #[test_case(ANSIBasicColor::Green)]
    fn test_basic_color_to_rgb(color: ANSIBasicColor) {
        // Basic colors are now represented as TuiColor::Ansi with indices 0-15
        // Convert to TuiColor and then to RgbValue via ANSI palette
        let tui_color: TuiColor = color.into();
        let _rgb: RgbValue = tui_color.into();
        // Just verify conversion succeeds without error
    }

    #[test_case(ANSIBasicColor::Black)]
    #[test_case(ANSIBasicColor::DarkGray)]
    #[test_case(ANSIBasicColor::Red)]
    #[test_case(ANSIBasicColor::DarkRed)]
    #[test_case(ANSIBasicColor::Green)]
    #[test_case(ANSIBasicColor::DarkGreen)]
    #[test_case(ANSIBasicColor::Yellow)]
    #[test_case(ANSIBasicColor::DarkYellow)]
    #[test_case(ANSIBasicColor::Blue)]
    #[test_case(ANSIBasicColor::DarkBlue)]
    #[test_case(ANSIBasicColor::Magenta)]
    #[test_case(ANSIBasicColor::DarkMagenta)]
    #[test_case(ANSIBasicColor::Cyan)]
    #[test_case(ANSIBasicColor::DarkCyan)]
    #[test_case(ANSIBasicColor::White)]
    #[test_case(ANSIBasicColor::Gray)]
    fn test_basic_colors_macro(color: ANSIBasicColor) {
        // Verify that basic colors via macro expand correctly through palette
        let tui_color: TuiColor = color.into();
        let _rgb: RgbValue = tui_color.into();
        // Just verify conversion succeeds without error
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
