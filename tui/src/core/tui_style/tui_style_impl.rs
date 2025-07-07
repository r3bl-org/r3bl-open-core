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
use std::{fmt::{Display, Formatter},
          ops::{Add, AddAssign, Deref}};

use super::TuiColor;
use crate::{ch, join, join_fmt, ok, tiny_inline_string, ChUnit, InlineVecStr,
            TinyInlineString};

/// Please use [`crate::new_style`!] declarative macro to generate code for this struct.
///
/// The following is handled by the [Default] implementation of `TuiStyle`:
/// - For the macro, if `id` isn't supplied, then [None] is used. This represents the
///   "style does not have an assigned id" case.
/// - Computed styles don't have an id and are set to [None] as well.
///
/// Here's an example.
///
/// ```
/// use r3bl_tui::{TuiStyle, TuiColor, TuiStylesheet, RgbValue, tui_style_attrib};
///
/// // Turquoise: TuiColor::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:      TuiColor::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:      TuiColor::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue:TuiColor::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = TuiStylesheet::new();
///
/// let _ = stylesheet.add_styles(smallvec::smallvec![
///     TuiStyle {
///         id: Some(tui_style_attrib::Id(0)),
///         bold: Some(tui_style_attrib::Bold),
///         dim: Some(tui_style_attrib::Dim),
///         color_fg: Some(TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })),
///         .. Default::default()
///     },
///     TuiStyle {
///         id: Some(tui_style_attrib::Id(1)),
///         bold: Some(tui_style_attrib::Bold),
///         dim: Some(tui_style_attrib::Dim),
///         color_fg: Some(TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })),
///         .. Default::default()
///     },
/// ]);
/// ```
///
/// Here are the [crossterm docs on
/// attributes](https://docs.rs/crossterm/0.25.0/crossterm/style/enum.Attribute.html)
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct TuiStyle {
    // XMARK: Use of newtype pattern `Option<T>` instead of `bool`
    pub id: Option<tui_style_attrib::Id>,
    pub bold: Option<tui_style_attrib::Bold>,
    pub italic: Option<tui_style_attrib::Italic>,
    pub dim: Option<tui_style_attrib::Dim>,
    pub underline: Option<tui_style_attrib::Underline>,
    pub reverse: Option<tui_style_attrib::Reverse>,
    pub hidden: Option<tui_style_attrib::Hidden>,
    pub strikethrough: Option<tui_style_attrib::Strikethrough>,
    pub computed: Option<tui_style_attrib::Computed>,
    pub color_fg: Option<TuiColor>,
    pub color_bg: Option<TuiColor>,
    /// The semantics of this are the same as CSS. The padding is space that is taken up
    /// inside a `FlexBox`. This does not affect the size or position of a `FlexBox`, it
    /// only applies to the contents inside that `FlexBox`.
    /// - [`FlexBox` docs](https://docs.rs/r3bl_tui/latest/r3bl_tui/tui/layout/flex_box/struct.FlexBox.html).
    pub padding: Option<ChUnit>,
    pub lolcat: Option<tui_style_attrib::Lolcat>,
}

pub mod tui_style_attrib {
    use super::{Debug, Deref, TinyInlineString};

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Id(pub u8);

    impl Id {
        #[must_use]
        pub fn eq(maybe_id: Option<Id>, other: u8) -> bool {
            match maybe_id {
                None => false,
                Some(id) => id.0 == other,
            }
        }

        #[must_use]
        pub fn fmt_id(maybe_id: Option<Id>) -> TinyInlineString {
            use std::fmt::Write as _;
            let mut acc = TinyInlineString::new();
            match maybe_id {
                None => {
                    // We don't care about the result of this operation.
                    write!(acc, "id: N/A").ok();
                }
                Some(id) => {
                    // We don't care about the result of this operation.
                    write!(acc, "id: {}", id.0).ok();
                }
            }
            acc
        }
    }

    pub fn id(arg_val: impl Into<u8>) -> Option<Id> { Some(Id(arg_val.into())) }

