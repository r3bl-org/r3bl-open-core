/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

use nom::{IResult, Parser, branch::alt, combinator::map, multi::many0};

use crate::{List, MdDocument, MdElement,
            md_parser::constants::{AUTHORS, DATE, TAGS, TITLE},
            parse_block_markdown_text_with_or_without_new_line, parse_csv_opt_eol,
            parse_fenced_code_block, parse_heading_in_single_line,
            parse_smart_list_block, parse_unique_kv_opt_eol};

// XMARK: Main Markdown parser entry point

/// This is the main parser entry point, aka, the root parser. It takes a string slice and
/// if it can be parsed, returns a [`MdDocument`] that represents the parsed Markdown.
///
/// 1. [`crate::MdLineFragments`] roughly corresponds to a line of parsed text.
/// 2. [`MdDocument`] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [`MdDocument`] corresponds to a block of Markdown [`MdElement`], which can
/// be one of the following variants:
/// 1. Metadata title. The parsers in [`crate::parse_metadata_kv`] file handle this.
/// 2. Metadata tags. The parsers in [`crate::parse_metadata_kcsv`] file handle this.
/// 3. Heading (which contains a [`crate::HeadingLevel`] & [`crate::MdLineFragments`]).
/// 4. Smart ordered & unordered list (which itself contains a [Vec] of
///    [`crate::MdLineFragments`]. The parsers in [`mod@parse_smart_list_block`] function
///    handle this.
/// 5. Code block (which contains string slices of the language & code). The parsers in
///    [`mod@parse_fenced_code_block`] handle this.
/// 6. line (which contains a [`crate::MdLineFragments`]). The parsers in
///    [`mod@crate::fragment`] handle this.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain valid markdown.
#[rustfmt::skip]
pub fn parse_markdown(input: &str) -> IResult<&str, MdDocument<'_>> {
    let (input, output) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_value,                                  MdElement::Title),
            map(parse_tags_list,                                    MdElement::Tags),
            map(parse_authors_list,                                 MdElement::Authors),
            map(parse_date_value,                                   MdElement::Date),
            map(parse_heading_in_single_line,                       MdElement::Heading),
            map(parse_smart_list_block,                             MdElement::SmartList),
            map(parse_fenced_code_block,                            MdElement::CodeBlock),
            map(parse_block_markdown_text_with_or_without_new_line, MdElement::Text),
        )),
    ).parse(input)?;

    let it = List::from(output);
    Ok((input, it))
}

// key: TAGS, value: CSV parser.
fn parse_tags_list(input: &str) -> IResult<&str, List<&str>> {
    parse_csv_opt_eol(TAGS, input)
}

// key: AUTHORS, value: CSV parser.
fn parse_authors_list(input: &str) -> IResult<&str, List<&str>> {
    parse_csv_opt_eol(AUTHORS, input)
}

// key: TITLE, value: KV parser.
fn parse_title_value(input: &str) -> IResult<&str, &str> {
    parse_unique_kv_opt_eol(TITLE, input)
}

// key: DATE, value: KV parser.
fn parse_date_value(input: &str) -> IResult<&str, &str> {
    parse_unique_kv_opt_eol(DATE, input)
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_integration_block_smart_lists {
    use crate::{MdDocument, PrettyPrintDebug, assert_eq2, parse_markdown};

    #[test]
    fn test_parse_valid_md_ol_with_indent() {
        let input = [
            "start",
            "1. ol1",
            "  2. ol2",
            "     ol2.1",
            "    3. ol3",
            "       ol3.1",
            "       ol3.2",
            "end",
            "",
        ]
        .join("\n");

        let expected_output = [
            "start",
            "[  ┊1.│ol1┊  ]",
            "[  ┊  2.│ol2┊ → ┊    │ol2.1┊  ]",
            "[  ┊    3.│ol3┊ → ┊      │ol3.1┊ → ┊      │ol3.2┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument<'_> = result.unwrap().1;

        // md_doc.console_log_fg();
        // remainder.console_log_bg();

        assert_eq2!(remainder, "");
        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );
    }

    #[test]
    fn test_parse_valid_md_ul_with_indent() {
        let input = [
            "start",
            "- ul1",
            "  - ul2",
            "    ul2.1",
            "    - ul3",
            "      ul3.1",
            "      ul3.2",
            "end",
            "",
        ]
        .join("\n");

        let expected_output = [
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊───┤ul2┊ → ┊   │ul2.1┊  ]",
            "[  ┊─────┤ul3┊ → ┊     │ul3.1┊ → ┊     │ul3.2┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument<'_> = result.unwrap().1;

        // console_log!(md_doc);
        // console_log!(remainder);

        assert_eq2!(remainder, "");
        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );
    }

    #[test]
    fn test_parse_valid_md_multiline_no_indent() {
        let input = vec![
            "start",
            "- ul1",
            "- ul2",
            "  ul2.1",
            "  ",
            "- ul3",
            "  ul3.1",
            "  ul3.2",
            "1. ol1",
            "2. ol2",
            "   ol2.1",
            "3. ol3",
            "   ol3.1",
            "   ol3.2",
            "- [ ] todo",
            "- [x] done",
            "end",
            "",
        ]
        .join("\n");

        let expected_output = [
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊─┤ul2┊ → ┊ │ul2.1┊  ]",
            "  ",
            "[  ┊─┤ul3┊ → ┊ │ul3.1┊ → ┊ │ul3.2┊  ]",
            "[  ┊1.│ol1┊  ]",
            "[  ┊2.│ol2┊ → ┊  │ol2.1┊  ]",
            "[  ┊3.│ol3┊ → ┊  │ol3.1┊ → ┊  │ol3.2┊  ]",
            "[  ┊─┤[ ] todo┊  ]",
            "[  ┊─┤[x] done┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument<'_> = result.unwrap().1;

        // console_log!(md_doc);
        // console_log!(remainder);

        assert_eq2!(remainder, "");
        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );
    }

    #[test]
    fn test_parse_valid_md_no_indent() {
        let input = [
            "start",
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

        let expected_output = [
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊─┤ul2┊  ]",
            "[  ┊1.│ol1┊  ]",
            "[  ┊2.│ol2┊  ]",
            "[  ┊─┤[ ] todo┊  ]",
            "[  ┊─┤[x] done┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument<'_> = result.unwrap().1;

        // console_log!(md_doc);
        // console_log!(remainder);

        assert_eq2!(remainder, "");
        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );
    }
}

