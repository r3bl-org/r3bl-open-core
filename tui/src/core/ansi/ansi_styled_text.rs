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

use smallvec::{SmallVec, smallvec};
use strum_macros::EnumCount;

use crate::{ASTColor, BufTextStorage, ColIndex, ColWidth, GCString, InlineString,
            InlineVec, PixelChar, SgrCode, TuiStyle, WriteToBuf, inline_string,
            tui_color,
            tui_style::tui_style_attrib::{Bold, Dim, Hidden, Italic, Reverse,
                                          Strikethrough, Underline}};

/// Please don't create this struct directly, use [`crate::ast()`], [`crate::ast_line`!],
/// [`crate::ast_lines`!] or the constructor functions like [`fg_red()`], [`fg_green()`],
/// [`fg_blue()`], etc.
///
/// The main struct that we have to consider is `AnsiStyledText` or `AST`. It has two
/// fields:
/// - `text` - the text to print.
/// - `styles` - a list of [`ASTStyle`] to apply to the text. This is owned in a stack
///   allocated buffer, which can spill to the heap if it gets larger than
///   `sizing::MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE`.
/// - Once created, either directly or using constructor functions like [`fg_red()`], you
///   can then use [`Self::bg_dark_gray()`] to add a background color to the text.
/// - If you want even more flexibility you can use constructor function [`fg_color()`]
///   and [`Self::bg_color()`] to create a styled text with a specific RGB color.
///
/// # Example usage:
///
/// ```
/// # use r3bl_tui::{
/// #     TuiStyle, tui_color, new_style,
/// #     ast, fg_red, dim, ASText, fg_color,
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
/// ASText {
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
    /// You can supply this directly, or use [`crate::new_style`!] to create a
    /// [`crate::TuiStyle`] and convert it to this type using `.into()`.
    pub styles: ASTextStyles,
}

// Type aliases for better readability.

pub type ASText = AnsiStyledText;
pub type ASTextLine = InlineVec<AnsiStyledText>;
pub type ASTextLines = InlineVec<ASTextLine>;
pub type ASTextStyles = sizing::InlineVecASTextStyles;

pub(in crate::core::ansi) mod sizing {
    use super::{ASTStyle, SmallVec};

    /// Attributes are: `color_fg`, `color_bg`, bold, dim, italic, underline, reverse,
    /// hidden, etc. which are in [`crate::ASTStyle`].
    pub const MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE: usize = 12;
    pub type InlineVecASTextStyles =
        SmallVec<[ASTStyle; MAX_ANSI_STYLED_TEXT_STYLE_ATTRIB_SIZE]>;
}

/// Easy to use constructor function, instead of creating a new [`AnsiStyledText`] struct
/// directly. If you need to assemble a bunch of these together, you can use
/// [`crate::ast_line!`] to create a list of them.
#[must_use]
pub fn ast(arg_text: impl AsRef<str>, arg_styles: impl Into<ASTextStyles>) -> ASText {
    ASText {
        text: arg_text.as_ref().into(),
        styles: arg_styles.into(),
    }
}

/// String together a bunch of [`AnsiStyledText`] structs into a single
/// [`crate::InlineVec<AnsiStyledText>`]. This is useful for creating a list of
/// [`AnsiStyledText`] structs that can be printed on a single line.
#[macro_export]
macro_rules! ast_line {
    (
        $( $ast_chunk:expr ),* $(,)?
    ) => {{
        use $crate::{InlineVec, ASTextLine};
        let mut acc: ASTextLine = InlineVec::new();
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
        use $crate::{InlineVec, ASTextLines};
        let mut acc: ASTextLines = InlineVec::new();
        $(
            acc.push($ast_line);
        )*
        acc
    }};
}

pub mod ansi_styled_text_impl {
    use super::{ASText, AnsiStyledText, ColIndex, ColWidth, GCString, InlineString,
                InlineVec, PixelChar, TuiStyle, inline_string};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct ASTextConvertOptions {
        pub start: Option<ColIndex>,
        pub end: Option<ColIndex>,
    }

    impl From<ColWidth> for ASTextConvertOptions {
        fn from(col_width: ColWidth) -> Self {
            Self {
                start: Some(0.into()),
                end: Some(col_width.convert_to_col_index()),
            }
        }
    }

    impl AnsiStyledText {
        pub fn println(&self) {
            println!("{self}");
        }

        pub fn print(&self) {
            print!("{self}");
        }

        /// This is different from the [`std::fmt::Display`] trait implementation, because
        /// it doesn't allocate a new [`String`], but instead allocates an inline
        /// buffer on the stack.
        #[must_use]
        pub fn to_small_str(&self) -> InlineString { inline_string!("{self}") }

