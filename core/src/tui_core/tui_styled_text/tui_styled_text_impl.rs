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

use crate::{TuiStyle, UnicodeString};

/// Macro to make building [TuiStyledText] easy.
///
/// Here's an example.
/// ```rust
/// use r3bl_rs_utils_core::*;
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
