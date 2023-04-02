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
    let (input, output) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_opt_eol,                 MdBlockElement::Title),
            map(parse_tags_opt_eol,                  MdBlockElement::Tags),

            map(parse_block_heading_opt_eol,         MdBlockElement::Heading),

            map(parse_block_smart_list,              MdBlockElement::SmartList),

            map(parse_block_code,                    MdBlockElement::CodeBlock),

            map(parse_block_markdown_text_until_eol, MdBlockElement::Text),
        )),
    )(input)?;
    let it = List::from(output);
    Ok((input, it))
}

#[cfg(test)]
mod tests {
    use ansi_term::Color::*;
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_parse_markdown_valid() {
        let input = vec![
            "@title: Something",
            "@tags: tag1, tag2, tag3",
            "# Foobar",
            "",
            "Foobar is a Python library for dealing with word pluralization.",
            "",
            "```bash",
            "pip install foobar",
            "```",
            "```fish",
            "```",
            "```python",
            "",
            "```",
            "## Installation",
            "",
            "Use the package manager [pip](https://pip.pypa.io/en/stable/) to install foobar.",
            "```python",
            "import foobar",
            "",
            "foobar.pluralize('word') # returns 'words'",
            "foobar.pluralize('goose') # returns 'geese'",
            "foobar.singularize('phenomena') # returns 'phenomenon'",
            "```",
            "- ul1",
            "- ul2",
            "1. ol1",
            "2. ol2",
            "- [ ] todo",
            "- [x] done",
            "end",
            "",
        ]
        .join("\n");
        let (remainder, vec_block) = parse_markdown(&input).unwrap();
        let expected_vec = vec![
            MdBlockElement::Title("Something"),
            MdBlockElement::Tags(list!["tag1", "tag2", "tag3"]),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                text: "Foobar",
            }),
            MdBlockElement::Text(list![]), // Empty line.
            MdBlockElement::Text(list![MdLineFragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            MdBlockElement::Text(list![]), // Empty line.
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading2,
                text: "Installation",
            }),
            MdBlockElement::Text(list![]), // Empty line.
            MdBlockElement::Text(list![
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
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("ul1"),
                ],],
                BulletKind::Unordered,
                0,
            )),
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("ul2"),
                ],],
                BulletKind::Unordered,
                0,
            )),
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::OrderedListBullet {
                        indent: 0,
                        number: 1,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("ol1"),
                ],],
                BulletKind::Ordered(1),
                0,
            )),
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::OrderedListBullet {
                        indent: 0,
                        number: 2,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("ol2"),
                ],],
                BulletKind::Ordered(2),
                0,
            )),
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Checkbox(false),
                    MdLineFragment::Plain(" todo"),
                ],],
                BulletKind::Unordered,
                0,
            )),
            MdBlockElement::SmartList((
                list![list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Checkbox(true),
                    MdLineFragment::Plain(" done"),
                ],],
                BulletKind::Unordered,
                0,
            )),
            MdBlockElement::Text(list![MdLineFragment::Plain("end")]),
        ];

        // Print a few of the last items.
        for block in vec_block.iter().skip(vec_block.len() - 7) {
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

    #[test]
    fn test_markdown_invalid() {
        let input = vec![
            "@tags: [foo, bar",
            "",
            "```rs",
            "let a=1;",
            "```",
            "",
            "*italic* **bold** [link](https://example.com)",
            "",
            "`inline code`",
        ]
        .join("\n");
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        assert_eq2!(remainder, "`inline code`");
        assert_eq2!(blocks.len(), 6);
    }
}
