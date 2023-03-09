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

/// This module is responsible for converting a `&Vec<UnicodeString>` into a `Vec<List<(Style,
/// String>)>>`. This is the main function that the [editor] uses this in order to display the
/// markdown to the user.
pub mod syn_hi_md_editor_content {
    use super::*;

    // IDEA: this is the "main" function for syn hi editor content
    pub fn highlight(editor_text: &Vec<UnicodeString>) -> Vec<List<(Style, String)>> {
        // AI: 2. parse markdown into Document
        todo!()
    }
}

/// This module is responsible for converting a [Document] into a list of tuples of [Style] and
/// [String].
pub mod translate_to_style_us_tuple {
    use super::*;

    pub fn translate(document: Document) -> Vec<List<(Style, String)>> {
        // AI: 1. iterate over document block (which represents a line) & convert to &[(Style,String)]
        todo!()
    }
}

/// This module is responsible for formatting a [Document] into [Style]s.
pub mod md_theme {
    use super::*;

    // IDEA: `Document` pieces to `Style` mapping
}
