// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{BufTextStorage, ColIndex, ColWidth, FastStringify, GCStringOwned,
            InlineString, InlineVec, PixelChar, PixelCharRenderer, SgrCode, TuiColor,
            TuiStyle, TuiStyleAttribs, UNICODE_REPLACEMENT_CHAR,
            cli_text_inline_impl::CliTextConvertOptions,
            generate_impl_display_for_fast_stringify, inline_string, tui_color,
            tui_style_attrib::{Bold, Dim, Italic, Strikethrough, Underline}};
use std::fmt::Result;
use strum_macros::EnumCount;

/// Please don't create this struct directly, use [`cli_text_inline`], [`cli_text_line!`],
/// [`cli_text_lines!`] or the constructor functions like [`fg_red`], [`fg_green`],
/// [`fg_blue`], etc.
///
/// [`CliTextInline`] represents a **text fragment** that can appear inline within a line.
/// Multiple fragments combine into a [`CliTextLine`], and multiple lines combine into
/// [`CliTextLines`]. This structure is optimized for stack allocation to avoid heap
/// overhead for typical CLI text.
///
/// The struct has four fields:
/// - [`text`] - the text content to display.
/// - [`attribs`] - text attributes (bold, italic, dim, underline, etc.) to apply to the
///   text.
/// - [`color_fg`] - optional foreground color.
/// - [`color_bg`] - optional background color.
///
/// Once created, either directly or using constructor functions like [`fg_red`], you
/// can then use [`bg_dark_gray`] to add a background color to the text.
/// If you want even more flexibility you can use constructor function [`fg_color`]
/// and [`bg_color`] to create a styled text with a specific RGB color.
///
/// # Example usage:
///
/// ```
/// # use r3bl_tui::{
/// #     TuiStyle, tui_color, new_style,
/// #     cli_text_inline, fg_red, dim, CliTextInline, fg_color,
/// #     TuiColor, TuiStyleAttribs,
/// # };
///
/// // Use [`cli_text_inline`] to create a styled text. Use this.
/// let styled_text = cli_text_inline("Hello", new_style!(bold));
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
/// // Struct construction.
/// CliTextInline {
///     text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.".into(),
///     attribs: TuiStyleAttribs::default(),
///     color_fg: Some(TuiColor::Rgb((50, 50, 50).into())),
///     color_bg: Some(TuiColor::Rgb((100, 200, 1).into())),
/// }
/// .println();
/// ```
///
/// [`text`]: Self::text
/// [`attribs`]: Self::attribs
/// [`color_fg`]: Self::color_fg
/// [`color_bg`]: Self::color_bg
/// [`cli_text_inline`]: crate::cli_text_inline
/// [`cli_text_line!`]: crate::cli_text_line
/// [`cli_text_lines!`]: crate::cli_text_lines
/// [`fg_red`]: crate::fg_red
/// [`fg_green`]: crate::fg_green
/// [`fg_blue`]: crate::fg_blue
/// [`bg_dark_gray`]: Self::bg_dark_gray
/// [`bg_color`]: Self::bg_color
/// [`fg_color`]: crate::fg_color
/// [`CliTextLine`]: crate::CliTextLine
/// [`CliTextLines`]: crate::CliTextLines
/// [`TuiStyleAttribs`]: crate::TuiStyleAttribs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliTextInline {
    pub text: InlineString,
    pub attribs: TuiStyleAttribs,
    pub color_fg: Option<TuiColor>,
    pub color_bg: Option<TuiColor>,
}

// Type aliases for better readability.

pub type CliTextLine = InlineVec<CliTextInline>;
pub type CliTextLines = InlineVec<CliTextLine>;

/// Easy to use constructor function, instead of creating a new [`CliTextInline`] struct
/// directly. If you need to assemble a bunch of these together, you can use
/// [`crate::cli_text_line!`] to create a list of them.
#[must_use]
pub fn cli_text_inline(
    arg_text: impl AsRef<str>,
    arg_style: impl Into<TuiStyle>,
) -> CliTextInline {
    let style: TuiStyle = arg_style.into();
    CliTextInline {
        text: arg_text.as_ref().into(),
        attribs: style.attribs,
        color_fg: style.color_fg,
        color_bg: style.color_bg,
    }
}

/// String together a bunch of [`CliTextInline`] structs into a single
/// [`crate::InlineVec<CliTextInline>`]. This is useful for creating a list of
/// [`CliTextInline`] structs that can be printed on a single line.
#[macro_export]
macro_rules! cli_text_line {
    (
        $( $cli_text_chunk:expr ),* $(,)?
    ) => {{
        use $crate::{InlineVec, CliTextLine};
        let mut acc: CliTextLine = InlineVec::new();
        $(
            acc.push($cli_text_chunk);
        )*
        acc
    }};
}

