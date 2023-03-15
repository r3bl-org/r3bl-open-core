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

use get_size::GetSize;
use serde::{Deserialize, Serialize};

use crate::*;

/// Please use [style!](crate::style) proc macro to generate code for this struct. For the macro, if
/// `id` isn't supplied, then [u8::MAX](u8::MAX) is used. This represents the "style does not have
/// an assigned id" case. Computed styles don't have an id and are set to [u8::MAX](u8::MAX) as
/// well.
///
/// Here's an example.
///
/// ```ignore
/// use r3bl_rs_utils_core::Stylesheet;
/// use r3bl_rs_utils_macro::style;
///
/// // Turquoise:  TuiColor::Rgb { r: 51, g: 255, b: 255 }
/// // Pink:       TuiColor::Rgb { r: 252, g: 157, b: 248 }
/// // Blue:       TuiColor::Rgb { r: 55, g: 55, b: 248 }
/// // Faded blue: TuiColor::Rgb { r: 85, g: 85, b: 255 }
/// let mut stylesheet = Stylesheet::new();
///
/// stylesheet.add_styles(vec![
///   style! {
///     id: style1
///     attrib: [dim, bold]
///     padding: 1
///     color_bg: TuiColor::Rgb { r: 55, g: 55, b: 248 }
///   },
///   style! {
///     id: style2
///     padding: 1
///     color_bg: TuiColor::Rgb { r: 85, g: 85, b: 255 }
///   }
/// ]);
/// ```
///
/// Here are the [crossterm docs on
/// attributes](https://docs.rs/crossterm/0.25.0/crossterm/style/enum.Attribute.html)
#[derive(Copy, Default, Clone, PartialEq, Eq, Serialize, Deserialize, Hash, GetSize)]
pub struct Style {
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
    /// The semantics of this are the same as CSS. The padding is space that is taken up inside a
    /// `FlexBox`. This does not affect the size or position of a `FlexBox`, it only applies to the
    /// contents inside of that `FlexBox`.
    ///
    /// [`FlexBox` docs](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/tui/layout/flex_box/struct.FlexBox.html).
    pub padding: Option<ChUnit>,
    pub lolcat: bool,
}

mod addition {
    use super::*;

    impl Add for Style {
        type Output = Self;
        fn add(self, other: Self) -> Self { add_styles(self, other) }
    }

    pub fn add_styles(lhs: Style, rhs: Style) -> Style {
        // Computed style has no id.
        let mut new_style: Style = Style {
            id: u8::MAX,
            computed: true,
            ..Style::default()
        };

        apply_style_flag(&mut new_style, &lhs);
        apply_style_flag(&mut new_style, &rhs);

        // other (if set) overrides new_style.
        fn apply_style_flag(new_style: &mut Style, other: &Style) {
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
            lhs.padding.unwrap_or_else(|| ch!(0)) + rhs.padding.unwrap_or_else(|| ch!(0));
        if *aggregate_padding > 0 {
            new_style.padding = aggregate_padding.into();
        } else {
            new_style.padding = None;
        }

        new_style
    }

    impl AddAssign<Style> for Style {
        fn add_assign(&mut self, rhs: Style) {
            let sum = add_styles(*self, rhs);
            *self = sum;
        }
    }

    impl AddAssign<&Style> for Style {
        fn add_assign(&mut self, rhs: &Style) {
            let sum = add_styles(*self, *rhs);
            *self = sum;
        }
    }

    impl AddAssign<&Option<Style>> for Style {
        fn add_assign(&mut self, rhs: &Option<Style>) {
            if let Some(rhs) = rhs {
                *self += rhs;
            }
        }
    }
}

mod style_helpers {
    use super::*;

    impl Display for Style {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            let msg = format!("{self:?}");
            f.write_str(&msg)
        }
    }

