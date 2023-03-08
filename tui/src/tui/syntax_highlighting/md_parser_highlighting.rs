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
    use super::*;

    pub fn translate(document: Document) -> Vec<List<(Style, String)>> {
        // AI: 3. iterate over document block (which represents a line) & convert to &[(Style,String)]
        // AI: 2. special handling of Document::Block(CodeBlock) since they may contain multiple lines
        todo!()
    }
}

pub mod code_block_to_line {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub struct CodeBlockLine<'a> {
        pub language: &'a str,
        pub content: CodeBlockLineContent<'a>,
    }

    #[derive(Debug, PartialEq)]
    pub enum CodeBlockLineContent<'a> {
        Text(&'a str),
        EmptyLine,
        StartTag,
        EndTag,
    }

    /// At a minimum, a [CodeBlock] will be 2 lines of text.
    /// 1. The first line will be the language of the code block, eg: "```rust"
    /// 2. The second line will be the end of the code block, eg: "```" Then there may be some
    /// number of lines of text in the middle. These lines are stored in the [text](CodeBlock.text)
    /// field.
    pub fn convert_from_code_block_into_lines<'input>(
        input: &'input CodeBlock,
    ) -> Vec<CodeBlockLine<'input>> {
        let lang = input.language;
        let lines = &input.code_block_lines;

        let mut acc = Vec::with_capacity(lines.len() + 2);
        acc.push(CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::StartTag,
        });
        for line in lines {
            acc.push(CodeBlockLine {
                language: lang,
                content: CodeBlockLineContent::Text(line),
            });
        }
        acc.push(CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::EndTag,
        });

        acc
    }

    // AI: 0. make this test more robust
    #[test]
    fn test_convert_from_code_block_into_lines() {
        let input = CodeBlock {
            language: "rust",
            code_block_lines: vec![""],
        };
        let expected = vec![
            CodeBlockLine {
                language: "rust",
                content: CodeBlockLineContent::StartTag,
            },
            CodeBlockLine {
                language: "rust",
                content: CodeBlockLineContent::Text(""),
            },
            CodeBlockLine {
                language: "rust",
                content: CodeBlockLineContent::EndTag,
            },
        ];
        let output = convert_from_code_block_into_lines(&input);
        dbg!(&output);
        dbg!(&expected);
        assert_eq2!(output, expected);
    }
}

pub mod md_theme {
    use super::*;

    // IDEA: `Document` pieces to `Style` mapping
}
