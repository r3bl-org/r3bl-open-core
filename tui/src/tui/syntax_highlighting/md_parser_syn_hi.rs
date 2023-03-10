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

//! This module holds the integration or glue code that ties together:
//! 1. [md_parser] - Responsible for parsing markdown into a [Document] data structure.
//! 2. [syntax_highlighting] - Responsible for converting a [Document] into a list of tuples of
//!    [Style] and [String].
//! 3. [editor] - Responsible for displaying the [Document] to the user.

use r3bl_rs_utils_core::*;

use crate::*;

/// This module is responsible for converting:
/// - from a &[Vec] of [US] which comes from the [editor],
/// - into a [Vec] of [StyleUSFragmentLine], which the [editor] will clip & render.
///
/// This is the main function that the [editor] uses this in order to display the markdown to the
/// user.
pub mod syn_hi_md_editor_content {
    use super::*;

    // AI: 0. this is the main entry point for the editor to use this module.
    pub fn highlight(editor_text: &Vec<US>) -> CommonResult<Vec<StyleUSFragmentLine>> {
        // Convert the editor text into a string.
        let mut acc = Vec::<&str>::new();
        for line in editor_text {
            acc.push(line.string.as_str());
            acc.push("\n");
        }
        let editor_text_string = acc.join("\n");

        // Try and parse the string into a Document.
        match parse_markdown(&editor_text_string) {
            Ok((_, document)) => Ok(translate_to_style_us_tuple::translate(document)),
            Err(_) => CommonError::new_err_with_only_type(CommonErrorType::ParsingError),
        }
    }
}

/// This module is responsible for converting a [Document] into a [Vec] of [StyleUSFragmentLine].
pub mod translate_to_style_us_tuple {
    use super::*;

    pub fn translate(document: Document) -> Vec<StyleUSFragmentLine> {
        // AI: 1. iterate over document block (which represents a line) & convert to &[(Style,US)]
        todo!()
    }
}

/// This module is responsible for formatting a [Document] into [Style]s.
pub mod md_theme {
    use super::*;

    // IDEA: `Document` pieces to `Style` mapping
}
