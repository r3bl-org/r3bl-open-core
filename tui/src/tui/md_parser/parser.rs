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

use nom::{branch::*, combinator::*, multi::*, IResult};

use crate::*;

/// This is the main parser entry point. It takes a string slice and if it can be parsed, returns a
/// [Document] that represents the parsed Markdown.
///
/// 1. [Fragments] roughly corresponds to a line of parsed text.
/// 2. [Document] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [Document] corresponds to a block of Markdown [Block], which can be one of the
/// following variants:
/// 1. heading (which contains a [Level] & [Fragments]),
/// 2. ordered & unordered list (which itself contains a [Vec] of [Fragments],
/// 3. code block (which contains string slices of the language & code),
/// 4. line (which contains a [Fragments]).
#[rustfmt::skip]
pub fn parse_markdown(input: &str) -> IResult<&str, Document> {
    many0(
        /* Each of these parsers end up scanning until EOL. */
        alt((
            map(parse_title,                         Block::Title),
            map(parse_tags,                          Block::Tags),
            map(parse_block_heading,                 Block::Heading),
            map(parse_block_unordered_list,          Block::UnorderedList),
            map(parse_block_ordered_list,            Block::OrderedList),
            map(parse_block_code,                    Block::CodeBlock),
            map(parse_block_markdown_text_until_eol, Block::Text),
        )),
    )(input)
}

#[cfg(test)]
mod tests {
    use ansi_term::Color::*;
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_parse_markdown() {
        let (remainder, vec_block) =
            parse_markdown(include_str!("test_assets/valid_md_input.md")).unwrap();
        let expected_vec = vec![
            Block::Title("Something"),
            Block::Tags(vec!["tag1", "tag2", "tag3"]),
            Block::Heading((Level::Heading1, vec![Fragment::Plain("Foobar")])),
            Block::Text(vec![]), // Empty line.
            Block::Text(vec![Fragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            Block::Text(vec![]), // Empty line.
            Block::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            Block::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            Block::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            Block::Heading((Level::Heading2, vec![Fragment::Plain("Installation")])),
            Block::Text(vec![]), // Empty line.
            Block::Text(vec![
                Fragment::Plain("Use the package manager "),
                Fragment::Link(("pip", "https://pip.pypa.io/en/stable/")),
                Fragment::Plain(" to install foobar."),
            ]),
            Block::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
            Block::UnorderedList(vec![
                vec![Fragment::Plain("ul1")],
                vec![Fragment::Plain("ul2")],
            ]),
            Block::OrderedList(vec![
                vec![Fragment::Plain("ol1")],
                vec![Fragment::Plain("ol2")],
            ]),
            Block::UnorderedList(vec![
                vec![Fragment::Checkbox(false), Fragment::Plain(" todo")],
                vec![Fragment::Checkbox(true), Fragment::Plain(" done")],
            ]),
            Block::Text(vec![Fragment::Plain("end")]),
        ];

        // Print last 2 items.
        for block in vec_block.iter().skip(vec_block.len() - 2) {
            println!(
                "{0} {1}",
                Purple.bold().paint("█ → "),
                Green.paint(format!("{:?}", block))
            );
        }

        assert_eq2!(remainder, "");

        vec_block
            .iter()
            .zip(expected_vec.iter())
            .for_each(|(lhs, rhs)| assert_eq2!(lhs, rhs));
    }
}