/// String together a bunch of formatted lines into a single
/// [`crate::InlineVec<InlineVec<CliTextInline>>`]. This is useful for assembling
/// multiline formatted text which is used in multi line headers, for example.
#[macro_export]
macro_rules! cli_text_lines {
    (
        $( $cli_text_line:expr ),* $(,)?
    ) => {{
        use $crate::{InlineVec, CliTextLines};
        let mut acc: CliTextLines = InlineVec::new();
        $(
            acc.push($cli_text_line);
        )*
        acc
    }};
}

pub mod cli_text_inline_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Options for converting or clipping [`CliTextInline`] to a range.
    ///
    /// Uses (start, width) semantics with display-width awareness to correctly handle
    /// Unicode characters of varying widths (e.g., CJK characters that occupy 2 columns):
    /// - `start`: Display column index where to start (0-based)
    /// - `width`: Display width in columns to include, or None for "to end of text"
    ///
    /// # Display Width Handling
    /// This uses the same semantics as [`GCStringOwned::clip()`], accounting for:
    /// - Wide characters (CJK) that occupy multiple columns
    /// - Zero-width characters (combining marks)
    /// - Accurate terminal column positioning
    ///
    /// # Examples
    /// - ASCII text "Hello World": Each char is 1 column wide
    ///   - `start=0, width=5` → "Hello"
    /// - CJK text "你好世界": Each char is 2 columns wide
    ///   - `start=0, width=2` → "你" (starts at col 0, takes 2 cols)
    ///   - `start=2, width=4` → "好世" (starts at col 2, takes 4 cols)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct CliTextConvertOptions {
        pub start: ColIndex,
        pub width: Option<ColWidth>,
    }

    impl Default for CliTextConvertOptions {
        fn default() -> Self {
            Self {
                start: 0.into(),
                width: None, // Full text from start
            }
        }
    }

    impl From<ColWidth> for CliTextConvertOptions {
        fn from(col_width: ColWidth) -> Self {
            Self {
                start: 0.into(),
                width: Some(col_width),
            }
        }
    }

    impl CliTextInline {
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
        /// Uses (start, width) semantics where:
        /// - `start`: 0-based index of the first character to include
        /// - `width`: Number of characters to include, or None for "to end of text"
        ///
        /// This is optimized to avoid the wasteful
        /// [`convert()`](Self::convert) call by
        /// directly slicing the text using grapheme clustering.
        #[must_use]
        pub fn clip(
            &self,
            arg_options: impl Into<CliTextConvertOptions>,
        ) -> CliTextInline {
            let CliTextConvertOptions { start, width } = arg_options.into();

            // Early return if text is empty
            if self.text.is_empty() {
                return CliTextInline {
                    text: InlineString::new(),
                    attribs: self.attribs,
                    color_fg: self.color_fg,
                    color_bg: self.color_bg,
                };
            }

            // Use GCStringOwned to slice the text directly without creating PixelChars
            let gc_string = GCStringOwned::from(&self.text);
            let start_index = start.as_usize();
            let gc_len = gc_string.len().as_usize();
            let width_count = match width {
                None => gc_len.saturating_sub(start_index),
                Some(w) => w.as_usize(),
            };
            let end_index = (start_index + width_count).min(gc_len);

            // Early return if start is out of bounds
            if start_index >= gc_len {
                return CliTextInline {
                    text: InlineString::new(),
                    attribs: self.attribs,
                    color_fg: self.color_fg,
                    color_bg: self.color_bg,
                };
            }

            // Build the sliced text directly
            let mut clipped_text = InlineString::with_capacity(self.text.len());
            for (idx, item) in gc_string.iter().enumerate() {
                if idx >= start_index && idx < end_index {
                    let segment_str = item.get_str(&gc_string);
                    for c in segment_str.chars() {
                        clipped_text.push(c);
                    }
                }
            }

            CliTextInline {
                text: clipped_text,
                attribs: self.attribs,
                color_fg: self.color_fg,
                color_bg: self.color_bg,
            }
        }

        /// Converts the text to a vector of [`PixelChar`]s. This is used for rendering
        /// the text on the screen.
        /// - To convert the entire text, just pass in
        ///   [`CliTextConvertOptions::default()`].
        /// - To clip the text to a certain display width, pass in the [`ColWidth`] via
        ///   [`From<ColWidth>`] or explicitly set the
        ///   [`width`](CliTextConvertOptions::width) field.
        /// - Uses (start, width) semantics where:
        ///   - `start`: 0-based display column index of the first column to include
        ///   - `width`: Display width in columns to include, or None for "to end of text"
        ///
        /// # Display Width Handling
        /// This method uses display-width-aware clipping to correctly handle:
        /// - Wide Unicode characters (e.g., CJK characters that occupy 2 columns)
        /// - Zero-width characters (combining marks, ZWJ sequences)
        /// - Accurate terminal rendering positioning
        pub fn convert(
            &self,
            arg_options: impl Into<CliTextConvertOptions>,
        ) -> InlineVec<PixelChar> {
            let CliTextConvertOptions { start, width } = arg_options.into();

            // 1. Early return if the text is empty.
            if self.text.is_empty() {
                return InlineVec::new();
            }

            // 2. Create TuiStyle from the struct fields.
            let tui_style = TuiStyle {
                attribs: self.attribs,
                color_fg: self.color_fg,
                color_bg: self.color_bg,
                ..Default::default()
            };

            // 3. Create GCStringOwned for display-width-aware clipping.
            let gc_string = GCStringOwned::from(&self.text);

            // 4. Use display-width-aware clipping via GCStringOwned::clip().
            // This correctly handles wide characters and display column positioning.
            let clip_width = width.unwrap_or_else(|| {
                // If no width specified, use the actual display width of the entire
                // string. This ensures all grapheme clusters are
                // included, correctly handling:
                // - Wide Unicode characters (emoji, CJK) that occupy 2+ columns
                // - Zero-width characters (combining marks)
                // - Accurate terminal column positioning
                gc_string.display_width()
            });
            let clipped_str = gc_string.clip(start, clip_width);

            // 5. Create a new GCStringOwned from the clipped string to preserve
            // grapheme cluster information.
            let gc_clipped = GCStringOwned::from(clipped_str);

            // 6. Convert each grapheme cluster segment to a PixelChar.
            gc_clipped
                .seg_iter()
                .map(|seg| {
                    let segment_str = seg.get_str(&gc_clipped);
                    let display_char = segment_str
                        .chars()
                        .next()
                        .unwrap_or(UNICODE_REPLACEMENT_CHAR);
                    PixelChar::PlainText {
                        display_char,
                        style: tui_style,
                    }
                })
                .collect()
        }
    }
}

