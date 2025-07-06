/*
 *   Copyright (c) 2025 R3BL LLC
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

use std::fmt::Display;

use r3bl_tui::{InlineString, fg_frozen_blue, fg_lizard_green, fg_silver_metallic,
               fg_slate_gray, fg_soft_pink, inline_string};

#[must_use]
pub fn colon() -> InlineString { dim(":") }

#[must_use]
pub fn comma() -> InlineString { dim(",") }

#[must_use]
pub fn period() -> InlineString { dim(".") }

#[must_use]
pub fn exclamation() -> InlineString { dim("!") }

/// Normal or default text style.
pub fn normal(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_silver_metallic(text).to_small_str()
}

/// Error text style.
pub fn error(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_soft_pink(text).to_small_str()
}

/// Emphasis text style to highlight.
pub fn emphasis(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_lizard_green(text).to_small_str()
}

pub fn emphasis_delete(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_soft_pink(text).to_small_str()
}

/// De-emphasize (dim) text.
pub fn dim(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_slate_gray(text).to_small_str()
}

/// This is for `readline_async` prompt segment. This the part of the prompt which
/// gives the user instruction, e.g.: `Branch name to create `.
pub fn prompt_seg_normal(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_frozen_blue(text).to_small_str()
}

/// This is for `readline_async` prompt segment. This is the part of the prompt
/// which gives the user bailout directions, e.g.: `(Ctrl+C exits)`.
pub fn prompt_seg_bail(arg_text: impl Display) -> InlineString {
    let text = inline_string!("{}", arg_text);
    fg_soft_pink(text)
        .italic()
        .bg_moonlight_blue()
        .to_small_str()
}
