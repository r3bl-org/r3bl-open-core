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

use r3bl_rs_utils_core::*;

use super::*;
use crate::{constants::*, *};

/// This is the main function that the [editor] uses this in order to display the markdown to the
/// user.It is responsible for converting:
/// - from a &[Vec] of [US] which comes from the [editor],
/// - into a [StyleUSFragmentLines], which the [editor] will clip & render.
/// ## Arguments
/// - `editor_text` - The text that the user has typed into the editor.
/// - `current_box_computed_style` - The computed style of the box that the editor is in.
// AI: ∞. this is the main entry point for the editor to use this module.
pub fn try_parse_and_highlight(
    editor_text: &Vec<US>,
    maybe_current_box_computed_style: &Option<Style>,
) -> CommonResult<StyleUSFragmentLines> {
    // Convert the editor text into a string.
    let editor_text_to_string = {
        let mut acc = Vec::<&str>::new();
        for line in editor_text {
            acc.push(line.string.as_str());
            acc.push("\n");
        }
        acc.join("\n")
    };

    // Try and parse `editor_text_to_string` into a `Document`.
    match parse_markdown(&editor_text_to_string) {
        Ok((_, document)) => Ok(StyleUSFragmentLines::from_document(
            &document,
            maybe_current_box_computed_style,
        )),
        Err(_) => CommonError::new_err_with_only_type(CommonErrorType::ParsingError),
    }
}
