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

use crate::{ASTColor,
            DEFAULT_STRING_STORAGE_SIZE,
            InlineString,
            InlineVec,
            SgrCode,
            TuiStyle,
            tui_color};

/// Please don't create this struct directly, use [crate::ast()], [crate::ast_line!],
/// [crate::ast_lines!] or the constructor functions like [fg_red()], [fg_green()],
/// [fg_blue()], etc.
///
/// The main struct that we have to consider is `AnsiStyledText` or `AST`. It has two
/// fields:
/// - `text` - the text to print.
/// - `styles` - a list of [ASTStyle] to apply to the text. This is owned in a stack
///   allocated buffer, which can spill to the heap if it gets larger than
///   `sizing::MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE`.
/// - Once created, either directly or using constructor functions like [fg_red()], you
///   can then use [Self::bg_dark_gray()] to add a background color to the text.
/// - If you want even more flexibility you can use constructor function [fg_color()] and
///   [Self::bg_color()] to create a styled text with a specific RGB color.
///
/// # Example usage:
///
/// ```rust
/// # use r3bl_core::{
/// #     TuiStyle, tui_color, new_style,
/// #     ast, fg_red, dim, AST, fg_color,
/// #     ASTStyle, ASTColor,
/// # };
///
/// // Use ast() to create a styled text. Use this.
/// let styled_text = ast("Hello", new_style!(bold));
/// println!("{styled_text}");
/// styled_text.println();
///
/// // Using the constructor functions.
/// let red_text = fg_red("This is red text.");
/// let red_text_on_dark_gray = red_text.bg_dark_gray();
/// println!("{red_text_on_dark_gray}");
/// red_text_on_dark_gray.println();
///
/// // Combine constructor functions.
/// let dim_red_text_on_dark_gray = dim("text").fg_color(tui_color!(255, 0, 0)).bg_color(tui_color!(50, 50, 50));
/// println!("{dim_red_text_on_dark_gray}");
/// dim_red_text_on_dark_gray.println();
///
/// // Flexible construction using RGB color codes.
/// let blue_text = fg_color(tui_color!(blue), "This is blue text.");
/// let blue_text_on_white = blue_text.bg_color(tui_color!(white));
/// println!("{blue_text_on_white}");
/// blue_text_on_white.println();
///
/// // Verbose struct construction (don't use this).
/// AST {
///     text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.".into(),
///     styles: smallvec::smallvec![
///         ASTStyle::Bold,
///         ASTStyle::Italic,
///         ASTStyle::Underline,
///         ASTStyle::Foreground(ASTColor::Rgb((50, 50, 50).into())),
///         ASTStyle::Background(ASTColor::Rgb((100, 200, 1).into())),
///     ],
/// }
/// .println();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiStyledText {
    pub text: InlineString,
    /// You can supply this directly, or use [crate::new_style!] to create a
    /// [crate::TuiStyle] and convert it to this type using `.into()`.
    pub styles: ASTStyles,
}

// Type aliases for better readability.

pub type AST = AnsiStyledText;
pub type ASTLine = InlineVec<AnsiStyledText>;
pub type ASTLines = InlineVec<ASTLine>;
pub type ASTStyles = sizing::InlineVecASTStyles;

pub(in crate::ansi) mod sizing {
    use super::*;

    /// Attributes are: color_fg, color_bg, bold, dim, italic, underline, reverse, hidden,
    /// etc. which are in [crate::ASTStyle].
    pub const MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE: usize = 12;
    pub type InlineVecASTStyles =
        SmallVec<[ASTStyle; MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE]>;
}

/// Easy to use constructor function, instead of creating a new [AnsiStyledText] struct
/// directly. If you need to assemble a bunch of these together, you can use
/// [crate::ast_line!] to create a list of them.
pub fn ast(arg_text: impl AsRef<str>, arg_styles: impl Into<ASTStyles>) -> AST {
    AST {
        text: arg_text.as_ref().into(),
        styles: arg_styles.into(),
    }
}

/// String together a bunch of [AnsiStyledText] structs into a single
/// [`crate::InlineVec<AnsiStyledText>`]. This is useful for creating a list of
/// [AnsiStyledText] structs that can be printed on a single line.
#[macro_export]
macro_rules! ast_line {
    (
        $( $ast_chunk:expr ),* $(,)?
    ) => {{
        use $crate::{InlineVec, ASTLine};
        let mut acc: ASTLine = InlineVec::new();
        $(
            acc.push($ast_chunk);
        )*
        acc
    }};
}

/// String together a bunch of formatted lines into a single
/// [`crate::InlineVec<InlineVec<AnsiStyledText>>`]. This is useful for assembling
/// multiline formatted text which is used in multi line headers, for example.
#[macro_export]
macro_rules! ast_lines {
    (
        $( $ast_line:expr ),* $(,)?
    ) => {{
        use $crate::{InlineVec, ASTLines};
        let mut acc: ASTLines = InlineVec::new();
        $(
            acc.push($ast_line);
        )*
        acc
    }};
}

mod ansi_styled_text_impl {
    use super::*;

    impl AnsiStyledText {
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

pub fn fg_color(arg_color: impl Into<ASTColor>, text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(arg_color.into())),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_dark_gray(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(236.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_black(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(0.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_yellow(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(226.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_green(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(34.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_blue(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(27.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_red(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(196.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_white(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(231.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_cyan(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(51.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
pub fn fg_magenta(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(201.into()))),
    }
}

pub fn fg_silver_metallic(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(
            crate::tui_color!(silver_metallic).into()
        )),
    }
}

pub fn fg_lizard_green(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(lizard_green).into())),
    }
}

pub fn fg_pink(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(pink).into())),
    }
}

pub fn fg_dark_pink(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(dark_pink).into())),
    }
}

pub fn fg_frozen_blue(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(frozen_blue).into())),
    }
}

pub fn fg_guards_red(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(guards_red).into())),
    }
}

