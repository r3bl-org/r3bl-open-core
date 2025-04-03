/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use std::fmt::{Display, Formatter, Result};

use smallstr::SmallString;
use smallvec::{SmallVec, smallvec};
use strum_macros::EnumCount;

use crate::{ASTColor, DEFAULT_STRING_STORAGE_SIZE, RgbValue, SgrCode, TuiStyle};

/// The main struct that we have to consider is `AnsiStyledText`. It has two fields:
/// - `text` - the text to print.
/// - `style` - a list of [ASTStyle] to apply to the text. This is owned in a stack
///   allocated buffer, which can spill to the heap if it gets larger than
///   `sizing::MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE`.
/// - Once created, either directly or using constructor functions like [super::red()],
///   you can then use [Self::bg_dark_grey()] to add a background color to the text.
/// - If you want even more flexibility you can use constructor function
///   [super::fg_rgb_color()] and [Self::bg_rgb_color()] to create a styled text with a specific RGB
///   color.
///
/// # Example usage:
///
/// ```rust
/// use r3bl_core::{
///     TuiStyle,
///     red, rgb_value, dim, AnsiStyledText, fg_rgb_color, ASTStyle, ASTColor
/// };
///
/// // Using the constructor functions.
/// let red_text = red("This is red text.");
/// let red_text_on_dark_grey = red_text.bg_dark_grey();
/// println!("{red_text_on_dark_grey}");
/// red_text_on_dark_grey.println();
///
/// // Combine constructor functions.
/// let dim_red_text_on_dark_grey = dim("text").fg_rgb_color((255, 0, 0)).bg_rgb_color((50, 50, 50));
/// println!("{dim_red_text_on_dark_grey}");
/// dim_red_text_on_dark_grey.println();
///
/// // Flexible construction using RGB color codes.
/// let blue_text = fg_rgb_color(rgb_value!(blue), "This is blue text.");
/// let blue_text_on_white = blue_text.bg_rgb_color(rgb_value!(white));
/// println!("{blue_text_on_white}");
/// blue_text_on_white.println();
///
/// // Verbose struct construction.
/// AnsiStyledText {
///     text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
///     style: smallvec::smallvec![
///         ASTStyle::Bold,
///         ASTStyle::Italic,
///         ASTStyle::Underline,
///         ASTStyle::Foreground(ASTColor::Rgb((50, 50, 50).into())),
///         ASTStyle::Background(ASTColor::Rgb((100, 200, 1).into())),
///     ],
/// }
/// .println();
///
/// // Use TuiStyle to create a styled text.
/// let tui_style = TuiStyle {
///    bold: true,
///    dim: false,
///    ..Default::default()
/// };
/// let styled_text = AnsiStyledText {
///    text: "Hello",
///    style: tui_style.into(),
/// };
/// println!("{styled_text}");
/// styled_text.println();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiStyledText<'a> {
    pub text: &'a str,
    /// You can supply this directly, or use [crate::new_style!] to create a
    /// [crate::TuiStyle] and convert it to this type using `.into()`.
    pub style: sizing::InlineVecASTStyles,
}

pub(in crate::ansi) mod sizing {
    use super::*;

    /// Attributes are: color_fg, color_bg, bold, dim, italic, underline, reverse, hidden,
    /// etc. which are in [crate::ASTStyle].
    pub const MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE: usize = 12;
    pub type InlineVecASTStyles =
        SmallVec<[ASTStyle; MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE]>;
}

mod ansi_styled_text_impl {
    use super::*;

    impl AnsiStyledText<'_> {
        pub fn println(&self) {
            println!("{}", self);
        }

        pub fn print(&self) {}

        /// This is different than the [Display] trait implementation, because it doesn't
        /// allocate a new [String], but instead allocates an inline buffer on the stack.
        /// If this buffer gets larger than [DEFAULT_STRING_STORAGE_SIZE], it will
        /// spill to the heap.
        pub fn to_small_str(&self) -> SmallString<[u8; DEFAULT_STRING_STORAGE_SIZE]> {
            format!("{}", self).into()
        }
    }
}

pub fn fg_rgb_color(arg_color: impl Into<RgbValue>, text: &str) -> AnsiStyledText<'_> {
    let rgb_color = arg_color.into();
    let ast_color = ASTColor::from(rgb_color);
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ast_color)),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn green(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(34.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn red(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(196.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn white(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(231.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn cyan(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(51.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn yellow(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(226.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn magenta(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(201.into()))),
    }
}

pub fn lizard_green(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(crate::rgb_value!(lizard_green).into())),
    }
}

pub fn pink(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(crate::rgb_value!(pink).into())),
    }
}

pub fn dark_pink(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(crate::rgb_value!(dark_pink).into())),
    }
}

pub fn frozen_blue(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(crate::rgb_value!(frozen_blue).into())),
    }
}