        /// This is a convenience function to clip the text to a certain display width.
        /// You can also clip it to any given start and end index (inclusive).
        #[must_use]
        pub fn clip(&self, arg_options: impl Into<ASTextConvertOptions>) -> ASText {
            let ir_text = self.convert(arg_options);

            let mut acc = InlineString::with_capacity(self.text.len());
            for pixel_char in &ir_text {
                if let PixelChar::PlainText {
                    display_char,
                    maybe_style: _,
                } = pixel_char
                {
                    acc.push(*display_char);
                }
            }

            ASText {
                text: acc,
                styles: self.styles.clone(),
            }
        }

        /// Converts the text to a vector of [`PixelChar`]s. This is used for rendering
        /// the text on the screen.
        /// - To clip the text to a certain display width you can pass in the [`ColWidth`]
        ///   to this function.
        /// - To convert the entire text, just pass in [`ASTextConvertOptions::default()`]
        ///   function.
        /// - To convert a range of text, pass in the start and end indices. Note that it
        ///   will be inclusive (not the default Rust behavior), so the end index will be
        ///   included in the result.
        pub fn convert(
            &self,
            arg_options: impl Into<ASTextConvertOptions>,
        ) -> InlineVec<PixelChar> {
            let ASTextConvertOptions { start, end } = arg_options.into();

            // 1. Early return if the text is empty.
            if self.text.is_empty() {
                return InlineVec::new();
            }

            // 2. Convert self.styles to Option<TuiStyle>.
            let maybe_tui_style: Option<TuiStyle> = if self.styles.is_empty() {
                None
            } else {
                Some(self.styles.clone().into())
            };

            // 3. Iterate through characters and create PixelChar with maybe_tui_style.
            let pixel_chars = {
                let mut acc: InlineVec<PixelChar> =
                    InlineVec::with_capacity(self.text.len());
                let gc_string = GCString::from(&self.text);
                for item in &gc_string {
                    // Convert the grapheme cluster to a single char
                    // For multi-char graphemes, use the first char or fallback to
                    // replacement char
                    let display_char = item.chars().next().unwrap_or('�');
                    let pixel_char = PixelChar::PlainText {
                        display_char,
                        maybe_style: maybe_tui_style,
                    };
                    acc.push(pixel_char);
                }
                acc
            };

            // 4. Handle start and end inclusive indices, and slice the result.
            let end_index = match end {
                None => pixel_chars.len().saturating_sub(1),
                Some(it) => it.as_usize(),
            };
            let start_index = match start {
                None => 0,
                Some(it) => it.as_usize(),
            };

            // 4.1. Validate indices, and return empty InlineVec if invalid.
            if pixel_chars.is_empty()
                || start_index > end_index
                || start_index >= pixel_chars.len()
                || end_index >= pixel_chars.len()
            {
                return InlineVec::new();
            }

            // Slice and collect into a new InlineVec
            pixel_chars[start_index..=end_index]
                .iter()
                .copied()
                .collect()
        }
    }
}

// The following functions are convenience functions for providing ANSI attributes.

#[must_use]
pub fn bold(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Bold),
    }
}

#[must_use]
pub fn italic(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Italic),
    }
}

#[must_use]
pub fn underline(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Underline),
    }
}

#[must_use]
pub fn strikethrough(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Strikethrough),
    }
}

#[must_use]
pub fn dim(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Dim),
    }
}

#[must_use]
pub fn dim_underline(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Dim, ASTStyle::Underline),
    }
}

// The following function is a convenience function for providing any color.

#[must_use]
pub fn fg_color(arg_color: impl Into<ASTColor>, text: &str) -> ASText {
    ASText {
        text: text.into(),
        styles: smallvec!(ASTStyle::Foreground(arg_color.into())),
    }
}

// The following functions are convenience functions for providing ANSI colors.

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_dark_gray(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(236.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_black(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(0.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_yellow(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(226.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_green(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(34.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_blue(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(27.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_red(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(196.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_white(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(231.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_cyan(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(51.into()))),
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_magenta(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(ASTColor::Ansi(201.into()))),
    }
}

// The following colors are a convenience for using the [crate::tui_color!] macro.

#[must_use]
pub fn fg_medium_gray(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(medium_gray).into())),
    }
}

#[must_use]
pub fn fg_light_cyan(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(light_cyan).into())),
    }
}

#[must_use]
pub fn fg_light_purple(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(light_purple).into())),
    }
}

#[must_use]
pub fn fg_deep_purple(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(deep_purple).into())),
    }
}

