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

// IDEA: do integrations here

use r3bl_rs_utils_core::*;

use crate::*;

pub mod syn_hi_md_editor_content {
    use super::*;

    // IDEA: this is the "main" function for syn hi editor content
    pub fn highlight(editor_text: &Vec<UnicodeString>) -> Vec<List<(Style, String)>> {
        // AI: 4. parse markdown into Document
        todo!()
    }
}

pub mod translate_to_style_us_tuple {
    pub fn translate(document: Document) -> Vec<List<(Style, String)>> {
        // AI: 3. iterate over document block (which represents a line) & convert to &[(Style,String)]
        // AI: 2. special handling of Document::Block(CodeBlock) since they may contain multiple lines
        todo!()
    }
}

pub mod code_block_to_line {
    use super::*;

    pub struct CodeBlockLine<'a> {
        pub language: &'a str,
        pub content: &'a CodeBlockLineContent<'a>,
    }

    enum CodeBlockLineContent<'a> {
        Text(&'a str),
        Newline,
    }

    pub fn convert_from_code_block_into_lines(input: &CodeBlock) -> Vec<&CodeBlockLine> {
        // AI: 1. iterate over code_block.text.split("\n") & convert to CodeBlockLine
        todo!()
    }
}

pub mod md_theme {
    use super::*;

    // IDEA: `Document` pieces to `Style` mapping
}
