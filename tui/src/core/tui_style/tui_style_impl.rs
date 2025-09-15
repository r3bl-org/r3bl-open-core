// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use core::fmt::Debug;
use std::{fmt::{Display, Formatter},
          ops::{Add, AddAssign, Deref}};

use super::{TuiColor, TuiStyleAttribs, tui_style_attrib};
use crate::{ChUnit, InlineVecStr, TinyInlineString, ch, join, join_fmt, ok,
            tiny_inline_string};

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
/// use r3bl_tui::{TuiStyle, TuiStyleAttribs, TuiColor, TuiStylesheet, RgbValue, tui_style_attrib, TuiStyleId};
///
/// // Turquoise: TuiColor::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:      TuiColor::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:      TuiColor::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue:TuiColor::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = TuiStylesheet::new();
///
/// let _ = stylesheet.add_styles(smallvec::smallvec![
///     TuiStyle {
///         id: Some(TuiStyleId(0)),
///         attribs: TuiStyleAttribs {
///             bold: Some(tui_style_attrib::Bold),
///             dim: Some(tui_style_attrib::Dim),
///             ..Default::default()
///         },
///         color_fg: Some(TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })),
///         .. Default::default()
///     },
///     TuiStyle {
///         id: Some(TuiStyleId(1)),
///         attribs: TuiStyleAttribs {
///             bold: Some(tui_style_attrib::Bold),
///             dim: Some(tui_style_attrib::Dim),
///             ..Default::default()
///         },
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
    pub id: Option<TuiStyleId>,
    pub attribs: TuiStyleAttribs,
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

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TuiStyleId(pub u8);
pub fn tui_style_id(arg_val: impl Into<u8>) -> Option<TuiStyleId> {
    Some(TuiStyleId(arg_val.into()))
}

mod id_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl TuiStyleId {
        #[must_use]
        pub fn eq(maybe_id: Option<TuiStyleId>, other: u8) -> bool {
            match maybe_id {
                None => false,
                Some(id) => id.0 == other,
            }
        }

        #[must_use]
        pub fn fmt_id(maybe_id: Option<TuiStyleId>) -> TinyInlineString {
            use std::fmt::Write;
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

    impl From<u8> for TuiStyleId {
        fn from(id: u8) -> Self { TuiStyleId(id) }
    }

    impl Deref for TuiStyleId {
        type Target = u8;
        fn deref(&self) -> &Self::Target { &self.0 }
    }
}

mod addition {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Add for TuiStyle {
        type Output = Self;
        fn add(self, other: Self) -> Self { add_styles(self, other) }
    }

