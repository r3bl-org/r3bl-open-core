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

use std::ops::{AddAssign, Index};

use r3bl_rs_utils_core::{ChUnit,
                         ConvertToPlainText,
                         PrettyPrintDebug,
                         TuiStyle,
                         UnicodeString};

/// Macro to make building [TuiStyledText] easy.
///
/// Here's an example.
/// ```rust
/// use r3bl_rs_utils_core::*;
/// use r3bl_tui::*;
///
/// let style = TuiStyle::default();
/// let st = tui_styled_text!(@style: style, @text: "Hello World");
/// ```
#[macro_export]
macro_rules! tui_styled_text {
    (
        @style: $style_arg: expr,
        @text: $text_arg: expr
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) => {
        TuiStyledText::new($style_arg, $text_arg.to_string())
    };
}

/// Use [tui_styled_text!] macro for easier construction.
#[derive(Debug, Clone, Default, size_of::SizeOf)]
pub struct TuiStyledText {
    pub style: TuiStyle,
    pub text: UnicodeString,
}

mod tui_styled_text_impl {
    use super::*;

    impl TuiStyledText {
        pub fn new(style: TuiStyle, text: String) -> Self {
            TuiStyledText {
                style,
                text: UnicodeString::from(text),
            }
        }

        pub fn get_text(&self) -> &UnicodeString { &self.text }

        pub fn get_style(&self) -> &TuiStyle { &self.style }
    }
}

/// Macro to make building [`TuiStyledTexts`] easy.
///
/// Here's an example.
/// ```rust
/// use r3bl_rs_utils_core::*;
/// use r3bl_tui::*;
///
/// let mut st_vec = tui_styled_texts! {
///   tui_styled_text! {
///     @style: TuiStyle::default(),
///     @text: "Hello",
///   },
///   tui_styled_text! {
///     @style: TuiStyle::default(),
///     @text: "World",
///   }
/// };
/// ```
#[macro_export]
macro_rules! tui_styled_texts {
    (
        $($styled_text_arg : expr),*
        $(,)* /* Optional trailing comma https://stackoverflow.com/a/43143459/2085356. */
    ) =>
    {
        {
            let mut styled_texts: TuiStyledTexts = Default::default();
            $(
                styled_texts += $styled_text_arg;
            )*
            styled_texts
        }
    };
}

/// Use [tui_styled_texts!] macro for easier construction.
#[derive(Debug, Clone, Default, size_of::SizeOf)]
pub struct TuiStyledTexts {
    pub inner: Vec<TuiStyledText>,
}

mod tui_styled_texts_impl_ops {
    use super::*;

    impl TuiStyledTexts {
        pub fn len(&self) -> usize { self.inner.len() }

        pub fn is_empty(&self) -> bool { self.inner.is_empty() }
    }

    impl AddAssign<TuiStyledText> for TuiStyledTexts {
        fn add_assign(&mut self, rhs: TuiStyledText) { self.inner.push(rhs); }
    }

    impl AddAssign<TuiStyledTexts> for TuiStyledTexts {
        fn add_assign(&mut self, rhs: TuiStyledTexts) { self.inner.extend(rhs.inner); }
    }

    impl Index<usize> for TuiStyledTexts {
        type Output = TuiStyledText;

        fn index(&self, index: usize) -> &Self::Output { &self.inner[index] }
    }
}

mod tui_styled_texts_impl_display {
    use super::*;

    impl ConvertToPlainText for TuiStyledTexts {
        fn to_plain_text_us(&self) -> UnicodeString {
            let mut it = UnicodeString::default();
            for styled_text in self.inner.iter() {
                it = it + styled_text.get_text();
            }
            it
        }
    }

    impl TuiStyledTexts {
        pub fn display_width(&self) -> ChUnit { self.to_plain_text_us().display_width }
    }
}

mod tui_styled_texts_impl_debug {
    use super::*;

    impl PrettyPrintDebug for TuiStyledTexts {
        fn pretty_print_debug(&self) -> String {
            let mut it = vec![];
            for (index, item) in self.inner.iter().enumerate() {
                let string = format!(
                    "{index}: [{}, {}]",
                    item.get_style(),
                    item.get_text().string
                );
                it.push(string);
            }
            it.join("\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::{assert_eq2,
                             ch,
                             throws,
                             throws_with_return,
                             tui_stylesheet,
                             ChUnit,
                             CommonResult,
                             RgbValue,
                             TuiColor,
                             TuiStyle,
                             TuiStylesheet};

    use crate::{TuiStyledText, TuiStyledTexts};

    #[test]
    fn test_create_styled_text_with_dsl() -> CommonResult<()> {
        throws!({
            let st_vec = helpers::create_styled_text()?;
            assert_eq2!(st_vec.is_empty(), false);
            assert_eq2!(st_vec.len(), 2);
        })
    }

    mod helpers {
        use super::*;

        pub fn create_styled_text() -> CommonResult<TuiStyledTexts> {
            throws_with_return!({
                let stylesheet = create_stylesheet()?;
                let maybe_style1 = stylesheet.find_style_by_id(1);
                let maybe_style2 = stylesheet.find_style_by_id(2);

                tui_styled_texts! {
                    tui_styled_text! {
                        @style: maybe_style1.unwrap(),
                        @text: "Hello",
                    },
                    tui_styled_text! {
                        @style: maybe_style2.unwrap(),
                        @text: "World",
                    }
                }
            })
        }

        pub fn create_stylesheet() -> CommonResult<TuiStylesheet> {
            throws_with_return!({
                tui_stylesheet! {
                    TuiStyle {
                        id: 1,
                        padding: Some(ch!(1)),
                        color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(55, 55, 100))),
                        ..Default::default()
                    },
                    TuiStyle {
                        id: 2,
                        padding: Some(ch!(1)),
                        color_bg: Some(TuiColor::Rgb(RgbValue::from_u8(55, 55, 248))),
                        ..Default::default()
                    }
                }
            })
        }
    }
}