pub fn fg_slate_gray(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(slate_gray).into())),
    }
}

pub fn bold(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Bold),
    }
}

pub fn italic(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Italic),
    }
}

pub fn underline(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Underline),
    }
}

pub fn strikethrough(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Strikethrough),
    }
}

pub fn dim(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Dim),
    }
}

pub fn dim_underline(text: &str) -> AST {
    AST {
        text: text.into(),
        styles: smallvec!(ASTStyle::Dim, ASTStyle::Underline),
    }
}

impl AST {
    pub fn dim(mut self) -> Self {
        self.styles.push(ASTStyle::Dim);
        self
    }

    pub fn bold(mut self) -> Self {
        self.styles.push(ASTStyle::Bold);
        self
    }

    pub fn bg_color(mut self, arg_color: impl Into<ASTColor>) -> Self {
        let color: ASTColor = arg_color.into();
        self.styles.push(ASTStyle::Background(color));
        self
    }

    pub fn fg_color(mut self, arg_color: impl Into<ASTColor>) -> Self {
        let color: ASTColor = arg_color.into();
        self.styles.push(ASTStyle::Foreground(color));
        self
    }

    pub fn bg_cyan(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(51.into())));
        self
    }

    pub fn bg_yellow(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(226.into())));
        self
    }

    pub fn bg_green(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(34.into())));
        self
    }

    pub fn bg_slate_gray(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(crate::tui_color!(slate_gray).into()));
        self
    }

    pub fn bg_dark_gray(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(236.into())));
        self
    }

    pub fn bg_night_blue(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(tui_color!(night_blue).into()));
        self
    }

    pub fn bg_moonlight_blue(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(tui_color!(moonlight_blue).into()));
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
            if tui_style.bold.is_some() {
                styles.push(ASTStyle::Bold);
            }
            if tui_style.dim.is_some() {
                styles.push(ASTStyle::Dim);
            }
            if tui_style.italic.is_some() {
                styles.push(ASTStyle::Italic);
            }
            if tui_style.underline.is_some() {
                styles.push(ASTStyle::Underline);
            }
            if tui_style.reverse.is_some() {
                styles.push(ASTStyle::Invert);
            }
            // Not supported:
            // - Overline,
            // - RapidBlink,
            // - SlowBlink,
            if tui_style.hidden.is_some() {
                styles.push(ASTStyle::Hidden);
            }
            if tui_style.strikethrough.is_some() {
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

    impl Display for AST {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            for style_item in &self.styles {
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
    use crate::{AST,
                ASTColor,
                ASTStyle,
                ColorSupport,
                TuiStyle,
                ansi::sizing::InlineVecASTStyles,
                global_color_support,
                tui_color,
                tui_style::tui_style_attrib::{Bold,
                                              Dim,
                                              Hidden,
                                              Italic,
                                              Reverse,
                                              Strikethrough,
                                              Underline}};

    #[serial]
    #[test]
    fn test_convert_tui_style_to_vec_ast_style() {
        {
            let tui_style = TuiStyle {
                bold: Some(Bold),
                dim: None,
                italic: Some(Italic),
                underline: None,
                reverse: None,
                hidden: None,
                strikethrough: Some(Strikethrough),
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
                bold: None,
                dim: Some(Dim),
                italic: None,
                underline: Some(Underline),
                reverse: Some(Reverse),
                hidden: Some(Hidden),
                strikethrough: None,
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
                bold: Some(Bold),
                dim: Some(Dim),
                italic: Some(Italic),
                underline: Some(Underline),
                reverse: Some(Reverse),
                hidden: Some(Hidden),
                strikethrough: Some(Strikethrough),
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
        let eg_1 = AST {
            text: "Hello".into(),
            styles: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
            ),
        };
        println!("{:?}", eg_1);
        println!("{}", eg_1);
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "Hello", styles: [Bold, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 }))] }"#
        );

        let eg_2 = eg_1.bg_dark_gray();
        println!("{:?}", eg_2);
        println!("{}", eg_2);
        assert_eq!(
            format!("{:?}", eg_2),
            r#"AnsiStyledText { text: "Hello", styles: [Bold, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 })), Background(Ansi(AnsiValue { index: 236 }))] }"#
        );
    }

    #[serial]
    #[test]
    fn test_fg_bg_combo() {
        let eg_1 = dim("hello")
            .fg_color(tui_color!(0, 0, 0))
            .bg_color(tui_color!(1, 1, 1));
        println!("{:?}", eg_1);
        println!("{}", eg_1);
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "hello", styles: [Dim, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 })), Background(Rgb(RgbValue { red: 1, green: 1, blue: 1 }))] }"#
        );
    }

    #[serial]
    #[test]
    fn test_formatted_string_creation_ansi256() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Ansi256);
        let eg_1 = AST {
            text: "Hello".into(),
            styles: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;5;16m\x1b[48;5;16mHello\x1b[0m".to_string()
        );

        let eg_2 = AST {
            text: "World".into(),
            styles: smallvec!(
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
        let eg_1 = AST {
            text: "Hello".into(),
            styles: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((1, 1, 1).into())),
            ),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;2;0;0;0m\x1b[48;2;1;1;1mHello\x1b[0m".to_string()
        );

        let eg_2 = AST {
            text: "World".into(),
            styles: smallvec!(
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
        let eg_1 = AST {
            text: "Hello".into(),
            styles: smallvec!(
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

        let eg_2 = AST {
            text: "World".into(),
            styles: smallvec!(
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
