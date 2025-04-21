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

use sizing::StringTuiStyledText;
use smallstr::SmallString;
use smallvec::SmallVec;

use crate::TuiStyle;

pub(in crate::core::tui_core::tui_styled_text) mod sizing {
    use super::*;

    /// Default internal storage for [TuiStyledText], which is very small.
    pub(crate) type StringTuiStyledText = SmallString<[u8; MAX_CHARS_IN_SMALL_STRING]>;
    const MAX_CHARS_IN_SMALL_STRING: usize = 8;

    pub(crate) type VecTuiStyledText = SmallVec<[TuiStyledText; MAX_ITEMS_IN_SMALL_VEC]>;
    const MAX_ITEMS_IN_SMALL_VEC: usize = 32;
}

/// Macro to make building [TuiStyledText] easy.
///
/// Here's an example.
/// ```rust
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
        $crate::TuiStyledText::new($style_arg, $text_arg.to_string())
    };
}

/// Use [tui_styled_text!] macro for easier construction.
#[derive(Debug, Clone)]
pub struct TuiStyledText {
    pub style: TuiStyle,
    pub text: StringTuiStyledText,
}

impl Default for TuiStyledText {
    fn default() -> Self {
        TuiStyledText {
            style: TuiStyle::default(),
            text: "".into(),
        }
    }
}

impl TuiStyledText {
    pub fn new(style: TuiStyle, arg_styled_text: impl Into<StringTuiStyledText>) -> Self {
        TuiStyledText {
            style,
            text: arg_styled_text.into(),
        }
    }

    pub fn get_text(&self) -> &str { self.text.as_str() }

    pub fn get_style(&self) -> &TuiStyle { &self.style }
}