#[cfg(test)]
mod tests_parse_markdown {
    use super::*;
    use crate::{BulletKind, CodeBlockLine, CodeBlockLineContent, HeadingData,
                HeadingLevel, HyperlinkData, MdLineFragment, assert_eq2, list};

    #[test]
    fn test_no_line() {
        let input = "Something";
        let (remainder, blocks) = parse_markdown(input).unwrap();
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![MdLineFragment::Plain("Something")])
        );
    }

    #[test]
    fn test_one_line() {
        let input = "Something\n";
        let (remainder, blocks) = parse_markdown(input).unwrap();
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![MdLineFragment::Plain("Something")])
        );
    }

    #[test]
    fn test_parse_markdown_with_invalid_text_in_heading() {
        let input = ["# LINE 1", "", "##% LINE 2 FOO_BAR:", ""].join("\n");
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{remainder:?}");
        println!("\nblocks:\n{blocks:#?}");
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 3);
        assert_eq2!(
            blocks[0],
            MdElement::Heading(HeadingData {
                level: HeadingLevel { level: 1 },
                text: "LINE 1",
            })
        );
        assert_eq2!(
            blocks[1],
            MdElement::Text(list![]), // Empty line.
        );
        assert_eq2!(
            blocks[2],
            MdElement::Text(list![
                MdLineFragment::Plain("##% LINE 2 FOO"),
                MdLineFragment::Plain("_"),
                MdLineFragment::Plain("BAR:"),
            ])
        );
    }

    #[test]
    fn test_parse_markdown_single_line_plain_text() {
        let input = ["_this should not be italic", ""].join("\n");
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{remainder:?}");
        println!("\nblocks:\n{blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 1);
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![
                MdLineFragment::Plain("_"),
                MdLineFragment::Plain("this should not be italic"),
            ])
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

        let (remainder, list_block) = parse_markdown(&input).unwrap();

        let vec_block = &[
            MdElement::Title("Something"),
            MdElement::Tags(list!["tag1", "tag2", "tag3"]),
            MdElement::Heading(HeadingData {
                level: HeadingLevel { level: 1 },
                text: "Foobar",
            }),
            MdElement::Text(list![]), /* Empty line */
            MdElement::Text(list![MdLineFragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            MdElement::Text(list![]), /* Empty line */
            MdElement::CodeBlock(list![
                CodeBlockLine {
                    language: Some("bash"),
                    content: CodeBlockLineContent::StartTag
                },
                CodeBlockLine {
                    language: Some("bash"),
                    content: CodeBlockLineContent::Text("pip install foobar")
                },
                CodeBlockLine {
                    language: Some("bash"),
                    content: CodeBlockLineContent::EndTag
                },
            ]),
            MdElement::CodeBlock(list![
                CodeBlockLine {
                    language: Some("fish"),
                    content: CodeBlockLineContent::StartTag
                },
                CodeBlockLine {
                    language: Some("fish"),
                    content: CodeBlockLineContent::EndTag
                },
            ]),
            MdElement::CodeBlock(list![
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::StartTag
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text("")
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::EndTag
                },
            ]),
            MdElement::Heading(HeadingData {
                level: HeadingLevel { level: 2 },
                text: "Installation",
            }),
            MdElement::Text(list![]), /* Empty line */
            MdElement::Text(list![
                MdLineFragment::Plain("Use the package manager "),
                MdLineFragment::Link(HyperlinkData::from((
                    "pip",
                    "https://pip.pypa.io/en/stable/",
                ))),
                MdLineFragment::Plain(" to install foobar."),
            ]),
            MdElement::CodeBlock(list![
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::StartTag
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text("import foobar")
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text("")
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text(
                        "foobar.pluralize('word') # returns 'words'"
                    )
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text(
                        "foobar.pluralize('goose') # returns 'geese'"
                    )
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::Text(
                        "foobar.singularize('phenomena') # returns 'phenomenon'"
                    )
                },
                CodeBlockLine {
                    language: Some("python"),
                    content: CodeBlockLineContent::EndTag
                },
            ]),
            MdElement::SmartList((
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
            MdElement::SmartList((
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
            MdElement::SmartList((
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
            MdElement::SmartList((
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
            MdElement::SmartList((
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
            MdElement::SmartList((
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
            MdElement::Text(list![MdLineFragment::Plain("end")]),
        ];

        // Print a few of the last items.
        // for block in list_block.iter().skip(list_block.len() - 7) {
        //     println!(
        //         "{0} {1}",
        //         "█ → ".magenta().bold(),
        //         format!("{:?}", block).green()
        //     );
        // }

        assert_eq2!(remainder, "");

        let size_left = list_block.len();
        let size_right = vec_block.len();

        assert_eq2!(size_left, size_right);

        list_block
            .iter()
            .zip(vec_block.iter())
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

        // println!("🍎input: '{}'", input);
        // println!("🍎remainder: {:?}", remainder);
        // println!("🍎blocks: {:#?}", blocks);

        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 7);
    }
}
