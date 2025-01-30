/*
 *   Copyright (c) 2022 R3BL LLC
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
          ops::{Add, AddAssign}};

use sizing::VecStyles;
use smallvec::SmallVec;
use strum::EnumCount;

use super::TuiColor;
use crate::{ChUnit, VecArrayStr, ch, convert_tui_color_into_r3bl_ansi_color};

/// Please use [tui_style!](crate::tui_style) proc macro to generate code for this struct.
///
/// For the macro, if `id` isn't supplied, then [u8::MAX](u8::MAX) is used. This
/// represents the "style does not have an assigned id" case. Computed styles don't have
/// an id and are set to [u8::MAX](u8::MAX) as well.
///
/// Here's an example.
///
/// ```rust
/// use r3bl_core::{TuiStyle, TuiColor, TuiStylesheet, RgbValue};
///
/// // Turquoise:  TuiColor::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:       TuiColor::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:       TuiColor::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue: TuiColor::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = TuiStylesheet::new();
///
/// let _ = stylesheet.add_styles(smallvec::smallvec![
///     TuiStyle {
///         id: 1,
///         bold: true,
///         dim: true,
///         color_fg: Some(TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })),
///         .. Default::default()
///     },
///     TuiStyle {
///         id: 1,
///         bold: true,
///         dim: true,
///         color_fg: Some(TuiColor::Rgb (RgbValue{ red: 55, green: 55, blue: 248 })),
///         .. Default::default()
///     },
/// ]);
/// ```
///
/// Here are the [crossterm docs on
/// attributes](https://docs.rs/crossterm/0.25.0/crossterm/style/enum.Attribute.html)
#[derive(Copy, Default, Clone, PartialEq, Eq, Hash, size_of::SizeOf)]
pub struct TuiStyle {
    pub id: u8,
    pub bold: bool,
    pub italic: bool,
    pub dim: bool,
    pub underline: bool,
    pub reverse: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    pub computed: bool,
    pub color_fg: Option<TuiColor>,
    pub color_bg: Option<TuiColor>,
    /// The semantics of this are the same as CSS. The padding is space that is taken up
    /// inside a `FlexBox`. This does not affect the size or position of a `FlexBox`, it
    /// only applies to the contents inside of that `FlexBox`.
    ///
    /// [`FlexBox`
    /// docs](https://docs.rs/r3bl_tui/latest/r3bl_tui/tui/layout/flex_box/struct.FlexBox.html).
    pub padding: Option<ChUnit>,
    pub lolcat: bool,
}

mod addition {
    use super::*;

    impl Add for TuiStyle {
        type Output = Self;
        fn add(self, other: Self) -> Self { add_styles(self, other) }
    }

    pub fn add_styles(lhs: TuiStyle, rhs: TuiStyle) -> TuiStyle {
        // Computed style has no id.
        let mut new_style: TuiStyle = TuiStyle {
            id: u8::MAX,
            computed: true,
            ..TuiStyle::default()
        };

        apply_style_flag(&mut new_style, &lhs);
        apply_style_flag(&mut new_style, &rhs);

        // other (if set) overrides new_style.
        fn apply_style_flag(new_style: &mut TuiStyle, other: &TuiStyle) {
            if other.color_fg.is_some() {
                new_style.color_fg = other.color_fg;
            }
            if other.color_bg.is_some() {
                new_style.color_bg = other.color_bg;
            }
            if other.bold {
                new_style.bold = other.bold;
            }
            if other.italic {
                new_style.italic = other.italic;
            }
            if other.dim {
                new_style.dim = other.dim;
            }
            if other.underline {
                new_style.underline = other.underline;
            }
            if other.padding.is_some() {
                new_style.padding = other.padding;
            }
            if other.reverse {
                new_style.reverse = other.reverse;
            }
            if other.hidden {
                new_style.hidden = other.hidden;
            }
            if other.strikethrough {
                new_style.strikethrough = other.strikethrough;
            }
        }

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

mod style_helpers {
    use super::*;
    use crate::{CharStorage, char_storage, join, join_fmt, ok};

    impl Debug for TuiStyle {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            let id = self.id.to_string();

            // This accumulator is needed in order to be able to add `+` delimiter between
            // attributes.
            let mut acc_attrs = VecArrayStr::new();

            if self.computed {
                acc_attrs.push("computed");
            } else if self.id == u8::MAX {
                acc_attrs.push("id: N/A");
            } else {
                acc_attrs.push(id.as_str());
            }

            if self.bold {
                acc_attrs.push("bold");
            }

            if self.italic {
                acc_attrs.push("italic");
            }

            if self.dim {
                acc_attrs.push("dim");
            }

            if self.underline {
                acc_attrs.push("underline");
            }

            if self.reverse {
                acc_attrs.push("reverse");
            }

            if self.hidden {
                acc_attrs.push("hidden");
            }

            if self.strikethrough {
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
                Some(padding) => char_storage!("pad:{padding:?}"),
                None => CharStorage::new(),
            };

            // Need `acc` since we don't know how many attributes are set.
            let mut acc = VecArrayStr::new();

            if self.bold {
                acc.push("bld");
            }

            if self.italic {
                acc.push("itl");
            }

            if self.dim {
                acc.push("dim");
            }

            if self.underline {
                acc.push("und");
            }

            if self.reverse {
                acc.push("rev");
            }

            if self.hidden {
                acc.push("hid");
            }

            if self.strikethrough {
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
    use super::*;

    impl TuiStyle {
        pub fn remove_bg_color(&mut self) { self.color_bg = None; }
    }
}

#[cfg(test)]
mod test_style {
    use super::*;
    use crate::{ANSIBasicColor, assert_eq2, color};

    #[test]
    fn test_all_fields_in_style() {
        let style = TuiStyle {
            id: 1,
            bold: true,
            dim: true,
            underline: true,
            reverse: true,
            hidden: true,
            strikethrough: true,
            color_fg: color!(@red).into(),
            color_bg: color!(0, 0, 0).into(),
            padding: Some(ch(10)),
            ..TuiStyle::default()
        };

        assert!(!style.computed);
        assert_eq2!(style.id, 1);
        assert!(style.bold);
        assert!(style.dim);
        assert!(style.underline);
        assert!(style.reverse);
        assert!(style.hidden);
        assert!(style.strikethrough);
        assert_eq2!(style.color_fg, color!(@red).into());
        assert_eq2!(style.color_bg, color!(0, 0, 0).into());
        assert_eq2!(style.padding, Some(ch(10)));
    }

    #[test]
    fn test_style() {
        let style = TuiStyle {
            id: 1,
            color_fg: color!(0, 0, 0).into(),
            color_bg: color!(0, 0, 0).into(),
            bold: true,
            dim: true,
            italic: true,
            ..TuiStyle::default()
        };

        dbg!(&style);

        assert!(style.bold);
        assert!(style.dim);
        assert!(style.italic);
        assert!(!style.underline);
        assert!(!style.strikethrough);
        assert!(!style.reverse);
    }
}

mod sizing {
    use super::*;

    /// Attributes are: color_fg, color_bg, bold, dim, italic, underline, reverse, hidden,
    /// etc. which are in [r3bl_ansi_color::Style].
    pub type VecStyles = SmallVec<[r3bl_ansi_color::Style; MAX_STYLE_ATTRIB_SIZE]>;
    const MAX_STYLE_ATTRIB_SIZE: usize = r3bl_ansi_color::Style::COUNT;
}

pub mod convert_to_ansi_color_styles {
    use super::*;

    pub fn from_tui_style(tui_style: TuiStyle) -> VecStyles {
        let mut acc = VecStyles::new();

        if let Some(color_fg) = tui_style.color_fg {
            acc.push(r3bl_ansi_color::Style::Foreground(
                convert_tui_color_into_r3bl_ansi_color(color_fg),
            ));
        }

        if let Some(color_bg) = tui_style.color_bg {
            acc.push(r3bl_ansi_color::Style::Background(
                convert_tui_color_into_r3bl_ansi_color(color_bg),
            ));
        }

        if tui_style.bold {
            acc.push(r3bl_ansi_color::Style::Bold);
        }

        if tui_style.dim {
            acc.push(r3bl_ansi_color::Style::Dim);
        }

        if tui_style.italic {
            acc.push(r3bl_ansi_color::Style::Italic);
        }

        if tui_style.underline {
            acc.push(r3bl_ansi_color::Style::Underline);
        }

        if tui_style.reverse {
            acc.push(r3bl_ansi_color::Style::Invert);
        }

        if tui_style.hidden {
            acc.push(r3bl_ansi_color::Style::Hidden);
        }

        if tui_style.strikethrough {
            acc.push(r3bl_ansi_color::Style::Strikethrough);
        }

        acc
    }

    #[cfg(test)]
    mod tests_style {
        use super::*;
        use crate::{ANSIBasicColor, assert_eq2, color};

        #[test]
        fn test_all_fields_in_style() {
            let style = TuiStyle {
                id: 1,
                bold: true,
                dim: true,
                underline: true,
                reverse: true,
                hidden: true,
                strikethrough: true,
                color_fg: color!(@red).into(),
                color_bg: color!(0, 0, 0).into(),
                padding: Some(ch(10)),
                ..TuiStyle::default()
            };

            assert!(!style.computed);
            assert_eq2!(style.id, 1);
            assert!(style.bold);
            assert!(style.dim);
            assert!(style.underline);
            assert!(style.reverse);
            assert!(style.hidden);
            assert!(style.strikethrough);
            assert_eq2!(style.color_fg, color!(@red).into());
            assert_eq2!(style.color_bg, color!(0, 0, 0).into());
            assert_eq2!(style.padding, Some(ch(10)));
        }

        #[test]
        fn test_style() {
            let style = TuiStyle {
                id: 1,
                color_fg: color!(0, 0, 0).into(),
                color_bg: color!(0, 0, 0).into(),
                bold: true,
                dim: true,
                italic: true,
                ..TuiStyle::default()
            };

            dbg!(&style);

            assert!(style.bold);
            assert!(style.dim);
            assert!(style.italic);
            assert!(!style.underline);
            assert!(!style.strikethrough);
            assert!(!style.reverse);
        }

        #[test]
        fn test_add_styles() {
            let style1 = TuiStyle {
                bold: true,
                color_fg: color!(@red).into(),
                ..TuiStyle::default()
            };

            let style2 = TuiStyle {
                italic: true,
                color_bg: color!(0, 0, 0).into(),
                ..TuiStyle::default()
            };

            let combined_style = style1 + style2;

            assert!(combined_style.bold);
            assert!(combined_style.italic);
            assert_eq2!(combined_style.color_fg, color!(@red).into());
            assert_eq2!(combined_style.color_bg, color!(0, 0, 0).into());
        }

        #[test]
        fn test_add_assign_styles() {
            let mut style1 = TuiStyle {
                bold: true,
                color_fg: color!(@red).into(),
                ..TuiStyle::default()
            };

            let style2 = TuiStyle {
                italic: true,
                color_bg: color!(0, 0, 0).into(),
                ..TuiStyle::default()
            };

            style1 += style2;

            assert!(style1.bold);
            assert!(style1.italic);
            assert_eq2!(style1.color_fg, color!(@red).into());
            assert_eq2!(style1.color_bg, color!(0, 0, 0).into());
        }

        #[test]
        fn test_remove_bg_color() {
            let mut style = TuiStyle {
                color_bg: color!(0, 0, 0).into(),
                ..TuiStyle::default()
            };

            style.remove_bg_color();

            assert!(style.color_bg.is_none());
        }
    }
}