    impl From<u8> for Id {
        fn from(id: u8) -> Self { Id(id) }
    }

    impl Deref for Id {
        type Target = u8;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Bold;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Italic;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Dim;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Underline;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Reverse;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Hidden;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Strikethrough;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Computed;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Lolcat;
}

mod addition {
    use super::{ch, tui_style_attrib, Add, AddAssign, ChUnit, TuiStyle};

    impl Add for TuiStyle {
        type Output = Self;
        fn add(self, other: Self) -> Self { add_styles(self, other) }
    }

    pub fn add_styles(lhs: TuiStyle, rhs: TuiStyle) -> TuiStyle {
        // other (if set) overrides new_style.
        fn apply_style_flag(new_style: &mut TuiStyle, other: &TuiStyle) {
            if other.color_fg.is_some() {
                new_style.color_fg = other.color_fg;
            }
            if other.color_bg.is_some() {
                new_style.color_bg = other.color_bg;
            }
            if other.bold.is_some() {
                new_style.bold = other.bold;
            }
            if other.italic.is_some() {
                new_style.italic = other.italic;
            }
            if other.dim.is_some() {
                new_style.dim = other.dim;
            }
            if other.underline.is_some() {
                new_style.underline = other.underline;
            }
            if other.padding.is_some() {
                new_style.padding = other.padding;
            }
            if other.reverse.is_some() {
                new_style.reverse = other.reverse;
            }
            if other.hidden.is_some() {
                new_style.hidden = other.hidden;
            }
            if other.strikethrough.is_some() {
                new_style.strikethrough = other.strikethrough;
            }
        }

        // Computed style has no id.
        let mut new_style: TuiStyle = TuiStyle {
            id: None,
            computed: Some(tui_style_attrib::Computed),
            ..TuiStyle::default()
        };

        apply_style_flag(&mut new_style, &lhs);
        apply_style_flag(&mut new_style, &rhs);

        // Aggregate paddings.
        let aggregate_padding: ChUnit =
            lhs.padding.unwrap_or_else(|| ch(0)) + rhs.padding.unwrap_or_else(|| ch(0));
        if *aggregate_padding > 0 {
            new_style.padding = aggregate_padding.into();
        } else {
            new_style.padding = None;
        }

        new_style
    }

    impl AddAssign<TuiStyle> for TuiStyle {
        fn add_assign(&mut self, rhs: TuiStyle) {
            let sum = add_styles(*self, rhs);
            *self = sum;
        }
    }

    impl AddAssign<&TuiStyle> for TuiStyle {
        fn add_assign(&mut self, rhs: &TuiStyle) {
            let sum = add_styles(*self, *rhs);
            *self = sum;
        }
    }

    impl AddAssign<&Option<TuiStyle>> for TuiStyle {
        fn add_assign(&mut self, rhs: &Option<TuiStyle>) {
            if let Some(rhs) = rhs {
                *self += rhs;
            }
        }
    }
}

mod style_helper {
    use super::{ch, join, join_fmt, ok, tiny_inline_string, tui_style_attrib, Debug,
                Display, Formatter, InlineVecStr, TinyInlineString, TuiStyle};

    impl Debug for TuiStyle {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let id_str = tui_style_attrib::Id::fmt_id(self.id);

            // This accumulator is needed to be able to add `+` delimiter between
            // attributes.
            let mut acc_attrs = InlineVecStr::new();

            if self.computed.is_some() {
                acc_attrs.push("computed");
            } else {
                acc_attrs.push(&id_str);
            }

            if self.bold.is_some() {
                acc_attrs.push("bold");
            }

            if self.italic.is_some() {
                acc_attrs.push("italic");
            }

            if self.dim.is_some() {
                acc_attrs.push("dim");
            }

            if self.underline.is_some() {
                acc_attrs.push("underline");
            }

            if self.reverse.is_some() {
                acc_attrs.push("reverse");
            }

            if self.hidden.is_some() {
                acc_attrs.push("hidden");
            }