    impl Style {
        pub fn pretty_print(&self) -> String {
            let mut msg_vec: Vec<String> = Default::default();

            if self.bold {
                msg_vec.push("bld".to_string())
            }

            if self.italic {
                msg_vec.push("itl".to_string())
            }

            if self.dim {
                msg_vec.push("dim".to_string())
            }

            if self.underline {
                msg_vec.push("und".to_string())
            }

            if self.reverse {
                msg_vec.push("rev".to_string())
            }

            if self.hidden {
                msg_vec.push("hid".to_string())
            }

            if self.strikethrough {
                msg_vec.push("str".to_string())
            }

            if self.color_fg.is_some() {
                msg_vec.push("fg".to_string())
            }

            if self.color_fg.is_some() {
                msg_vec.push("bg".to_string())
            }

            if let Some(padding) = self.padding {
                msg_vec.push(format!("pad:{padding:?}"))
            }

            msg_vec.join("‐")
        }
    }

    impl Debug for Style {
        fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
            let mut msg_vec: Vec<String> = vec![];

            if self.computed {
                msg_vec.push("computed".to_string())
            } else if self.id == u8::MAX {
                msg_vec.push("id: N/A".to_string())
            } else {
                msg_vec.push(self.id.to_string());
            }

            if self.bold {
                msg_vec.push("bold".to_string())
            }

            if self.italic {
                msg_vec.push("italic".to_string())
            }

            if self.dim {
                msg_vec.push("dim".to_string())
            }

            if self.underline {
                msg_vec.push("underline".to_string())
            }

            if self.reverse {
                msg_vec.push("reverse".to_string())
            }

            if self.hidden {
                msg_vec.push("hidden".to_string())
            }

            if self.strikethrough {
                msg_vec.push("strikethrough".to_string())
            }

            write!(
                f,
                "Style {{ {} | fg: {:?} | bg: {:?} | padding: {:?} }}",
                msg_vec.join(" + "),
                self.color_fg,
                self.color_bg,
                *self.padding.unwrap_or_else(|| ch!(0))
            )
        }
    }
}

mod style_impl {
    use super::*;

    impl Style {
        pub fn remove_bg_color(&mut self) { self.color_bg = None; }
    }
}

mod convert_syntect_highlighting_style {
    use super::*;

    type SyntectStyle = syntect::highlighting::Style;
    type SyntectFontStyle = syntect::highlighting::FontStyle;
    type SyntectColor = syntect::highlighting::Color;

    impl From<SyntectStyle> for Style {
        fn from(st_style: SyntectStyle) -> Self {
            Style {
                color_fg: Some(st_style.foreground.into()),
                color_bg: Some(st_style.background.into()),
                bold: st_style.font_style.contains(SyntectFontStyle::BOLD),
                italic: st_style.font_style.contains(SyntectFontStyle::ITALIC),
                underline: st_style.font_style.contains(SyntectFontStyle::UNDERLINE),
                ..Default::default()
            }
        }
    }

    impl From<SyntectColor> for TuiColor {
        fn from(st_color: SyntectColor) -> Self {
            TuiColor::Rgb(RgbValue::from_u8(st_color.r, st_color.g, st_color.b))
        }
    }
}

#[cfg(test)]
mod test_style {
    use super::*;

    #[test]
    fn test_all_fields_in_style() {
        let style = Style {
            id: 1,
            bold: true,
            dim: true,
            underline: true,
            reverse: true,
            hidden: true,
            strikethrough: true,
            color_fg: color!(@red).into(),
            color_bg: color!(0, 0, 0).into(),
            padding: Some(ch!(10)),
            ..Style::default()
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
        assert_eq2!(style.padding, Some(ch!(10)));
    }

    #[test]
    fn test_style() {
        let style = Style {
            id: 1,
            color_fg: color!(0, 0, 0).into(),
            color_bg: color!(0, 0, 0).into(),
            bold: true,
            dim: true,
            italic: true,
            ..Style::default()
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
