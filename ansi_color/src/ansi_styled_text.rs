/*
 *   Copyright (c) 2023 R3BL LLC
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

use crate::*;

/// The main struct that we have to consider is `AnsiStyledText`. It has two fields:
/// - `text` - the text to print.
/// - `style` - a list of [Style] to apply to the text.
///
/// ## Example usage:
///
/// ```rust
/// use r3bl_ansi_color::*;
///
/// AnsiStyledText {
///     text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
///     style: &[
///         Style::Bold,
///         Style::Italic,
///         Style::Underline,
///         Style::Foreground(Color::Rgb(50, 50, 50)),
///         Style::Background(Color::Rgb(100, 200, 1)),
///     ],
/// }
/// .println();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnsiStyledText<'a> {
    pub text: &'a str,
    pub style: &'a [Style],
}

mod ansi_styled_text_impl {
    use crate::AnsiStyledText;

    impl AnsiStyledText<'_> {
        pub fn println(&self) {
            println!("{}", self);
        }

        pub fn print(&self) {
            print!("{}", self);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Foreground(Color),
    Background(Color),
    Bold,
    Dim,
    Italic,
    Underline,
    Overline,
    RapidBlink,
    SlowBlink,
    Invert,
    Hidden,
    Strikethrough,
}

mod style_impl {
    use std::fmt::{Display, Formatter, Result};

    use crate::{global_color_support,
                Color,
                ColorSupport,
                RgbColor,
                SgrCode,
                Style,
                TransformColor};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ColorKind {
        Foreground,
        Background,
    }

    fn fmt_color(color: Color, color_kind: ColorKind, f: &mut Formatter<'_>) -> Result {
        match global_color_support::detect() {
            ColorSupport::Ansi256 => {
                // ANSI 256 color mode.
                let color = color.as_ansi256();
                let index = color.index;
                write!(
                    f,
                    "{}",
                    match color_kind {
                        ColorKind::Foreground => SgrCode::ForegroundAnsi256(index),
                        ColorKind::Background => SgrCode::BackgroundAnsi256(index),
                    }
                )
            }

            ColorSupport::Grayscale => {
                // Grayscale mode.
                let color = color.as_grayscale();
                let index = color.index;
                write!(
                    f,
                    "{}",
                    match color_kind {
                        ColorKind::Foreground => SgrCode::ForegroundAnsi256(index),
                        ColorKind::Background => SgrCode::BackgroundAnsi256(index),
                    }
                )
            }

            _ => {
                // True color mode.
                let color = color.as_rgb();
                let RgbColor { red, green, blue } = color;
                write!(
                    f,
                    "{}",
                    match color_kind {
                        ColorKind::Foreground => SgrCode::ForegroundRGB(red, green, blue),
                        ColorKind::Background => SgrCode::BackgroundRGB(red, green, blue),
                    }
                )
            }
        }
    }

    impl Display for Style {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                Style::Foreground(color) => fmt_color(*color, ColorKind::Foreground, f),
                Style::Background(color) => fmt_color(*color, ColorKind::Background, f),
                Style::Bold => write!(f, "{}", SgrCode::Bold),
                Style::Dim => write!(f, "{}", SgrCode::Dim),
                Style::Italic => write!(f, "{}", SgrCode::Italic),
                Style::Underline => write!(f, "{}", SgrCode::Underline),
                Style::SlowBlink => write!(f, "{}", SgrCode::SlowBlink),
                Style::RapidBlink => write!(f, "{}", SgrCode::RapidBlink),
                Style::Invert => write!(f, "{}", SgrCode::Invert),
                Style::Hidden => write!(f, "{}", SgrCode::Hidden),
                Style::Strikethrough => write!(f, "{}", SgrCode::Strikethrough),
                Style::Overline => write!(f, "{}", SgrCode::Overline),
            }
        }
    }
}

mod display_trait_impl {
    use std::fmt::{Display, Formatter, Result};

    use crate::{AnsiStyledText, SgrCode};

    // https://doc.rust-lang.org/std/fmt/trait.Display.html
    impl Display for AnsiStyledText<'_> {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
            let mut style_string_vec = vec![];
            for style_item in self.style {
                style_string_vec.push(style_item.to_string());
            }
            style_string_vec.push(self.text.to_string());
            style_string_vec.push(SgrCode::Reset.to_string());
            write!(formatter, "{}", style_string_vec.join(""))
        }
    }

    #[cfg(test)]
    mod tests {
        use pretty_assertions::assert_eq;
        use serial_test::serial;

        use crate::*;

        #[serial]
        #[test]
        fn test_formatted_string_creation_ansi256() -> Result<(), String> {
            global_color_support::set_override(ColorSupport::Ansi256);
            let eg_1 = AnsiStyledText {
                text: "Hello",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Rgb(0, 0, 0)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            assert_eq!(
                format!("{0}", eg_1),
                "\x1b[1m\x1b[38;5;16m\x1b[48;5;16mHello\x1b[0m".to_string()
            );

            let eg_2 = AnsiStyledText {
                text: "World",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Ansi256(150)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            assert_eq!(
                format!("{0}", eg_2),
                "\x1b[1m\x1b[38;5;150m\x1b[48;5;16mWorld\x1b[0m".to_string()
            );

            Ok(())
        }

        #[serial]
        #[test]
        fn test_formatted_string_creation_truecolor() -> Result<(), String> {
            global_color_support::set_override(ColorSupport::Truecolor);
            let eg_1 = AnsiStyledText {
                text: "Hello",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Rgb(0, 0, 0)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            assert_eq!(
                format!("{0}", eg_1),
                "\x1b[1m\x1b[38;2;0;0;0m\x1b[48;2;1;1;1mHello\x1b[0m".to_string()
            );

            let eg_2 = AnsiStyledText {
                text: "World",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Ansi256(150)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            assert_eq!(
                format!("{0}", eg_2),
                "\x1b[1m\x1b[38;2;175;215;135m\x1b[48;2;1;1;1mWorld\x1b[0m".to_string()
            );

            Ok(())
        }

        #[serial]
        #[test]
        fn test_formatted_string_creation_grayscale() -> Result<(), String> {
            global_color_support::set_override(ColorSupport::Grayscale);
            let eg_1 = AnsiStyledText {
                text: "Hello",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Rgb(0, 0, 0)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            println!("{:?}", format!("{0}", eg_1));

            assert_eq!(
                format!("{0}", eg_1),
                "\u{1b}[1m\u{1b}[38;5;16m\u{1b}[48;5;16mHello\u{1b}[0m".to_string()
            );

            let eg_2 = AnsiStyledText {
                text: "World",
                style: &[
                    Style::Bold,
                    Style::Foreground(Color::Ansi256(150)),
                    Style::Background(Color::Rgb(1, 1, 1)),
                ],
            };

            println!("{:?}", format!("{0}", eg_2));

            assert_eq!(
                format!("{0}", eg_2),
                "\u{1b}[1m\u{1b}[38;5;251m\u{1b}[48;5;16mWorld\u{1b}[0m".to_string()
            );

            Ok(())
        }
    }
}
