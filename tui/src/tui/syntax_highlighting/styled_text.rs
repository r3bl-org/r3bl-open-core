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

use std::ops::{Add, AddAssign, Deref, DerefMut};

use r3bl_rs_utils_core::*;

use crate::*;

/// Use [styled_text!] macro for easier construction.
#[derive(Debug, Clone, Default)]
pub struct StyledText {
    plain_text: UnicodeString,
    style: Style,
}

mod styled_text_impl {
    use super::*;

    impl StyledText {
        /// Just as a precaution, the `text` argument is passed through
        /// [try_strip_ansi](ANSIText::try_strip_ansi) method to remove any ANSI escape sequences.
        pub fn new(text: String, style: Style) -> Self {
            let plain_text = match ANSIText::try_strip_ansi(&text) {
                Some(plain_text) => plain_text,
                None => text,
            };
            StyledText {
                plain_text: UnicodeString::from(plain_text),
                style,
            }
        }

        pub fn get_plain_text(&self) -> &UnicodeString { &self.plain_text }

        pub fn get_style(&self) -> &Style { &self.style }
    }
}

/// Macro to make building [StyledText] easy.
///
/// Here's an example.
/// ```ignore
/// let st = styled_text! {
///   "Hello".to_string(),
///   maybe_style1.unwrap()
/// };
/// ```
#[macro_export]
macro_rules! styled_text {
    () => {
        StyledText::new(String::new(), Style::default())
    };
    ($text:expr) => {
        StyledText::new($text.to_string(), Style::default())
    };
    ($text:expr, $style:expr) => {
        StyledText::new($text.to_string(), $style)
    };
}

/// Use [styled_texts!] macro for easier construction.
#[derive(Default, Debug, Clone)]
pub struct StyledTexts {
    styled_texts: List<StyledText>,
}

mod impl_styled_texts {
    use super::*;

    impl Add<StyledText> for StyledTexts {
        type Output = StyledTexts;
        fn add(mut self, other: StyledText) -> Self::Output {
            self.push(other);
            self
        }
    }

    impl AddAssign<StyledText> for StyledTexts {
        fn add_assign(&mut self, other: StyledText) { self.push(other); }
    }

    impl Deref for StyledTexts {
        type Target = Vec<StyledText>;
        fn deref(&self) -> &Self::Target { &self.styled_texts }
    }

    impl DerefMut for StyledTexts {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.styled_texts }
    }

    impl StyledTexts {
        pub fn pretty_print(&self) -> String {
            let mut it = vec![];
            for (index, item) in self.iter().enumerate() {
                let string = format!(
                    "{index}: [{}, {}]",
                    item.get_style(),
                    item.get_plain_text().string
                );
                it.push(string);
            }
            it.join("\n")
        }

        pub fn get_plain_text(&self) -> UnicodeString {
            let mut it = UnicodeString::default();
            for styled_text in self.iter() {
                it = it + &styled_text.plain_text;
            }
            it
        }

        pub fn display_width(&self) -> ChUnit { self.get_plain_text().display_width }

        pub fn render_into(&self, render_ops: &mut RenderOps) {
            for styled_text in self.iter() {
                let style = styled_text.style.clone();
                let text = styled_text.plain_text.clone();
                render_ops.push(RenderOp::ApplyColors(style.clone().into()));
                render_ops.push(RenderOp::PaintTextWithAttributes(text.string, style.into()));
                render_ops.push(RenderOp::ResetColor);
            }
        }
    }
}

/// Macro to make building [`StyledTexts`] easy.
///
/// Here's an example.
/// ```ignore
/// let mut st_vec = styled_texts! {
///   styled_text! {
///     "Hello".to_string(),
///     maybe_style1.unwrap()
///   },
///   styled_text! {
///     "World".to_string(),
///     maybe_style2.unwrap()
///   }
/// };
/// ```
#[macro_export]
macro_rules! styled_texts {
  ($($style:expr),*) => {
    {
      let mut styled_texts: StyledTexts = Default::default();
      $(
        styled_texts += $style;
      )*
      styled_texts
    }
  };
}
