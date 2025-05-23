/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use std::{fmt::Debug,
          ops::{AddAssign, Index}};

use super::{sizing::VecTuiStyledText, TuiStyledText};
use crate::{join_with_index_fmt, ok, ConvertToPlainText, GCString, InlineString};

/// Macro to make building [`TuiStyledTexts`] easy.
///
/// Here's an example.
/// ```
/// # use r3bl_tui::{tui_styled_text, tui_styled_texts, TuiStyledText, TuiStyle};
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
            let mut styled_texts: $crate::TuiStyledTexts = Default::default();
            $(
                styled_texts += $styled_text_arg;
            )*
            styled_texts
        }
    };
}

/// Use [tui_styled_texts!] macro for easier construction.
#[derive(Clone, Default)]
pub struct TuiStyledTexts {
    pub inner: VecTuiStyledText,
}

mod impl_ops {
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

mod impl_display {
    use super::*;
    use crate::{join, ColWidth};

    impl ConvertToPlainText for TuiStyledTexts {
        fn to_plain_text(&self) -> InlineString {
            join!(
                from: self.inner,
                each: styled_text,
                delim: "",
                format: "{}", styled_text.get_text()
            )
        }
    }

    impl TuiStyledTexts {
        pub fn display_width(&self) -> ColWidth {
            let plain_text = self.to_plain_text();

            GCString::width(plain_text.as_str())
        }
    }
}

mod impl_debug {
    use super::*;

    impl Debug for TuiStyledTexts {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            join_with_index_fmt!(
                fmt: f,
                from: self.inner,
                each: styled_text,
                index: index,
                delim: "\n",
                format: "{index}: [{}, {}]",
                styled_text.get_style(),
                styled_text.get_text()
            );
            ok!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2,
                ch,
                throws,
                throws_with_return,
                tui_styled_text,
                tui_stylesheet,
                CommonResult,
                TuiStyle,
                TuiStylesheet};

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
        use crate::{tui_color, tui_style_attrib};

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
                        id: tui_style_attrib::id(1),
                        padding: Some(ch(1)),
                        color_bg: Some(tui_color!(55, 55, 100)),
                        ..Default::default()
                    },
                    TuiStyle {
                        id: tui_style_attrib::id(2),
                        padding: Some(ch(1)),
                        color_bg: Some(tui_color!(55, 55, 248)),
                        ..Default::default()
                    }
                }
            })
        }
    }
}