#[must_use]
pub fn fg_soft_pink(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(soft_pink).into())),
    }
}

#[must_use]
pub fn fg_hot_pink(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(hot_pink).into())),
    }
}

#[must_use]
pub fn fg_light_yellow_green(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(
            crate::tui_color!(light_yellow_green).into()
        )),
    }
}

#[must_use]
pub fn fg_dark_teal(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(dark_teal).into())),
    }
}

#[must_use]
pub fn fg_bright_cyan(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(bright_cyan).into())),
    }
}

#[must_use]
pub fn fg_dark_purple(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(dark_purple).into())),
    }
}

#[must_use]
pub fn fg_sky_blue(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(sky_blue).into())),
    }
}

#[must_use]
pub fn fg_lavender(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(lavender).into())),
    }
}

#[must_use]
pub fn fg_dark_lizard_green(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(
            crate::tui_color!(dark_lizard_green).into()
        )),
    }
}

#[must_use]
pub fn fg_orange(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(orange).into())),
    }
}

#[must_use]
pub fn fg_silver_metallic(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(
            crate::tui_color!(silver_metallic).into()
        )),
    }
}

#[must_use]
pub fn fg_lizard_green(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(lizard_green).into())),
    }
}

#[must_use]
pub fn fg_pink(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(pink).into())),
    }
}

#[must_use]
pub fn fg_dark_pink(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(dark_pink).into())),
    }
}

#[must_use]
pub fn fg_frozen_blue(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(frozen_blue).into())),
    }
}

#[must_use]
pub fn fg_guards_red(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(guards_red).into())),
    }
}

#[must_use]
pub fn fg_slate_gray(text: impl AsRef<str>) -> ASText {
    ASText {
        text: text.as_ref().into(),
        styles: smallvec!(ASTStyle::Foreground(crate::tui_color!(slate_gray).into())),
    }
}

impl ASText {
    #[must_use]
    pub fn dim(mut self) -> Self {
        self.styles.push(ASTStyle::Dim);
        self
    }

    #[must_use]
    pub fn italic(mut self) -> Self {
        self.styles.push(ASTStyle::Italic);
        self
    }

    #[must_use]
    pub fn bold(mut self) -> Self {
        self.styles.push(ASTStyle::Bold);
        self
    }

    #[must_use]
    pub fn underline(mut self) -> Self {
        self.styles.push(ASTStyle::Underline);
        self
    }

    #[must_use]
    pub fn bg_color(mut self, arg_color: impl Into<ASTColor>) -> Self {
        let color: ASTColor = arg_color.into();
        self.styles.push(ASTStyle::Background(color));
        self
    }

    #[must_use]
    pub fn fg_color(mut self, arg_color: impl Into<ASTColor>) -> Self {
        let color: ASTColor = arg_color.into();
        self.styles.push(ASTStyle::Foreground(color));
        self
    }

    #[must_use]
    pub fn bg_cyan(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(51.into())));
        self
    }

    #[must_use]
    pub fn bg_yellow(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(226.into())));
        self
    }

    #[must_use]
    pub fn bg_green(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(34.into())));
        self
    }

    #[must_use]
    pub fn bg_slate_gray(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(crate::tui_color!(slate_gray).into()));
        self
    }

    #[must_use]
    pub fn bg_dark_gray(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(ASTColor::Ansi(236.into())));
        self
    }

    #[must_use]
    pub fn bg_night_blue(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(tui_color!(night_blue).into()));
        self
    }

    #[must_use]
    pub fn bg_moonlight_blue(mut self) -> Self {
        self.styles
            .push(ASTStyle::Background(tui_color!(moonlight_blue).into()));
        self
    }
}

/// This enum isn't the same as the [`TuiStyle`] struct. This enum can only hold a single
/// variant. The [`TuiStyle`] struct can hold multiple variants. This is a low level enum
/// that shouldn't be used directly. It is best to use [`TuiStyle`] and
/// [`crate::new_style`!] to create a [`TuiStyle`] and convert it to this type using
/// `.into()`.
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

mod convert_vec_ast_style_to_tui_style {
    use super::{ASTStyle, ASTextStyles, Bold, Dim, Hidden, Italic, Reverse,
                Strikethrough, TuiStyle, Underline};