pub fn guards_red(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(crate::rgb_value!(guards_red).into())),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn blue(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(27.into()))),
    }
}

pub fn bold(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Bold),
    }
}

pub fn italic(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Italic),
    }
}

pub fn underline(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Underline),
    }
}

pub fn strikethrough(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Strikethrough),
    }
}

pub fn dim(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Dim),
    }
}

pub fn dim_underline(text: &str) -> AnsiStyledText<'_> {
    AnsiStyledText {
        text,
        style: smallvec!(ASTStyle::Dim, ASTStyle::Underline),
    }
}

impl AnsiStyledText<'_> {
    pub fn bg_dark_grey(mut self) -> Self {
        self.style
            .push(ASTStyle::Background(ASTColor::Ansi(236.into())));
        self
    }

    pub fn bg_rgb_color(mut self, arg_color: impl Into<RgbValue>) -> Self {
        let color = arg_color.into();
        let ast_color = ASTColor::from(color);
        self.style.push(ASTStyle::Background(ast_color));
        self
    }

    pub fn fg_rgb_color(mut self, arg_color: impl Into<RgbValue>) -> Self {
        let color = arg_color.into();
        let ast_color = ASTColor::from(color);
        self.style.push(ASTStyle::Foreground(ast_color));
        self
    }
}

/// This enum isn't the same as the [TuiStyle] struct. This enum can only hold a single
/// variant. The [TuiStyle] struct can hold multiple variants. This is a low level enum
/// that shouldn't be used directly. It is best to use [TuiStyle] and [crate::new_style!]
/// to create a [TuiStyle] and convert it to this type using `.into()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount)]
pub enum ASTStyle {
    Foreground(ASTColor),
    Background(ASTColor),
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

mod convert_tui_style_to_vec_ast_style {
    use super::{sizing::InlineVecASTStyles, *};

    impl From<TuiStyle> for sizing::InlineVecASTStyles {
        fn from(tui_style: TuiStyle) -> Self {
            let mut styles = InlineVecASTStyles::new();
            if tui_style.bold {
                styles.push(ASTStyle::Bold);
            }
            if tui_style.dim {
                styles.push(ASTStyle::Dim);
            }
            if tui_style.italic {
                styles.push(ASTStyle::Italic);
            }
            if tui_style.underline {
                styles.push(ASTStyle::Underline);
            }
            if tui_style.reverse {
                styles.push(ASTStyle::Invert);
            }
            // Not supported:
            // Overline,
            // RapidBlink,
            // SlowBlink,
            if tui_style.hidden {
                styles.push(ASTStyle::Hidden);
            }
            if tui_style.strikethrough {
                styles.push(ASTStyle::Strikethrough);
            }
            if let Some(color_fg) = tui_style.color_fg {
                styles.push(ASTStyle::Foreground(color_fg.into()));
            }
            if let Some(color_bg) = tui_style.color_bg {
                styles.push(ASTStyle::Background(color_bg.into()));
            }
            styles
        }
    }
}

mod style_impl {
    use std::fmt::{Display, Formatter, Result};

    use crate::{ASTColor,
                ASTStyle,
                ColorSupport,
                RgbValue,
                SgrCode,
                TransformColor,
                global_color_support};

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ColorKind {
        Foreground,
        Background,
    }

    fn fmt_color(
        color: ASTColor,
        color_kind: ColorKind,
        f: &mut Formatter<'_>,
    ) -> Result {
        match global_color_support::detect() {
            ColorSupport::Ansi256 => {
                // ANSI 256 color mode.
                let color = color.as_ansi();
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
                let RgbValue { red, green, blue } = color;
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

    impl Display for ASTStyle {
        #[rustfmt::skip]
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                ASTStyle::Foreground(color)  => fmt_color(*color, ColorKind::Foreground, f),
                ASTStyle::Background(color)  => fmt_color(*color, ColorKind::Background, f),
                ASTStyle::Bold               => write!(f, "{}", SgrCode::Bold),
                ASTStyle::Dim                => write!(f, "{}", SgrCode::Dim),
                ASTStyle::Italic             => write!(f, "{}", SgrCode::Italic),
                ASTStyle::Underline          => write!(f, "{}", SgrCode::Underline),
                ASTStyle::SlowBlink          => write!(f, "{}", SgrCode::SlowBlink),
                ASTStyle::RapidBlink         => write!(f, "{}", SgrCode::RapidBlink),
                ASTStyle::Invert             => write!(f, "{}", SgrCode::Invert),
                ASTStyle::Hidden             => write!(f, "{}", SgrCode::Hidden),
                ASTStyle::Strikethrough      => write!(f, "{}", SgrCode::Strikethrough),
                ASTStyle::Overline           => write!(f, "{}", SgrCode::Overline),
            }
        }
    }
}

mod display_trait_impl {
    use super::*;