// The following functions are convenience functions for providing ANSI attributes.

#[must_use]
pub fn bold(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Bold.into(),
        color_fg: None,
        color_bg: None,
    }
}

#[must_use]
pub fn italic(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Italic.into(),
        color_fg: None,
        color_bg: None,
    }
}

#[must_use]
pub fn underline(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Underline.into(),
        color_fg: None,
        color_bg: None,
    }
}

#[must_use]
pub fn strikethrough(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Strikethrough.into(),
        color_fg: None,
        color_bg: None,
    }
}

#[must_use]
pub fn dim(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Dim.into(),
        color_fg: None,
        color_bg: None,
    }
}

#[must_use]
pub fn dim_underline(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: Dim + Underline,
        color_fg: None,
        color_bg: None,
    }
}

// The following function is a convenience function for providing any color.

#[must_use]
pub fn fg_color(arg_color: impl Into<TuiColor>, text: &str) -> CliTextInline {
    CliTextInline {
        text: text.into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(arg_color.into()),
        color_bg: None,
    }
}

// The following functions are convenience functions for providing ANSI colors.

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_dark_gray(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(236.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_black(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(0.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_yellow(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(226.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_green(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(34.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_blue(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(27.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_red(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(196.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_white(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(231.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_cyan(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(51.into())),
        color_bg: None,
    }
}

/// More info: <https://www.ditig.com/256-colors-cheat-sheet>
#[must_use]
pub fn fg_magenta(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(TuiColor::Ansi(201.into())),
        color_bg: None,
    }
}

// The following colors are a convenience for using the tui_color! macro.

#[must_use]
pub fn fg_medium_gray(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(medium_gray)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_light_cyan(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(light_cyan)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_light_purple(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(light_purple)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_deep_purple(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(deep_purple)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_soft_pink(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(soft_pink)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_hot_pink(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(hot_pink)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_light_yellow_green(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(light_yellow_green)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_dark_teal(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(dark_teal)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_bright_cyan(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(bright_cyan)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_dark_purple(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(dark_purple)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_sky_blue(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(sky_blue)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_lavender(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(lavender)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_dark_lizard_green(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(dark_lizard_green)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_orange(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(orange)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_silver_metallic(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(silver_metallic)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_lizard_green(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(lizard_green)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_pink(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(pink)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_dark_pink(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(dark_pink)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_frozen_blue(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(frozen_blue)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_guards_red(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(guards_red)),
        color_bg: None,
    }
}

#[must_use]
pub fn fg_slate_gray(text: impl AsRef<str>) -> CliTextInline {
    CliTextInline {
        text: text.as_ref().into(),
        attribs: TuiStyleAttribs::default(),
        color_fg: Some(tui_color!(slate_gray)),
        color_bg: None,
    }
}

impl CliTextInline {
    #[must_use]
    pub fn dim(mut self) -> Self {
        self.attribs += Dim;
        self
    }

    #[must_use]
    pub fn italic(mut self) -> Self {
        self.attribs += Italic;
        self
    }

    #[must_use]
    pub fn bold(mut self) -> Self {
        self.attribs += Bold;
        self
    }

    #[must_use]
    pub fn underline(mut self) -> Self {
        self.attribs += Underline;
        self
    }

    #[must_use]
    pub fn bg_color(mut self, arg_color: impl Into<TuiColor>) -> Self {
        let color: TuiColor = arg_color.into();
        self.color_bg = Some(color);
        self
    }

    #[must_use]
    pub fn fg_color(mut self, arg_color: impl Into<TuiColor>) -> Self {
        let color: TuiColor = arg_color.into();
        self.color_fg = Some(color);
        self
    }

    #[must_use]
    pub fn bg_cyan(mut self) -> Self {
        self.color_bg = Some(TuiColor::Ansi(51.into()));
        self
    }

    #[must_use]
    pub fn bg_yellow(mut self) -> Self {
        self.color_bg = Some(TuiColor::Ansi(226.into()));
        self
    }

    #[must_use]
    pub fn bg_green(mut self) -> Self {
        self.color_bg = Some(TuiColor::Ansi(34.into()));
        self
    }

    #[must_use]
    pub fn bg_slate_gray(mut self) -> Self {
        self.color_bg = Some(tui_color!(slate_gray));
        self
    }

    #[must_use]
    pub fn bg_dark_gray(mut self) -> Self {
        self.color_bg = Some(TuiColor::Ansi(236.into()));
        self
    }

    #[must_use]
    pub fn bg_night_blue(mut self) -> Self {
        self.color_bg = Some(tui_color!(night_blue));
        self
    }

    #[must_use]
    pub fn bg_moonlight_blue(mut self) -> Self {
        self.color_bg = Some(tui_color!(moonlight_blue));
        self
    }
}

/// This enum isn't the same as the [`TuiStyle`] struct. This enum can only hold a single
/// variant. The [`TuiStyle`] struct can hold multiple variants. This is a low level enum
/// that shouldn't be used directly. It is best to use [`TuiStyle`] and
/// [`crate::new_style`!] to create a [`TuiStyle`] and convert it
/// to this type using `.into()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumCount)]
pub enum CliStyle {
    Foreground(TuiColor),
    Background(TuiColor),
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

impl FastStringify for CliStyle {
    fn write_to_buf(&self, buf: &mut BufTextStorage) -> Result {
        use crate::core::ansi::{ColorSupport, TransformColor, global_color_support};

        // Helper function to convert color to appropriate
        // SgrCode.
        fn color_to_sgr(
            color_support: ColorSupport,
            color: TuiColor,
            is_foreground: bool,
        ) -> SgrCode {
            match color_support {
                ColorSupport::Ansi256 => {
                    let ansi = color.as_ansi();
                    if is_foreground {
                        SgrCode::ForegroundAnsi256(ansi)
                    } else {
                        SgrCode::BackgroundAnsi256(ansi)
                    }
                }
                ColorSupport::Grayscale => {
                    let gray = color.as_grayscale();
                    if is_foreground {
                        SgrCode::ForegroundAnsi256(gray)
                    } else {
                        SgrCode::BackgroundAnsi256(gray)
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
            CliStyle::Foreground(color) => {
                color_to_sgr(color_support, *color, true).write_to_buf(buf)
            }
            CliStyle::Background(color) => {
                color_to_sgr(color_support, *color, false).write_to_buf(buf)
            }
            CliStyle::Bold => SgrCode::Bold.write_to_buf(buf),
            CliStyle::Dim => SgrCode::Dim.write_to_buf(buf),
            CliStyle::Italic => SgrCode::Italic.write_to_buf(buf),
            CliStyle::Underline => SgrCode::Underline.write_to_buf(buf),
            CliStyle::SlowBlink => SgrCode::SlowBlink.write_to_buf(buf),
            CliStyle::RapidBlink => SgrCode::RapidBlink.write_to_buf(buf),
            CliStyle::Invert => SgrCode::Invert.write_to_buf(buf),
            CliStyle::Hidden => SgrCode::Hidden.write_to_buf(buf),
            CliStyle::Strikethrough => SgrCode::Strikethrough.write_to_buf(buf),
            CliStyle::Overline => SgrCode::Overline.write_to_buf(buf),
        }
    }
}

generate_impl_display_for_fast_stringify!(CliStyle);

impl FastStringify for CliTextInline {
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
        // Convert to PixelChar array using the unified representation
        let pixels = self.convert(CliTextConvertOptions::default());

        // Use unified renderer for ANSI generation
        let mut renderer = PixelCharRenderer::new();
        let ansi_output = renderer.render_line(&pixels);

        // Write the ANSI-encoded output to the buffer
        // ansi_output is UTF-8 valid since it contains ANSI codes and UTF-8 characters
        acc.push_str(std::str::from_utf8(ansi_output).map_err(|_| std::fmt::Error)?);

        // Emit final reset code (consistent with old behavior - always emitted)
        SgrCode::Reset.write_to_buf(acc)?;

        Ok(())
    }
}

generate_impl_display_for_fast_stringify!(CliTextInline);

#[cfg(test)]
mod tests {
    use super::{cli_text_inline_impl::CliTextConvertOptions, dim};
    use crate::{CliTextInline, ColIndex, ColorSupport, InlineVec, PixelChar, TuiColor,
                TuiStyle, TuiStyleAttribs, global_color_support, tui_color,
                tui_style::tui_style_attrib::Bold, tui_style_attribs, width};
    use pretty_assertions::assert_eq;
    use serial_test::serial;

    #[serial]
    #[test]
    fn test_fg_color_on_bg_color() {
        let eg_1 = CliTextInline {
            text: "Hello".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Rgb((0, 0, 0).into())),
            color_bg: None,
        };
        println!("{eg_1:?}");
        println!("{eg_1}");
        // Just check it contains the expected parts
        let debug_str = format!("{eg_1:?}");
        assert!(debug_str.contains("text: \"Hello\""));
        assert!(debug_str.contains("bold: Some(Bold)"));
        assert!(debug_str.contains("color_fg: Some(0,0,0)"));
        assert!(debug_str.contains("color_bg: None"));

        let eg_2 = eg_1.bg_dark_gray();
        println!("{eg_2:?}");
        println!("{eg_2}");
        let debug_str_2 = format!("{eg_2:?}");
        assert!(debug_str_2.contains("text: \"Hello\""));
        assert!(debug_str_2.contains("bold: Some(Bold)"));
        assert!(debug_str_2.contains("color_fg: Some(0,0,0)"));
        assert!(debug_str_2.contains("color_bg: Some(ansi_value(236))"));
    }

    #[serial]
    #[test]
    fn test_fg_bg_combo() {
        let eg_1 = dim("hello")
            .fg_color(tui_color!(0, 0, 0))
            .bg_color(tui_color!(1, 1, 1));
        println!("{eg_1:?}");
        println!("{eg_1}");
        // Just check it contains the expected parts
        let debug_str = format!("{eg_1:?}");
        assert!(debug_str.contains("text: \"hello\""));
        assert!(debug_str.contains("dim: Some(Dim)"));
        assert!(debug_str.contains("color_fg: Some(0,0,0)"));
        assert!(debug_str.contains("color_bg: Some(1,1,1)"));
    }

    #[serial]
    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_formatted_string_creation_ansi256() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Ansi256);
        let eg_1 = CliTextInline {
            text: "Hello".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Rgb((0, 0, 0).into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;5;16m\x1b[48;5;16mHello\x1b[0m".to_string()
        );

        let eg_2 = CliTextInline {
            text: "World".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Ansi(150.into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
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
        let eg_1 = CliTextInline {
            text: "Hello".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Rgb((0, 0, 0).into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
        };

        assert_eq!(
            format!("{0}", eg_1),
            "\x1b[1m\x1b[38;2;0;0;0m\x1b[48;2;1;1;1mHello\x1b[0m".to_string()
        );

        let eg_2 = CliTextInline {
            text: "World".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Ansi(150.into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
        };

        assert_eq!(
            format!("{0}", eg_2),
            "\x1b[1m\x1b[38;5;150m\x1b[48;2;1;1;1mWorld\x1b[0m".to_string()
        );

        Ok(())
    }

    #[serial]
    #[test]
    #[allow(clippy::missing_errors_doc)]
    fn test_formatted_string_creation_grayscale() -> Result<(), String> {
        global_color_support::set_override(ColorSupport::Grayscale);
        let eg_1 = CliTextInline {
            text: "Hello".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Rgb((0, 0, 0).into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
        };

        println!("{:?}", format!("{0}", eg_1));

        assert_eq!(
            format!("{0}", eg_1),
            "\u{1b}[1m\u{1b}[38;5;16m\u{1b}[48;5;16mHello\u{1b}[0m".to_string()
        );

        let eg_2 = CliTextInline {
            text: "World".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Ansi(150.into())),
            color_bg: Some(TuiColor::Rgb((1, 1, 1).into())),
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
    fn test_ast_convert_options_struct() {
        let options1 = CliTextConvertOptions {
            start: ColIndex::new(5),
            width: Some(width(6)),
        };
        assert_eq!(options1.start, ColIndex::new(5));
        assert_eq!(options1.width, Some(width(6)));

        let options2 = CliTextConvertOptions {
            start: ColIndex::new(0),
            width: None,
        };
        assert_eq!(options2.start, ColIndex::new(0));
        assert_eq!(options2.width, None);
    }

    #[serial]
    #[test]
    fn test_from_col_width_for_ast_convert_options() {
        let col_width = width(20);
        let options: CliTextConvertOptions = col_width.into();
        assert_eq!(options.start, ColIndex::new(0));
        // ColWidth 20 should result in width(20).
        assert_eq!(options.width, Some(width(20)));

        let col_width_zero = width(0);
        let options_zero: CliTextConvertOptions = col_width_zero.into();
        assert_eq!(options_zero.start, ColIndex::new(0));
        // ColWidth(0) converts to width(0).
        assert_eq!(options_zero.width, Some(width(0)));
    }

    #[serial]
    #[test]
    fn test_ast_convert_method() {
        let tui_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(TuiColor::Ansi(196.into())), // Red.
            ..Default::default()
        };
        let styled_text = CliTextInline {
            text: "Hello World".into(),
            attribs: tui_style.attribs,
            color_fg: tui_style.color_fg,
            color_bg: tui_style.color_bg,
        };

        // Test case 1: Using From<ColWidth>.
        {
            let col_width = width(5);
            let res: InlineVec<PixelChar> = styled_text.convert(col_width);
            assert_eq!(res.len(), 5); // "Hello"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    style: tui_style
                }
            );
            assert_eq!(
                res[1],
                PixelChar::PlainText {
                    display_char: 'e',
                    style: tui_style
                }
            );
            assert_eq!(
                res[2],
                PixelChar::PlainText {
                    display_char: 'l',
                    style: tui_style
                }
            );
            assert_eq!(
                res[3],
                PixelChar::PlainText {
                    display_char: 'l',
                    style: tui_style
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: tui_style
                }
            );
        }

        // Test case 2: Convert full text (None, None).
        {
            let res: InlineVec<PixelChar> =
                styled_text.convert(CliTextConvertOptions::default());
            assert_eq!(res.len(), 11);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    style: tui_style
                }
            );
            assert_eq!(
                res[10],
                PixelChar::PlainText {
                    display_char: 'd',
                    style: tui_style
                }
            );
        }

        // Test case 3: Convert partial text (start specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(6),
                width: None,
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 5); // "World"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'W',
                    style: tui_style
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'd',
                    style: tui_style
                }
            );
        }

        // Test case 4: Convert partial text (end specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(5)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 5); // "Hello"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    style: tui_style
                }
            );
            assert_eq!(
                res[4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: tui_style
                }
            );
        }

        // Test case 5: Convert partial text (start and end specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(2),
                width: Some(width(7)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 7); // "llo Wor"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'l',
                    style: tui_style
                }
            );
            assert_eq!(
                res[6],
                PixelChar::PlainText {
                    display_char: 'r',
                    style: tui_style
                }
            );
        }

        // Test case 6: Empty text.
        {
            let empty_text = CliTextInline {
                text: "".into(),
                attribs: tui_style.attribs,
                color_fg: tui_style.color_fg,
                color_bg: tui_style.color_bg,
            };
            let res: InlineVec<PixelChar> =
                empty_text.convert(CliTextConvertOptions::default());
            assert!(res.is_empty());
        }

        // Test case 7: No styles.
        {
            let no_style_text = CliTextInline {
                text: "Test".into(),
                attribs: TuiStyleAttribs::default(),
                color_fg: None,
                color_bg: None,
            };
            let res: InlineVec<PixelChar> =
                no_style_text.convert(CliTextConvertOptions::default());
            assert_eq!(res.len(), 4);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'T',
                    style: TuiStyle::default()
                }
            );
            assert_eq!(
                res[3],
                PixelChar::PlainText {
                    display_char: 't',
                    style: TuiStyle::default()
                }
            );
        }

        // Test case 8: Out of bounds (start beyond text length).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(20),
                width: None,
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert!(res.is_empty());
        }

        // Test case 9: Out of bounds with width (start beyond text).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(15),
                width: Some(width(5)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert!(res.is_empty());
        }

        // Test case 10: Width exceeds remaining text.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(8),
                width: Some(width(10)), // Only 3 chars available ("rld")
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 3); // "rld" - clamps to available chars
        }

        // Test case 10.1: Width exactly matches remaining text.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(8),
                width: Some(width(3)), // Exactly "rld"
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 3); // "rld"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'r',
                    style: tui_style
                }
            );
            assert_eq!(
                res[2],
                PixelChar::PlainText {
                    display_char: 'd',
                    style: tui_style
                }
            );
        }

        // Test case 11: Single character range.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(6),
                width: Some(width(1)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            assert_eq!(res.len(), 1); // "W"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'W',
                    style: tui_style
                }
            );
        }

        // Test case 12: Unicode characters (display-width-aware).
        // "你好世界" - Each character is 2 columns wide
        // Layout: 你(cols 0-2), 好(cols 2-4), 世(cols 4-6), 界(cols 6-8)
        {
            let unicode_text = CliTextInline {
                text: "你好世界".into(),
                attribs: tui_style.attribs,
                color_fg: tui_style.color_fg,
                color_bg: tui_style.color_bg,
            };
            // Start at column 2 (beginning of "好"), take 4 columns ("好世")
            let opt = CliTextConvertOptions {
                start: ColIndex::new(2),
                width: Some(width(4)),
            };
            let res: InlineVec<PixelChar> = unicode_text.convert(opt);
            assert_eq!(res.len(), 2); // "好世"
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: '好',
                    style: tui_style
                }
            );
            assert_eq!(
                res[1],
                PixelChar::PlainText {
                    display_char: '世',
                    style: tui_style
                }
            );
        }

        // Test case 13: Width zero (single char at start).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(0)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            // start=0, width=0 -> should return 0 chars
            assert!(res.is_empty());
        }

        // Test case 14: Width one (single char at start).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(1)),
            };
            let res: InlineVec<PixelChar> = styled_text.convert(opt);
            // start=0, width=1 -> should return first char "H"
            assert_eq!(res.len(), 1);
            assert_eq!(
                res[0],
                PixelChar::PlainText {
                    display_char: 'H',
                    style: tui_style
                }
            );
        }

        // Test case 15: Full text conversion with None width uses display_width().
        // This is the key test demonstrating the fix: when width is None, it uses the
        // actual display width from GCStringOwned instead of a hardcoded 10_000.
        // The existing test case 12 (Unicode characters with CJK wide chars) validates
        // that display-width-aware handling works correctly for wide characters.
        {
            let full_text = styled_text.convert(CliTextConvertOptions::default());
            assert_eq!(full_text.len(), 11); // "Hello World"
        }
    }

    #[serial]
    #[test]
    fn test_ast_clip() {
        let tui_style = TuiStyle {
            attribs: tui_style_attribs(Bold),
            color_fg: Some(TuiColor::Ansi(196.into())), // Red.
            ..Default::default()
        };

        let styled_text = CliTextInline {
            text: "Hello World".into(),
            attribs: tui_style.attribs,
            color_fg: tui_style.color_fg,
            color_bg: tui_style.color_bg,
        };

        // Test case 1: Using From<ColWidth>.
        {
            let col_width = width(4);
            let clipped_text = styled_text.clip(col_width);
            assert_eq!(clipped_text.text, "Hell");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 2: Clip full text (None, None).
        {
            let clipped_text = styled_text.clip(CliTextConvertOptions::default());
            assert_eq!(clipped_text.text, "Hello World");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 3: Clip partial text (start specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(6),
                width: None,
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "World");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 4: Clip partial text (end specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(5)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "Hello");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 5: Clip partial text (start and end specified).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(2),
                width: Some(width(7)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "llo Wor");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 6: Empty text.
        {
            let empty_text = CliTextInline {
                text: "".into(),
                attribs: tui_style.attribs,
                color_fg: tui_style.color_fg,
                color_bg: tui_style.color_bg,
            };
            let clipped_text = empty_text.clip(CliTextConvertOptions::default());
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.attribs, empty_text.attribs);
            assert_eq!(clipped_text.color_fg, empty_text.color_fg);
            assert_eq!(clipped_text.color_bg, empty_text.color_bg);
        }

        // Test case 7: No styles.
        {
            let no_style_text = CliTextInline {
                text: "Test".into(),
                attribs: TuiStyleAttribs::default(),
                color_fg: None,
                color_bg: None,
            };
            let clipped_text = no_style_text.clip(CliTextConvertOptions::default());
            assert_eq!(clipped_text.text, "Test");
            assert_eq!(clipped_text.attribs, no_style_text.attribs);
            assert_eq!(clipped_text.color_fg, no_style_text.color_fg);
            assert_eq!(clipped_text.color_bg, no_style_text.color_bg);
        }

        // Test case 8: Out of bounds (start beyond text length).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(20),
                width: None,
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 9: Out of bounds with width (start beyond text).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(15),
                width: Some(width(5)),
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty());
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 10: Width exceeds remaining text.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(8),
                width: Some(width(10)), // Only 3 chars available ("rld")
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "rld"); // Clamps to available chars
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 10.1: Width exactly matches remaining text.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(8),
                width: Some(width(3)), // Exactly "rld"
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "rld");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 11: Single character range.
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(6),
                width: Some(width(1)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "W");
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 12: Unicode characters.
        {
            let unicode_text = CliTextInline {
                text: "你好世界".into(), // "Hello World" in Chinese
                attribs: tui_style.attribs,
                color_fg: tui_style.color_fg,
                color_bg: tui_style.color_bg,
            };
            let opt = CliTextConvertOptions {
                start: ColIndex::new(1),
                width: Some(width(2)),
            };
            let clipped_text = unicode_text.clip(opt);
            assert_eq!(clipped_text.text, "好世");
            assert_eq!(clipped_text.attribs, unicode_text.attribs);
            assert_eq!(clipped_text.color_fg, unicode_text.color_fg);
            assert_eq!(clipped_text.color_bg, unicode_text.color_bg);
        }

        // Test case 13: Width zero (empty).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(0)),
            };
            let clipped_text = styled_text.clip(opt);
            assert!(clipped_text.text.is_empty()); // start=0, width=0 -> empty
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 14: Width one (single char).
        {
            let opt = CliTextConvertOptions {
                start: ColIndex::new(0),
                width: Some(width(1)),
            };
            let clipped_text = styled_text.clip(opt);
            assert_eq!(clipped_text.text, "H"); // start=0, width=1 -> "H"
            assert_eq!(clipped_text.attribs, styled_text.attribs);
            assert_eq!(clipped_text.color_fg, styled_text.color_fg);
            assert_eq!(clipped_text.color_bg, styled_text.color_bg);
        }

        // Test case 15: Full text clipping with None width uses display_width().
        // This is the key test demonstrating the fix: when width is None, it uses the
        // actual display width from GCStringOwned instead of a hardcoded 10_000.
        // The existing test case 12 (Unicode characters with CJK wide chars) validates
        // that display-width-aware handling works correctly for wide characters.
        {
            let full_text = styled_text.clip(CliTextConvertOptions::default());
            assert_eq!(full_text.text, "Hello World");
            assert_eq!(full_text.attribs, styled_text.attribs);
            assert_eq!(full_text.color_fg, styled_text.color_fg);
            assert_eq!(full_text.color_bg, styled_text.color_bg);
        }
    }
}

#[cfg(test)]
mod bench_tests {
    extern crate test;
    use super::*;
    use test::Bencher;

    // Benchmark data setup.
    fn simple_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World!".into(),
            attribs: TuiStyleAttribs::default(),
            color_fg: None,
            color_bg: None,
        }
    }

    fn single_style_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World!".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: None,
            color_bg: None,
        }
    }

    fn multiple_styles_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World!".into(),
            attribs: TuiStyleAttribs::from(Bold) + Italic + Underline,
            color_fg: None,
            color_bg: None,
        }
    }

    fn colored_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World!".into(),
            attribs: TuiStyleAttribs::default(),
            color_fg: Some(TuiColor::Ansi(196.into())),
            color_bg: Some(TuiColor::Ansi(236.into())),
        }
    }

    fn rgb_colored_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World!".into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Rgb((255, 0, 0).into())),
            color_bg: Some(TuiColor::Rgb((0, 0, 255).into())),
        }
    }

    fn complex_styled_text() -> CliTextInline {
        CliTextInline {
            text: "Hello, World! This is a longer text with more content.".into(),
            attribs: TuiStyleAttribs::from(Bold) + Italic + Underline,
            color_fg: Some(TuiColor::Rgb((255, 128, 0).into())),
            color_bg: Some(TuiColor::Ansi(236.into())),
        }
    }

    fn long_text() -> CliTextInline {
        CliTextInline {
            text: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
                   Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
                .into(),
            attribs: TuiStyleAttribs::from(Bold),
            color_fg: Some(TuiColor::Ansi(34.into())),
            color_bg: None,
        }
    }

    // Display benchmarks.
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

    // Benchmark creating styled text and displaying.
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

    // Benchmark multiple ASText in sequence (simulating real usage).
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