    impl From<ASTextStyles> for TuiStyle {
        fn from(styles: ASTextStyles) -> Self {
            let mut tui_style = TuiStyle::default();
            for style in styles {
                match style {
                    ASTStyle::Foreground(color) => {
                        tui_style.color_fg = Some(color.into());
                    }
                    ASTStyle::Background(color) => {
                        tui_style.color_bg = Some(color.into());
                    }
                    ASTStyle::Bold => tui_style.bold = Some(Bold),
                    ASTStyle::Dim => tui_style.dim = Some(Dim),
                    ASTStyle::Italic => tui_style.italic = Some(Italic),
                    ASTStyle::Underline => tui_style.underline = Some(Underline),
                    ASTStyle::Invert => tui_style.reverse = Some(Reverse),
                    ASTStyle::Hidden => tui_style.hidden = Some(Hidden),
                    ASTStyle::Strikethrough => {
                        tui_style.strikethrough = Some(Strikethrough);
                    }
                    // TuiStyle doesn't have direct equivalents for these:
                    ASTStyle::Overline | ASTStyle::RapidBlink | ASTStyle::SlowBlink => {}
                }
            }
            tui_style
        }
    }
}

mod convert_tui_style_to_vec_ast_style {
    use super::{ASTStyle, TuiStyle, sizing, sizing::InlineVecASTextStyles};

    impl From<TuiStyle> for sizing::InlineVecASTextStyles {
        fn from(tui_style: TuiStyle) -> Self {
            let mut styles = InlineVecASTextStyles::new();
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
    use super::{ASTStyle, BufTextStorage, Display, Formatter, Result, WriteToBuf};

    impl Display for ASTStyle {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Delegate to WriteToBuf for consistency.
            let mut acc = BufTextStorage::new();
            self.write_to_buf(&mut acc)?;
            self.write_buf_to_fmt(&acc, f)
        }
    }
}

impl WriteToBuf for ASTStyle {
    fn write_to_buf(&self, buf: &mut BufTextStorage) -> Result {
        use super::{ColorSupport, TransformColor, global_color_support};

        // Helper function to convert color to appropriate SgrCode.
        fn color_to_sgr(
            color_support: ColorSupport,
            color: ASTColor,
            is_foreground: bool,
        ) -> SgrCode {
            match color_support {
                ColorSupport::Ansi256 => {
                    let ansi = color.as_ansi();
                    if is_foreground {
                        SgrCode::ForegroundAnsi256(ansi.index)
                    } else {
                        SgrCode::BackgroundAnsi256(ansi.index)
                    }
                }
                ColorSupport::Grayscale => {
                    let gray = color.as_grayscale();
                    if is_foreground {
                        SgrCode::ForegroundAnsi256(gray.index)
                    } else {
                        SgrCode::BackgroundAnsi256(gray.index)
                    }
                }
                _ => {
                    let rgb = color.as_rgb();
                    if is_foreground {
                        SgrCode::ForegroundRGB(rgb.red, rgb.green, rgb.blue)
                    } else {
                        SgrCode::BackgroundRGB(rgb.red, rgb.green, rgb.blue)
                    }
                }
            }
        }

        let color_support = global_color_support::detect();

        match self {
            ASTStyle::Foreground(color) => {
                color_to_sgr(color_support, *color, true).write_to_buf(buf)
            }
            ASTStyle::Background(color) => {
                color_to_sgr(color_support, *color, false).write_to_buf(buf)
            }
            ASTStyle::Bold => SgrCode::Bold.write_to_buf(buf),
            ASTStyle::Dim => SgrCode::Dim.write_to_buf(buf),
            ASTStyle::Italic => SgrCode::Italic.write_to_buf(buf),
            ASTStyle::Underline => SgrCode::Underline.write_to_buf(buf),
            ASTStyle::SlowBlink => SgrCode::SlowBlink.write_to_buf(buf),
            ASTStyle::RapidBlink => SgrCode::RapidBlink.write_to_buf(buf),
            ASTStyle::Invert => SgrCode::Invert.write_to_buf(buf),
            ASTStyle::Hidden => SgrCode::Hidden.write_to_buf(buf),
            ASTStyle::Strikethrough => SgrCode::Strikethrough.write_to_buf(buf),
            ASTStyle::Overline => SgrCode::Overline.write_to_buf(buf),
        }
    }
}

impl WriteToBuf for ASText {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
        // Write all styles to buffer.
        for style in &self.styles {
            style.write_to_buf(acc)?;
        }

        // Write text content.
        acc.push_str(&self.text);

        // Write reset code.
        SgrCode::Reset.write_to_buf(acc)?;

        Ok(())
    }
}

mod display_trait_impl {
    use super::{ASText, BufTextStorage, Display, Formatter, Result, WriteToBuf};

    impl Display for ASText {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            // Use BufTextStorage buffer for building the complete output.
            let mut acc = BufTextStorage::new();
            self.write_to_buf(&mut acc)?;

