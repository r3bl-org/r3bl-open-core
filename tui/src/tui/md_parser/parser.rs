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

use crate::{constants::*, tui::misc_types::list_of::List, *};

/// This is the main parser entry point. It takes a string slice and if it can be parsed, returns a
/// [MdDocument] that represents the parsed Markdown.
///
/// 1. [MdLineFragments] roughly corresponds to a line of parsed text.
/// 2. [MdDocument] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [MdDocument] corresponds to a block of Markdown [MdBlockElement], which can be
/// one of the following variants:
/// 1. Metadata title. The parsers in [parse_metadata_kv] file handle this.
/// 2. Metadata tags. The parsers in [parse_metadata_kcsv] file handle this.
/// 3. Heading (which contains a [HeadingLevel] & [MdLineFragments]).
/// 4. Smart ordered & unordered list (which itself contains a [Vec] of [MdLineFragments]. The
///    parsers in [mod@parse_block_smart_list] file handle this.
/// 5. Code block (which contains string slices of the language & code). The parsers in
///    [mod@parse_block_code] file handle this.
/// 6. line (which contains a [MdLineFragments]). The parsers in [parse_element] file handle this.
#[rustfmt::skip]
pub fn parse_markdown(input: &str) -> IResult<&str, MdDocument<'_>> {
    // key: TAGS, value: CSV parser.
    fn parse_tags_list(input: &str) -> IResult<&str, List<&str>>
    {
        let it = parse_csv_opt_eol(TAGS, input);
        it
    }

    // key: AUTHORS, value: CSV parser.
    fn parse_authors_list(input: &str) -> IResult<&str, List<&str>>
    {
        let it = parse_csv_opt_eol(AUTHORS, input);
        it
    }

    // key: TITLE, value: KV parser.
    fn parse_title_value(input: &str) -> IResult<&str, &str>
    {
        let it = parse_kv_opt_eol(TITLE, input);
        it
    }

    // key: DATE, value: KV parser.
    fn parse_date_value(input: &str) -> IResult<&str, &str>
    {
        let it = parse_kv_opt_eol(DATE, input);
        it
    }

    // BOOKM: main parser entry point
    let (input, output) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_value,                   MdBlockElement::Title),
            map(parse_tags_list,                     MdBlockElement::Tags),
            map(parse_authors_list,                  MdBlockElement::Authors),
            map(parse_date_value,                    MdBlockElement::Date),
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
    use crossterm::style::Stylize;
    use r3bl_rs_utils_core::*;

    use super::*;

    // BOOKM: main parser test entry point

    #[test]
    fn test_parse_markdown_with_invalid_text_in_heading() {
        let input = ["# LINE 1", "", "##% LINE 2 FOO_BAR:", ""].join("\n");
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{:?}", remainder);
        println!("\nblocks:\n{:#?}", blocks);
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 3);
        assert_eq2!(
            blocks[0],
            MdBlockElement::Heading(HeadingData {
                heading_level: HeadingLevel { level: 1 },
                text: "LINE 1",
            })
        );
        assert_eq2!(
            blocks[1],
            MdBlockElement::Text(list![]), // Empty line.
        );
        assert_eq2!(
            blocks[2],
            MdBlockElement::Text(list![
                MdLineFragment::Plain("##% LINE 2 FOO"),
                MdLineFragment::Plain("_BAR:")
            ])
        );
    }

    #[test]
    fn test_parse_markdown_single_line_plain_text() {
        let input = ["_this should not be italic", ""].join("\n");
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{:?}", remainder);
        println!("\nblocks:\n{:?}", blocks);
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 1);
        assert_eq2!(
            blocks[0],
            MdBlockElement::Text(list![MdLineFragment::Plain(
                "_this should not be italic"
            )])
        );
    }

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
                heading_level: HeadingLevel { level: 1 },
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
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("fish"),
                vec![],
            )),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![""],
            )),
            MdBlockElement::Heading(HeadingData {
                heading_level: HeadingLevel { level: 2 },
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
                "█ → ".magenta().bold(),
                format!("{:?}", block).green()
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
        let input = [
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