    impl Display for AnsiStyledText<'_> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            for style_item in &self.style {
                write!(f, "{}", style_item)?;
            }
            write!(f, "{}", self.text)?;
            write!(f, "{}", SgrCode::Reset)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use smallvec::smallvec;

    use super::dim;
    use crate::{ASTColor,
                ASTStyle,
                AnsiStyledText,
                ColorSupport,
                TuiStyle,
                ansi::sizing::InlineVecASTStyles,
                global_color_support};

    #[serial]
    #[test]
    fn test_convert_tui_style_to_vec_ast_style() {
        {
            let tui_style = TuiStyle {
                bold: true,
                dim: false,
                italic: true,
                underline: false,
                reverse: false,
                hidden: false,
                strikethrough: true,
                ..Default::default()
            };
            let ast_styles: InlineVecASTStyles = tui_style.into();
            assert_eq!(
                ast_styles.as_ref(),
                &[ASTStyle::Bold, ASTStyle::Italic, ASTStyle::Strikethrough]
            );
        }

        {
            let tui_style = TuiStyle {
                bold: false,
                dim: true,
                italic: false,
                underline: true,
                reverse: true,
                hidden: true,
                strikethrough: false,
                ..Default::default()
            };
            let ast_styles: InlineVecASTStyles = tui_style.into();
            assert_eq!(
                ast_styles.as_ref(),
                &[
                    ASTStyle::Dim,
                    ASTStyle::Underline,
                    ASTStyle::Invert,
                    ASTStyle::Hidden
                ]
            );
        }

        {
            let tui_style = TuiStyle {
                bold: true,
                dim: true,
                italic: true,
                underline: true,
                reverse: true,
                hidden: true,
                strikethrough: true,
                ..Default::default()
            };
            let ast_styles: InlineVecASTStyles = tui_style.into();
            assert_eq!(
                ast_styles.as_ref(),
                &[
                    ASTStyle::Bold,
                    ASTStyle::Dim,
                    ASTStyle::Italic,
                    ASTStyle::Underline,
                    ASTStyle::Invert,
                    ASTStyle::Hidden,
                    ASTStyle::Strikethrough
                ]
            );
        }

        {
            let tui_style = TuiStyle {
                ..Default::default()
            };
            let ast_styles: InlineVecASTStyles = tui_style.into();
            assert!(ast_styles.is_empty());
        }
    }

    #[serial]
    #[test]
    fn test_fg_color_on_bg_color() {
        let eg_1 = AnsiStyledText {
            text: "Hello",
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
            ),
        };
        println!("{:?}", eg_1);
        println!("{}", eg_1);
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "Hello", style: [Bold, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 }))] }"#
        );

        let eg_2 = eg_1.bg_dark_grey();
        println!("{:?}", eg_2);
        println!("{}", eg_2);
        assert_eq!(
            format!("{:?}", eg_2),
            r#"AnsiStyledText { text: "Hello", style: [Bold, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 })), Background(Ansi(AnsiValue { index: 236 }))] }"#
        );
    }

    #[serial]
    #[test]
    fn test_fg_bg_combo() {
        let eg_1 = dim("hello").fg_rgb_color((0, 0, 0)).bg_rgb_color((1, 1, 1));
        println!("{:?}", eg_1);
        println!("{}", eg_1);
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "hello", style: [Dim, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 })), Background(Rgb(RgbValue { red: 1, green: 1, blue: 1 }))] }"#
        );
    }

    #[serial]
    #[test]
    fn test_formatted_string_creation_ansi256() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Ansi256);
        let eg_1 = AnsiStyledText {
            text: "Hello",
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;5;16m\x1b[48;5;16mHello\x1b[0m".to_string()
        );

        let eg_2 = AnsiStyledText {
            text: "World",
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Ansi(150.into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
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
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;2;0;0;0m\x1b[48;2;1;1;1mHello\x1b[0m".to_string()
        );

        let eg_2 = AnsiStyledText {
            text: "World",
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Ansi(150.into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
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
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        println!("{:?}", format!("{0}", eg_1));

        assert_eq!(
            format!("{0}", eg_1),
            "\u{1b}[1m\u{1b}[38;5;16m\u{1b}[48;5;16mHello\u{1b}[0m".to_string()
        );

        let eg_2 = AnsiStyledText {
            text: "World",
            style: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Ansi(150.into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        println!("{:?}", format!("{0}", eg_2));

        assert_eq!(
            format!("{0}", eg_2),
            "\u{1b}[1m\u{1b}[38;5;251m\u{1b}[48;5;16mWorld\u{1b}[0m".to_string()
        );

        Ok(())
    }
}