            // Single write to formatter.
            self.write_buf_to_fmt(&acc, f)
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serial_test::serial;
    use smallvec::smallvec;

    use super::dim;
    use crate::{ASTColor, ASTStyle, ASText, ASTextStyles, ColIndex, ColorSupport,
                InlineVec, PixelChar, TuiColor, TuiStyle,
                ansi::sizing::InlineVecASTextStyles,
                ansi_styled_text::ansi_styled_text_impl::ASTextConvertOptions,
                global_color_support, tui_color,
                tui_style::tui_style_attrib::{Bold, Dim, Hidden, Italic, Reverse,
                                              Strikethrough, Underline},
                width};

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
            let ast_styles: InlineVecASTextStyles = tui_style.into();
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
            let ast_styles: InlineVecASTextStyles = tui_style.into();
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
            let ast_styles: InlineVecASTextStyles = tui_style.into();
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
            let ast_styles: InlineVecASTextStyles = tui_style.into();
            assert!(ast_styles.is_empty());
        }
    }

    #[serial]
    #[test]
    fn test_fg_color_on_bg_color() {
        let eg_1 = ASText {
            text: "Hello".into(),
            styles: smallvec!(
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((0, 0, 0).into())),
            ),
        };
        println!("{eg_1:?}");
        println!("{eg_1}");
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "Hello", styles: [Bold, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 }))] }"#
        );

        let eg_2 = eg_1.bg_dark_gray();
        println!("{eg_2:?}");
        println!("{eg_2}");
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
        println!("{eg_1:?}");
        println!("{eg_1}");
        assert_eq!(
            format!("{:?}", eg_1),
            r#"AnsiStyledText { text: "hello", styles: [Dim, Foreground(Rgb(RgbValue { red: 0, green: 0, blue: 0 })), Background(Rgb(RgbValue { red: 1, green: 1, blue: 1 }))] }"#
        );
    }

    #[serial]
    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_formatted_string_creation_ansi256() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Ansi256);
        let eg_1 = ASText {
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

        let eg_2 = ASText {
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
    #[allow(clippy::missing_errors_doc)]
    fn test_formatted_string_creation_truecolor() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Truecolor);
        let eg_1 = ASText {
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

        let eg_2 = ASText {
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
    #[allow(clippy::missing_errors_doc)]
    fn test_formatted_string_creation_grayscale() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Grayscale);
        let eg_1 = ASText {
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

        let eg_2 = ASText {
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

    #[serial]
    #[test]
    fn test_convert_vec_ast_style_to_tui_style() {
        // Test case 1: Mix of styles
        let ast_styles_1: ASTextStyles = smallvec![
            ASTStyle::Bold,
            ASTStyle::Foreground(ASTColor::Ansi(196.into())), // Red
            ASTStyle::Italic,
            ASTStyle::Background(ASTColor::Rgb((50, 50, 50).into())), // Dark Gray RGB
            ASTStyle::Underline,
            ASTStyle::Overline, // This should be ignored
        ];
        let expected_tui_style_1 = TuiStyle {
            bold: Some(Bold),
            italic: Some(Italic),
            underline: Some(Underline),
            color_fg: Some(TuiColor::Ansi(196.into())),
            color_bg: Some(TuiColor::Rgb((50, 50, 50).into())),
            ..Default::default()
        };
        let converted_tui_style_1: TuiStyle = ast_styles_1.into();
        assert_eq!(converted_tui_style_1, expected_tui_style_1);

        // Test case 2: Only attributes
        let ast_styles_2: ASTextStyles = smallvec![
            ASTStyle::Dim,
            ASTStyle::Strikethrough,
            ASTStyle::Hidden,
            ASTStyle::RapidBlink, // This should be ignored
        ];
        let expected_tui_style_2 = TuiStyle {
            dim: Some(Dim),
            strikethrough: Some(Strikethrough),
            hidden: Some(Hidden),
            ..Default::default()
        };
        let converted_tui_style_2: TuiStyle = ast_styles_2.into();
        assert_eq!(converted_tui_style_2, expected_tui_style_2);

        // Test case 3: Only colors
        let ast_styles_3: ASTextStyles = smallvec![
            ASTStyle::Foreground(ASTColor::Ansi(34.into())), // Green
            ASTStyle::Background(ASTColor::Ansi(226.into())), // Yellow
        ];
        let expected_tui_style_3 = TuiStyle {
            color_fg: Some(TuiColor::Ansi(34.into())),
            color_bg: Some(TuiColor::Ansi(226.into())),
            ..Default::default()
        };
        let converted_tui_style_3: TuiStyle = ast_styles_3.into();
        assert_eq!(converted_tui_style_3, expected_tui_style_3);

        // Test case 4: Empty styles
        let ast_styles_4: ASTextStyles = smallvec![];
        let expected_tui_style_4 = TuiStyle::default();
        let converted_tui_style_4: TuiStyle = ast_styles_4.into();
        assert_eq!(converted_tui_style_4, expected_tui_style_4);

        // Test case 5: Invert style
        let ast_styles_5: ASTextStyles = smallvec![ASTStyle::Invert];
        let expected_tui_style_5 = TuiStyle {
            reverse: Some(Reverse),
            ..Default::default()
        };
        let converted_tui_style_5: TuiStyle = ast_styles_5.into();
        assert_eq!(converted_tui_style_5, expected_tui_style_5);
    }

    #[serial]
    #[test]
    fn test_ast_convert_options_struct() {
        let options1 = ASTextConvertOptions {
            start: Some(ColIndex::new(5)),
            end: Some(ColIndex::new(10)),
        };
        assert_eq!(options1.start, Some(ColIndex::new(5)));
        assert_eq!(options1.end, Some(ColIndex::new(10)));

        let options2 = ASTextConvertOptions {
            start: None,
            end: None,
        };
        assert_eq!(options2.start, None);
        assert_eq!(options2.end, None);
    }

    #[serial]
    #[test]
    fn test_from_col_width_for_ast_convert_options() {
        let col_width = width(20);
        let options: ASTextConvertOptions = col_width.into();
        assert_eq!(options.start, Some(ColIndex::new(0)));
        // ColWidth 20 means indices 0-19.
        assert_eq!(options.end, Some(ColIndex::new(19)));

        let col_width_zero = width(0);
        let options_zero: ASTextConvertOptions = col_width_zero.into();
        assert_eq!(options_zero.start, Some(ColIndex::new(0)));
        // ColWidth(0) converts to ColIndex(0), which is technically index 0.
        assert_eq!(options_zero.end, Some(ColIndex::new(0)));
    }

    #[serial]
    #[test]
    fn test_ast_convert_method() {
        let tui_style = TuiStyle {
            bold: Some(Bold),
            color_fg: Some(TuiColor::Ansi(196.into())), // Red.
            ..Default::default()
        };
        let ast_style_vec: ASTextStyles = tui_style.into();

        let styled_text = ASText {
            text: "Hello World".into(),
            styles: ast_style_vec.clone(),
        };

        // Test case 1: Using From<ColWidth>
        {
            let col_width = width(5);
            let res: InlineVec<PixelChar> = styled_text.convert(col_width);
            assert_eq!(res.len(), 5); // "Hello"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[1],
                PixelChar::PlainText {
                    display_char: 'e',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[2],
                PixelChar::PlainText {
                    display_char: 'l',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[3],
                PixelChar::PlainText {
                    display_char: 'l',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'o',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 2: Convert full text (None, None)
        {
            let res: InlineVec<PixelChar> =
                styled_text.convert(ASTextConvertOptions::default());
            assert_eq!(res.len(), 11);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[10],
                PixelChar::PlainText {
                    display_char: 'd',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 3: Convert partial text (start specified)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(6)),
                end: None,
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 5); // "World"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'W',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'd',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 4: Convert partial text (end specified)
        {
            let opt = ASTextConvertOptions {
                start: None,
                end: Some(ColIndex::new(4)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 5); // "Hello"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'o',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 5: Convert partial text (start and end specified)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(2)),
                end: Some(ColIndex::new(8)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 7); // "llo Wor"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'l',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[6],
                PixelChar::PlainText {
                    display_char: 'r',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 6: Empty text
        {
            let empty_text = ASText {
                text: "".into(),
                styles: ast_style_vec.clone(),
            };
            let res: InlineVec<PixelChar> =
                empty_text.convert(ASTextConvertOptions::default());
            assert!(res.is_empty());
        }

        // Test case 7: No styles
        {
            let no_style_text = ASText {
                text: "Test".into(),
                styles: smallvec![],
            };
            let res: InlineVec<PixelChar> =
                no_style_text.convert(ASTextConvertOptions::default());
            assert_eq!(res.len(), 4);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'T',
                    maybe_style: None
                }
            );
            assert_eq!(
                res[3],
                PixelChar::PlainText {
                    display_char: 't',
                    maybe_style: None
                }
            );
        }

        // Test case 8: Invalid range (start > end)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(5)),
                end: Some(ColIndex::new(3)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert!(res.is_empty());
        }

        // Test case 9: Invalid range (start out of bounds)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(11)),
                end: Some(ColIndex::new(12)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert!(res.is_empty());
        }

        // Test case 10: Invalid range (end out of bounds, but start is valid)
        // The current implementation returns empty if end >= len()
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(8)),
                end: Some(ColIndex::new(11)), /* index 11 is out of bounds for "Hello
                                               * World" (len 11) */
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert!(res.is_empty());
        }

        // Test case 10.1: Valid range, end is last index
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(8)),
                end: Some(ColIndex::new(10)), // index 10 is the last valid index
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 3); // "rld"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'r',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[2],
                PixelChar::PlainText {
                    display_char: 'd',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 11: Single character range
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(6)),
                end: Some(ColIndex::new(6)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 1); // "W"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'W',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 12: Unicode characters
        {
            let unicode_text = ASText {
                text: "你好世界".into(), // "Hello World" in Chinese
                styles: ast_style_vec.clone(),
            };
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(1)),
                end: Some(ColIndex::new(2)),
            };
            let res: InlineVec<PixelChar> = unicode_text.convert(opt);
            assert_eq!(res.len(), 2); // "好世"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: '好',
                    maybe_style: Some(tui_style)
                }
            );
            assert_eq!(
                res[1],
                PixelChar::PlainText {
                    display_char: '世',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 13: ColWidth(0)
        {
            let col_width_zero = width(0);
            let res: InlineVec<PixelChar> = styled_text.convert(col_width_zero);
            // start=0, end=0 -> should return the first char
            assert_eq!(res.len(), 1);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    maybe_style: Some(tui_style)
                }
            );
        }

        // Test case 14: ColWidth(1)
        {
            let col_width_one = width(1);
            let res: InlineVec<PixelChar> = styled_text.convert(col_width_one);
            // start=0, end=0 -> should return the first char
            assert_eq!(res.len(), 1);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    maybe_style: Some(tui_style)
                }
            );
        }
    }

    #[serial]
    #[test]
    fn test_ast_clip() {
        let tui_style = TuiStyle {
            bold: Some(Bold),
            color_fg: Some(TuiColor::Ansi(196.into())), // Red.
            ..Default::default()
        };
        let ast_style_vec: ASTextStyles = tui_style.into();

        let styled_text = ASText {
            text: "Hello World".into(),
            styles: ast_style_vec.clone(),
        };

        // Test case 1: Using From<ColWidth>
        {
            let col_width = width(4);
            let clipped_text = styled_text.clip(col_width);
            assert_eq!(clipped_text.text, "Hell");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 2: Clip full text (None, None)
        {
            let clipped_text = styled_text.clip(ASTextConvertOptions::default());
            assert_eq!(clipped_text.text, "Hello World");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 3: Clip partial text (start specified)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(6)),
                end: None,
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "World");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 4: Clip partial text (end specified)
        {
            let opt = ASTextConvertOptions {
                start: None,
                end: Some(ColIndex::new(4)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "Hello");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 5: Clip partial text (start and end specified)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(2)),
                end: Some(ColIndex::new(8)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "llo Wor");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 6: Empty text
        {
            let empty_text = ASText {
                text: "".into(),
                styles: ast_style_vec.clone(),
            };
            let clipped_text = empty_text.clip(ASTextConvertOptions::default());
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.styles, empty_text.styles);
        }

        // Test case 7: No styles
        {
            let no_style_text = ASText {
                text: "Test".into(),
                styles: smallvec![],
            };
            let clipped_text = no_style_text.clip(ASTextConvertOptions::default());
            assert_eq!(clipped_text.text, "Test");
            assert_eq!(clipped_text.styles, no_style_text.styles);
        }

        // Test case 8: Invalid range (start > end)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(5)),
                end: Some(ColIndex::new(3)),
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 9: Invalid range (start out of bounds)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(11)),
                end: Some(ColIndex::new(12)),
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 10: Invalid range (end out of bounds, but start is valid)
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(8)),
                end: Some(ColIndex::new(11)), // index 11 is out of bounds
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty()); // convert returns empty for invalid end
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 10.1: Valid range, end is last index
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(8)),
                end: Some(ColIndex::new(10)), // index 10 is the last valid index
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "rld");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 11: Single character range
        {
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(6)),
                end: Some(ColIndex::new(6)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "W");
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 12: Unicode characters
        {
            let unicode_text = ASText {
                text: "你好世界".into(), // "Hello World" in Chinese
                styles: ast_style_vec.clone(),
            };
            let opt = ASTextConvertOptions {
                start: Some(ColIndex::new(1)),
                end: Some(ColIndex::new(2)),
            };
            let clipped_text = unicode_text.clip(opt);
            assert_eq!(clipped_text.text, "好世");
            assert_eq!(clipped_text.styles, unicode_text.styles);
        }

        // Test case 13: ColWidth(0)
        {
            let col_width_zero = width(0);
            let clipped_text = styled_text.clip(col_width_zero);
            assert_eq!(clipped_text.text, "H"); // start=0, end=0 -> first char
            assert_eq!(clipped_text.styles, styled_text.styles);
        }

        // Test case 14: ColWidth(1)
        {
            let col_width_one = width(1);
            let clipped_text = styled_text.clip(col_width_one);
            assert_eq!(clipped_text.text, "H"); // start=0, end=0 -> first char
            assert_eq!(clipped_text.styles, styled_text.styles);
        }
    }
}

