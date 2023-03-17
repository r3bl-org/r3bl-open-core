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
/// [MdDocument] that represents the parsed Markdown.
///
/// 1. [MdLineFragments] roughly corresponds to a line of parsed text.
/// 2. [MdDocument] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [MdDocument] corresponds to a block of Markdown [MdBlockElement], which can be
/// one of the following variants:
/// 1. heading (which contains a [HeadingLevel] & [MdLineFragments]),
/// 2. ordered & unordered list (which itself contains a [Vec] of [MdLineFragments],
/// 3. code block (which contains string slices of the language & code),
/// 4. line (which contains a [MdLineFragments]).
#[rustfmt::skip]
pub fn parse_markdown(input: &str) -> IResult<&str, MdDocument> {
    many0(
        /* Each of these parsers end up scanning until EOL. */
        alt((
            map(parse_title,                         MdBlockElement::Title),
            map(parse_tags,                          MdBlockElement::Tags),
            map(parse_block_heading,                 MdBlockElement::Heading),
            map(parse_block_unordered_list,          MdBlockElement::UnorderedList),
            map(parse_block_ordered_list,            MdBlockElement::OrderedList),
            map(parse_block_code,                    MdBlockElement::CodeBlock),
            map(parse_block_markdown_text_until_eol, MdBlockElement::Text),
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
            MdBlockElement::Title("Something"),
            MdBlockElement::Tags(vec!["tag1", "tag2", "tag3"]),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                content: vec![MdLineFragment::Plain("Foobar")],
            }),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::Text(vec![MdLineFragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading2,
                content: vec![MdLineFragment::Plain("Installation")],
            }),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::Text(vec![
                MdLineFragment::Plain("Use the package manager "),
                MdLineFragment::Link(HyperlinkData::from((
                    "pip",
                    "https://pip.pypa.io/en/stable/",
                ))),
                MdLineFragment::Plain(" to install foobar."),
            ]),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
            MdBlockElement::UnorderedList(vec![
                vec![MdLineFragment::Plain("ul1")],
                vec![MdLineFragment::Plain("ul2")],
            ]),
            MdBlockElement::OrderedList(vec![
                vec![MdLineFragment::Plain("ol1")],
                vec![MdLineFragment::Plain("ol2")],
            ]),
            MdBlockElement::UnorderedList(vec![
                vec![
                    MdLineFragment::Checkbox(false),
                    MdLineFragment::Plain(" todo"),
                ],
                vec![
                    MdLineFragment::Checkbox(true),
                    MdLineFragment::Plain(" done"),
                ],
            ]),
            MdBlockElement::Text(vec![MdLineFragment::Plain("end")]),
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