            if self.strikethrough.is_some() {
                acc_attrs.push("strikethrough");
            }

            let attrs_str = join!(
                from: acc_attrs,
                each: it,
                delim: " + ",
                format: "{it}",
            );

            write!(
                f,
                "Style {{ {attrs_str} | fg: {fg:?} | bg: {bg:?} | padding: {p:?} }}",
                fg = self.color_fg,
                bg = self.color_bg,
                p = *self.padding.unwrap_or_else(|| ch(0))
            )
        }
    }

    impl Display for TuiStyle {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let pad_str = match self.padding {
                Some(padding) => tiny_inline_string!("pad:{padding:?}"),
                None => TinyInlineString::new(),
            };

            // Need `acc` since we don't know how many attributes are set.
            let mut acc = InlineVecStr::new();

            if self.bold.is_some() {
                acc.push("bld");
            }

            if self.italic.is_some() {
                acc.push("itl");
            }

            if self.dim.is_some() {
                acc.push("dim");
            }

            if self.underline.is_some() {
                acc.push("und");
            }

            if self.reverse.is_some() {
                acc.push("rev");
            }

            if self.hidden.is_some() {
                acc.push("hid");
            }

            if self.strikethrough.is_some() {
                acc.push("str");
            }

            if self.color_fg.is_some() {
                acc.push("fg");
            }

            if self.color_fg.is_some() {
                acc.push("bg");
            }

            acc.push(pad_str.as_str());

            join_fmt!(
                fmt: f,
                from: acc,
                each: it,
                delim: "‐",
                format: "{it}",
            );

            ok!()
        }
    }
}

mod style_impl {
    use super::TuiStyle;

    impl TuiStyle {
        pub fn remove_bg_color(&mut self) { self.color_bg = None; }
    }
}

#[cfg(test)]
mod test_style {
    use super::*;
    use crate::{assert_eq2, tui_color};

    #[test]
    fn test_all_fields_in_style() {
        let style = TuiStyle {
            id: Some(tui_style_attrib::Id(1)),
            bold: Some(tui_style_attrib::Bold),
            dim: Some(tui_style_attrib::Dim),
            underline: Some(tui_style_attrib::Underline),
            reverse: Some(tui_style_attrib::Reverse),
            hidden: Some(tui_style_attrib::Hidden),
            strikethrough: Some(tui_style_attrib::Strikethrough),
            color_fg: tui_color!(red).into(),
            color_bg: tui_color!(0, 0, 0).into(),
            padding: Some(ch(10)),
            ..TuiStyle::default()
        };

        assert!(style.computed.is_none());
        assert!(tui_style_attrib::Id::eq(style.id, 1));
        assert!(style.bold.is_some());
        assert!(style.dim.is_some());
        assert!(style.underline.is_some());
        assert!(style.reverse.is_some());
        assert!(style.hidden.is_some());
        assert!(style.strikethrough.is_some());
        assert_eq2!(style.color_fg, tui_color!(red).into());
        assert_eq2!(style.color_bg, tui_color!(0, 0, 0).into());
        assert_eq2!(style.padding, Some(ch(10)));
    }

    #[test]
    fn test_style() {
        let style = TuiStyle {
            id: Some(tui_style_attrib::Id(1)),
            color_fg: tui_color!(0, 0, 0).into(),
            color_bg: tui_color!(0, 0, 0).into(),
            bold: Some(tui_style_attrib::Bold),
            dim: Some(tui_style_attrib::Dim),
            italic: Some(tui_style_attrib::Italic),
            ..TuiStyle::default()
        };

        dbg!(&style);

        assert!(style.computed.is_none());
        assert!(tui_style_attrib::Id::eq(style.id, 1));
        assert!(style.bold.is_some());
        assert!(style.dim.is_some());
        assert!(style.italic.is_some());
        assert!(style.underline.is_none());
        assert!(style.strikethrough.is_none());
        assert!(style.reverse.is_none());
    }
}