#[cfg(test)]
mod bench_tests {
    extern crate test;
    use smallvec::smallvec;
    use test::Bencher;

    use super::*;

    // Benchmark data setup
    fn simple_text() -> ASText {
        ASText {
            text: "Hello, World!".into(),
            styles: smallvec![],
        }
    }

    fn single_style_text() -> ASText {
        ASText {
            text: "Hello, World!".into(),
            styles: smallvec![ASTStyle::Bold],
        }
    }

    fn multiple_styles_text() -> ASText {
        ASText {
            text: "Hello, World!".into(),
            styles: smallvec![ASTStyle::Bold, ASTStyle::Italic, ASTStyle::Underline,],
        }
    }

    fn colored_text() -> ASText {
        ASText {
            text: "Hello, World!".into(),
            styles: smallvec![
                ASTStyle::Foreground(ASTColor::Ansi(196.into())),
                ASTStyle::Background(ASTColor::Ansi(236.into())),
            ],
        }
    }

    fn rgb_colored_text() -> ASText {
        ASText {
            text: "Hello, World!".into(),
            styles: smallvec![
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Rgb((255, 0, 0).into())),
                ASTStyle::Background(ASTColor::Rgb((0, 0, 255).into())),
            ],
        }
    }

    fn complex_styled_text() -> ASText {
        ASText {
            text: "Hello, World! This is a longer text with more content.".into(),
            styles: smallvec![
                ASTStyle::Bold,
                ASTStyle::Italic,
                ASTStyle::Underline,
                ASTStyle::Foreground(ASTColor::Rgb((255, 128, 0).into())),
                ASTStyle::Background(ASTColor::Ansi(236.into())),
            ],
        }
    }

    fn long_text() -> ASText {
        ASText {
            // <!-- cspell:disable -->
            text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                   Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
                .into(),
            // <!-- cspell:enable -->
            styles: smallvec![
                ASTStyle::Bold,
                ASTStyle::Foreground(ASTColor::Ansi(34.into())),
            ],
        }
    }

    // Display benchmarks
    #[bench]
    fn bench_display_simple_text(b: &mut Bencher) {
        let ast = simple_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_single_style(b: &mut Bencher) {
        let ast = single_style_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_multiple_styles(b: &mut Bencher) {
        let ast = multiple_styles_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_ansi_colors(b: &mut Bencher) {
        let ast = colored_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_rgb_colors(b: &mut Bencher) {
        let ast = rgb_colored_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_complex_styled(b: &mut Bencher) {
        let ast = complex_styled_text();
        b.iter(|| format!("{ast}"));
    }

    #[bench]
    fn bench_display_long_text(b: &mut Bencher) {
        let ast = long_text();
        b.iter(|| format!("{ast}"));
    }

    // Benchmark creating styled text and displaying
    #[bench]
    fn bench_create_and_display_fg_red(b: &mut Bencher) {
        b.iter(|| {
            let ast = fg_red("Hello, World!");
            format!("{ast}")
        });
    }

    #[bench]
    fn bench_create_and_display_complex(b: &mut Bencher) {
        b.iter(|| {
            let ast = bold("Hello, World!")
                .fg_color(tui_color!(255, 0, 0))
                .bg_color(tui_color!(0, 0, 255));
            format!("{ast}")
        });
    }

    // Benchmark multiple ASText in sequence (simulating real usage)
    #[bench]
    fn bench_display_multiple_ast_sequence(b: &mut Bencher) {
        let texts = vec![
            fg_red("Error: "),
            bold("Failed to compile"),
            dim(" at line "),
            fg_yellow("42"),
        ];

        b.iter(|| {
            let mut result = String::new();
            for ast in &texts {
                use std::fmt::Write;
                write!(result, "{ast}").unwrap();
            }
            result
        });
    }
}