    pub fn add_styles(lhs: TuiStyle, rhs: TuiStyle) -> TuiStyle {
        // other (if set) overrides new_style.
        fn apply_style_flag(new_style: &mut TuiStyle, other: &TuiStyle) {
            // Apply color attributes.
            if other.color_fg.is_some() {
                new_style.color_fg = other.color_fg;
            }
            if other.color_bg.is_some() {
                new_style.color_bg = other.color_bg;
            }
            // Apply style attributes.
            if other.attribs.bold.is_some() {
                new_style.attribs.bold = other.attribs.bold;
            }
            if other.attribs.italic.is_some() {
                new_style.attribs.italic = other.attribs.italic;
            }
            if other.attribs.dim.is_some() {
                new_style.attribs.dim = other.attribs.dim;
            }
            if other.attribs.underline.is_some() {
                new_style.attribs.underline = other.attribs.underline;
            }
            if other.attribs.blink.is_some() {
                new_style.attribs.blink = other.attribs.blink;
            }
            if other.attribs.reverse.is_some() {
                new_style.attribs.reverse = other.attribs.reverse;
            }
            if other.attribs.hidden.is_some() {
                new_style.attribs.hidden = other.attribs.hidden;
            }
            if other.attribs.strikethrough.is_some() {
                new_style.attribs.strikethrough = other.attribs.strikethrough;
            }
            // Apply padding (not part of attribs)
            if other.padding.is_some() {
                new_style.padding = other.padding;
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl Debug for TuiStyle {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let id_str = TuiStyleId::fmt_id(self.id);

            // This accumulator is needed to be able to add `+` delimiter between
            // attributes.
            let mut acc_attrs = InlineVecStr::new();

            if self.computed.is_some() {
                acc_attrs.push("computed");
            } else {
                acc_attrs.push(&id_str);
            }

            if self.attribs.bold.is_some() {
                acc_attrs.push("bold");
            }

            if self.attribs.italic.is_some() {
                acc_attrs.push("italic");
            }

            if self.attribs.dim.is_some() {
                acc_attrs.push("dim");
            }

            if self.attribs.underline.is_some() {
                acc_attrs.push("underline");
            }

            if self.attribs.blink.is_some() {
                acc_attrs.push("blink");
            }

            if self.attribs.reverse.is_some() {
                acc_attrs.push("reverse");
            }

            if self.attribs.hidden.is_some() {
                acc_attrs.push("hidden");
            }

            if self.attribs.strikethrough.is_some() {
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

            if self.attribs.bold.is_some() {
                acc.push("bld");
            }

            if self.attribs.italic.is_some() {
                acc.push("itl");
            }

            if self.attribs.dim.is_some() {
                acc.push("dim");
            }

            if self.attribs.underline.is_some() {
                acc.push("und");
            }

            if self.attribs.blink.is_some() {
                acc.push("blk");
            }

            if self.attribs.reverse.is_some() {
                acc.push("rev");
            }

            if self.attribs.hidden.is_some() {
                acc.push("hid");
            }

            if self.attribs.strikethrough.is_some() {
                acc.push("str");
            }

            if self.color_fg.is_some() {
                acc.push("fg");
            }

            if self.color_bg.is_some() {
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
    #[allow(clippy::wildcard_imports)]
    use super::*;

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
        let attribs = TuiStyleAttribs {
            bold: Some(tui_style_attrib::Bold),
            dim: Some(tui_style_attrib::Dim),
            underline: Some(tui_style_attrib::Underline),
            reverse: Some(tui_style_attrib::Reverse),
            hidden: Some(tui_style_attrib::Hidden),
            strikethrough: Some(tui_style_attrib::Strikethrough),
            ..Default::default()
        };

        let style = TuiStyle {
            id: Some(TuiStyleId(1)),
            attribs,
            color_fg: tui_color!(red).into(),
            color_bg: tui_color!(0, 0, 0).into(),
            padding: Some(ch(10)),
            ..TuiStyle::default()
        };

        assert!(style.computed.is_none());
        assert!(TuiStyleId::eq(style.id, 1));
        assert!(style.attribs.bold.is_some());
        assert!(style.attribs.dim.is_some());
        assert!(style.attribs.underline.is_some());
        assert!(style.attribs.reverse.is_some());
        assert!(style.attribs.hidden.is_some());
        assert!(style.attribs.strikethrough.is_some());
        assert_eq2!(style.color_fg, tui_color!(red).into());
        assert_eq2!(style.color_bg, tui_color!(0, 0, 0).into());
        assert_eq2!(style.padding, Some(ch(10)));
    }

    #[test]
    fn test_style() {
        let attribs = TuiStyleAttribs {
            bold: Some(tui_style_attrib::Bold),
            dim: Some(tui_style_attrib::Dim),
            italic: Some(tui_style_attrib::Italic),
            ..Default::default()
        };

        let style = TuiStyle {
            id: Some(TuiStyleId(1)),
            attribs,
            color_fg: tui_color!(0, 0, 0).into(),
            color_bg: tui_color!(0, 0, 0).into(),
            ..TuiStyle::default()
        };

        dbg!(&style);

        assert!(style.computed.is_none());
        assert!(TuiStyleId::eq(style.id, 1));
        assert!(style.attribs.bold.is_some());
        assert!(style.attribs.dim.is_some());
        assert!(style.attribs.italic.is_some());
        assert!(style.attribs.underline.is_none());
        assert!(style.attribs.strikethrough.is_none());
        assert!(style.attribs.reverse.is_none());
    }
}
