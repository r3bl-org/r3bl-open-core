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

use constants::*;
use nom::{branch::alt,
          bytes::complete::{is_not, tag},
          character::complete::{anychar, digit1, space0},
          combinator::{map, opt, recognize, verify},
          multi::{many0, many1},
          sequence::{preceded, terminated, tuple},
          IResult};

use crate::*;

/// Public API for parsing a smart list block in markdown.
pub fn parse_block_smart_list(input: &str) -> IResult<&str, (Lines, BulletKind, usize)> {
    let (remainder, smart_list_ir) = parse_smart_list(input)?;

    let indent = smart_list_ir.indent;
    let bullet_kind = smart_list_ir.bullet_kind;
    let mut output_lines: Lines = List::with_capacity(smart_list_ir.content_lines.len());

    for (index, line) in smart_list_ir.content_lines.iter().enumerate() {
        // Parse the line as a markdown text. Take special care of checkboxes if they show up at the
        // start of the line.
        let (_, fragments_in_line) = {
            let parse_checkbox_policy = {
                let checked = format!("{}{}", CHECKED, SPACE);
                let unchecked = format!("{}{}", UNCHECKED, SPACE);
                if line.content.starts_with(&checked) || line.content.starts_with(&unchecked) {
                    CheckboxParsePolicy::ParseCheckbox
                } else {
                    CheckboxParsePolicy::IgnoreCheckbox
                }
            };
            parse_block_markdown_text_opt_eol_with_checkbox_policy(
                line.content,
                parse_checkbox_policy,
            )?
        };

        // Mark is first line or not (to show or hide bullet).
        let is_first_line = index == 0;

        // Insert bullet marker before the line.
        let mut it = match bullet_kind {
            BulletKind::Ordered(number) => {
                list![MdLineFragment::OrderedListBullet {
                    indent,
                    number,
                    is_first_line
                }]
            }
            BulletKind::Unordered => list![MdLineFragment::UnorderedListBullet {
                indent,
                is_first_line
            }],
        };

        if fragments_in_line.is_empty() {
            // If the line is empty, then we need to insert a blank line.
            it.push(MdLineFragment::Plain(""));
        } else {
            // Otherwise, we can just append the fragments.
            it += fragments_in_line;
        }

        output_lines.push(it);
    }

    Ok((remainder, (output_lines, bullet_kind, indent)))
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_parse_block_smart_list {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_with_unicode() {
        let input = "- straight 😃 foo bar baz\n";
        let result = parse_block_smart_list(input);
        let remainder = result.as_ref().unwrap().0;
        let output = &result.as_ref().unwrap().1;
        assert_eq2!(remainder, "");
        assert_eq2!(
            output,
            &(
                list![list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("straight 😃 foo bar baz"),
                ],],
                BulletKind::Unordered,
                0
            )
        );
    }

    #[test]
    fn test_parse_block_smart_list_with_checkbox() {
        // Valid unchecked.
        {
            let input = vec!["- [ ] todo"].join("\n");
            let result = parse_block_smart_list(&input);
            let remainder = result.as_ref().unwrap().0;
            let (lines, _bullet_kind, _indent) = result.unwrap().1;
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                first_line,
                &list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Checkbox(false),
                    MdLineFragment::Plain(" todo"),
                ]
            );
        }

        // Valid checked.
        {
            let input = vec!["- [x] done"].join("\n");
            let result = parse_block_smart_list(&input);
            let remainder = result.as_ref().unwrap().0;
            let (lines, _bullet_kind, _indent) = result.unwrap().1;
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                first_line,
                &list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Checkbox(true),
                    MdLineFragment::Plain(" done"),
                ]
            );
        }

        // Invalid unchecked.
        {
            let input = vec!["- [ ]todo"].join("\n");
            let result = parse_block_smart_list(&input);
            let remainder = result.as_ref().unwrap().0;
            let (lines, _bullet_kind, _indent) = result.unwrap().1;
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                first_line,
                &list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("[ ]"),
                    MdLineFragment::Plain("todo"),
                ]
            );
        }

        // Invalid checked.
        {
            let input = vec!["- [x]done"].join("\n");
            let result = parse_block_smart_list(&input);
            let remainder = result.as_ref().unwrap().0;
            let (lines, _bullet_kind, _indent) = result.unwrap().1;
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                first_line,
                &list![
                    MdLineFragment::UnorderedListBullet {
                        indent: 0,
                        is_first_line: true
                    },
                    MdLineFragment::Plain("[x]"),
                    MdLineFragment::Plain("done"),
                ]
            );
        }
    }

    #[test]
    fn test_valid_ul_list_1() {
        let input = "- foo\n  bar baz\n";
        let expected = list! {
            list![
                MdLineFragment::UnorderedListBullet { indent: 0, is_first_line: true },
                MdLineFragment::Plain("foo"),
            ],
            list![
                MdLineFragment::UnorderedListBullet { indent: 0, is_first_line: false },
                MdLineFragment::Plain("bar baz"),
            ],
        };
        let result = parse_block_smart_list(input);
        let remainder = result.as_ref().unwrap().0;
        let (lines, _bullet_kind, _indent) = result.unwrap().1;
        assert_eq2!(remainder, "");
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ul_list_2() {
        let input = "- foo\n  bar baz\n- foo1\n  bar1 baz1\n";
        let expected = list! {
            list![
                MdLineFragment::UnorderedListBullet { indent: 0, is_first_line: true },
                MdLineFragment::Plain("foo"),
            ],
            list![
                MdLineFragment::UnorderedListBullet { indent: 0, is_first_line: false },
                MdLineFragment::Plain("bar baz"),
            ],
        };
        let result = parse_block_smart_list(input);
        let remainder = result.as_ref().unwrap().0;
        let (lines, _bullet_kind, _indent) = result.unwrap().1;
        assert_eq2!(remainder, "- foo1\n  bar1 baz1\n");
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ol_list_1() {
        let input = "1. foo\n   bar baz\n";
        let expected = list! {
            list![
                MdLineFragment::OrderedListBullet { indent: 0 , number: 1, is_first_line: true },
                MdLineFragment::Plain("foo"),
            ],
            list![
                MdLineFragment::OrderedListBullet { indent: 0 , number: 1, is_first_line: false },
                MdLineFragment::Plain("bar baz"),
            ],
        };
        let result = parse_block_smart_list(input);
        let remainder = result.as_ref().unwrap().0;
        let (lines, _bullet_kind, _indent) = result.unwrap().1;
        assert_eq2!(remainder, "");
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ol_list_2() {
        let input = "1. foo\n   bar baz\n1. foo\n   bar baz\n";
        let expected = list! {
            list![
                MdLineFragment::OrderedListBullet { indent: 0 , number: 1, is_first_line: true },
                MdLineFragment::Plain("foo"),
            ],
            list![
                MdLineFragment::OrderedListBullet { indent: 0 , number: 1, is_first_line: false },
                MdLineFragment::Plain("bar baz"),
            ],
        };
        let result = parse_block_smart_list(input);
        let remainder = result.as_ref().unwrap().0;
        let (lines, _bullet_kind, _indent) = result.unwrap().1;
        assert_eq2!(remainder, "1. foo\n   bar baz\n");
        assert_eq2!(lines, expected);
    }
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_parse_smart_lists_in_markdown {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_parse_valid_md_ol_with_indent() {
        let input = vec![
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

        let expected_output = vec![
            "start",
            "[  ┊1.│ol1┊  ]",
            "[  ┊  2.│ol2┊ → ┊    │ol2.1┊  ]",
            "[  ┊    3.│ol3┊ → ┊      │ol3.1┊ → ┊      │ol3.2┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument = result.unwrap().1;

        // md_doc.console_log_fg();
        // remainder.console_log_bg();

        assert_eq2!(remainder, "");
        md_doc
            .items
            .iter()
            .zip(expected_output.iter())
            .for_each(|(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            });
    }

    #[test]
    fn test_parse_valid_md_ul_with_indent() {
        let input = vec![
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

        let expected_output = vec![
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊───┤ul2┊ → ┊   │ul2.1┊  ]",
            "[  ┊─────┤ul3┊ → ┊     │ul3.1┊ → ┊     │ul3.2┊  ]",
            "end",
        ];

        let result = parse_markdown(input.as_str());
        let remainder = result.as_ref().unwrap().0;
        let md_doc: MdDocument = result.unwrap().1;

        // md_doc.console_log_fg();
        // remainder.console_log_bg();

        assert_eq2!(remainder, "");
        md_doc
            .items
            .iter()
            .zip(expected_output.iter())
            .for_each(|(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            });
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

        let expected_output = vec![
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
        let md_doc: MdDocument = result.unwrap().1;

        md_doc.console_log_fg();
        remainder.console_log_bg();

        assert_eq2!(remainder, "");
        md_doc
            .items
            .iter()
            .zip(expected_output.iter())
            .for_each(|(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            });
    }

    #[test]
    fn test_parse_valid_md_no_indent() {
        let input = vec![
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

        let expected_output = vec![
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
        let md_doc: MdDocument = result.unwrap().1;

        // md_doc.console_log_fg();
        // remainder.console_log_bg();

        assert_eq2!(remainder, "");
        md_doc
            .items
            .iter()
            .zip(expected_output.iter())
            .for_each(|(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            });
    }
}

/// Holds a single list item for a given indent level. This may contain multiple lines which are
/// stored in the `content_lines` field. Take a look at [parse_smart_list] for more details.
#[derive(Clone, Debug, PartialEq)]
pub struct SmartListIR<'a> {
    /// Spaces before the bullet (for all the lines in this list).
    pub indent: usize,
    /// Unordered or ordered.
    pub bullet_kind: BulletKind,
    /// Does not contain any bullets.
    pub content_lines: Vec<SmartListLine<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SmartListLine<'a> {
    /// Spaces before the bullet (for all the lines in this list).
    pub indent: usize,
    /// Unordered or ordered.
    pub bullet_str: &'a str,
    /// Does not contain any bullets or any spaces for the indent prefix.
    pub content: &'a str,
}

impl<'a> SmartListLine<'a> {
    pub fn new(indent: usize, bullet_str: &'a str, content: &'a str) -> Self {
        Self {
            indent,
            bullet_str,
            content,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum BulletKind {
    Ordered(usize),
    Unordered,
}

/// First line of `input` looks like this.
///
/// ```text
/// ╭─ Unordered ──────────────────┬───── Ordered ───────────────────╮
/// │"    - foobar"                │"    100. foobar"                │
/// │ ░░░░▓▓░░░░░░                 │ ░░░░▓▓▓▓▓░░░░░░                 │
/// │ ┬──┬┬┬┬────┬                 │ ┬──┬┬───┬┬────┬                 │
/// │ ╰──╯╰╯╰────╯                 │ ╰──╯╰───╯╰────╯                 │
/// │  │  │  └─→ first line content│  │   │    └─→ first line content│
/// │  │  └→ bullet.len(): 2       │  │   └→ bullet.len(): 4         │
/// │  └→ indent: 4                │  └→ indent: 4                   │
/// ╰──────────────────────────────┴─────────────────────────────────╯
/// ```
///
/// Rest of the lines of `input` look like this.
///
/// ```text
/// ╭─ Unordered ──────────────────┬───── Ordered ───────────────────╮
/// │"      foobar"                │"         foobar"                │
/// │ ░░░░▓▓░░░░░░                 │ ░░░░▓▓▓▓▓░░░░░░                 │
/// │ ┬──┬┬┬┬────┬                 │ ┬──┬┬───┬┬────┬                 │
/// │ ╰──╯╰╯╰────╯                 │ ╰──╯╰───╯╰────╯                 │
/// │  │  │  └─→ first line content│  │   │    └─→ first line content│
/// │  │  └→ bullet.len(): 2       │  │   └→ bullet.len(): 4         │
/// │  └→ indent: 4                │  └→ indent: 4                   │
/// ╰──────────────────────────────┴─────────────────────────────────╯
/// ```
#[rustfmt::skip]
pub fn parse_smart_list(
    input: &str
) -> IResult</* remainder */ &str, SmartListIR> {
    // Match empty spaces & count them into indent.
    let (input, indent) = map(
        space0,
        |it: &str| it.len()
    )(input)?;

    // Indent has to be multiple of the base width, otherwise it's not a list item.
    if indent % LIST_PREFIX_BASE_WIDTH != 0 {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Indent must be a multiple of LIST_PREFIX_INDENT_SIZE",
            nom::error::ErrorKind::Fail,
        )));
    }

    // Match the bullet: Ordered => "123. " or Unordered => "- ".
    let (input, bullet) =
        recognize(
            alt((
                tag(UNORDERED_LIST_PREFIX),
                terminated(digit1, tag(ORDERED_LIST_PARTIAL_PREFIX)),
            ))
        )(input)?;

    // Decide which kind of list item this is based on `bullet`.
    let bullet_kind = if bullet.starts_with(|c: char| c.is_ascii_digit()) {
        let number_str = bullet.trim_end_matches(ORDERED_LIST_PARTIAL_PREFIX);
        let number_usize = number_str.parse::<usize>().or(
            Err(nom::Err::Error(nom::error::Error::new(
                "Ordered list number must be usize",
                nom::error::ErrorKind::Fail,
            )))
        )?;
        BulletKind::Ordered(number_usize)
    } else {
        BulletKind::Unordered
    };

    // Match the rest of the line & other lines that have the same indent.
    let (input, content_lines) = parse_smart_list_content_lines(input, indent, bullet)?;

    // Return the result.
    Ok((
        input,
        SmartListIR { indent, bullet_kind, content_lines },
    ))
}

mod verify_rest {
    use super::*;

    /// Return true if:
    /// - No ul items (at any indent).
    /// - No other ol items with same indent + number.
    /// - No other ol items with any indent or number.
    pub fn list_contents_does_not_start_with_list_prefix(it: &str) -> bool {
        let result: IResult<&str, &str> = recognize(alt((
            tag(UNORDERED_LIST_PREFIX),
            terminated(digit1, tag(ORDERED_LIST_PARTIAL_PREFIX)),
        )))(it.trim_start());
        let starts_with_list_prefix = result.is_ok();
        !starts_with_list_prefix
    }

    fn count_whitespace_at_start(it: &str) -> usize {
        let mut count: usize = 0;
        for c in it.chars() {
            if c == SPACE_CHAR {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    /// - Eg: "  - ul2", indent: 0, my_bullet_str_len: 2 => true
    /// - Eg: "  ul2.1", indent: 2, my_bullet_str_len: 4 => true
    /// - Eg: "  u13.1", indent: 4, my_bullet_str_len: 6 => true
    pub fn must_start_with_correct_num_of_spaces(it: &str, my_bullet_str: &str) -> bool {
        let it_spaces_at_start = count_whitespace_at_start(it);
        it_spaces_at_start == my_bullet_str.len()
    }
}

#[rustfmt::skip]
pub fn parse_smart_list_content_lines<'a>(
    input: &'a str,
    indent: usize,
    bullet: &'a str,
) -> IResult</* remainder */ &'a str, /* lines */ Vec<SmartListLine<'a>>> {
    let indent_padding = SPACE.repeat(indent);
    let indent_padding = indent_padding.as_str();

    match input.find(NEW_LINE) {
        // Keep the first line. There may be more than 1 line.
        Some(first_line_end) => {
            let first = &input[..first_line_end];
            let input = &input[first_line_end+1..];

            // Match the rest of the lines.
            let (remainder, rest) = many0(
                tuple((
                    verify(
                        // FIRST STEP: Match the ul or ol list item line.
                        preceded(
                            // Match the indent.
                            tag(indent_padding),
                            // Match the rest of the line.
                            /* output */ alt((
                                is_not(NEW_LINE),
                                recognize(many1(anychar)),
                            )),
                        ),
                        // SECOND STEP: Verify it to make sure no ul or ol list prefix.
                        |it: &str| {
                            // `it` must not *just* have spaces.
                            if it.trim_start().is_empty() {
                                return false;
                            }

                            // `it` must start w/ *exactly* the correct number of spaces.
                            if !verify_rest::must_start_with_correct_num_of_spaces(it, bullet) {
                                return false;
                            }

                            // `it` must not start w/ the ul list prefix.
                            // `it` must not start w/ the ol list prefix.
                            verify_rest::list_contents_does_not_start_with_list_prefix(it)
                        }
                    ),
                    opt(tag(NEW_LINE)),
                ))
            )(input)?;

            // Convert `rest` into a Vec<&str> that contains the output lines.
            let output_lines: Vec<SmartListLine> = {
                let mut it = Vec::with_capacity(rest.len() + 1);

                it.push(SmartListLine {
                    indent,
                    bullet_str: bullet,
                    content: first
                });

                it.extend(rest.iter().map(
                    // Skip "bullet's width" number of spaces at the start of the line.
                    |(rest_line_content, _)|
                    SmartListLine {
                        indent,
                        bullet_str: bullet,
                        content: &rest_line_content[bullet.len()..]
                    })
                );

                it
            };

            Ok((remainder, output_lines))
        }
        None => {
            // Keep the first line. There are no more lines.
            Ok(("", vec![SmartListLine {
                indent,
                bullet_str: bullet,
                content: input
            }]))
        }
    }
}

#[cfg(test)]
mod tests_parse_list_item {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_invalid_ul_list() {
        let input = "  -";
        let actual = parse_smart_list(input);
        assert_eq2!(actual.is_err(), true);
    }

    #[test]
    fn test_valid_ul_list() {
        // 1 item.
        {
            let input = "- foo";
            let actual = parse_smart_list(input);
            assert_eq2!(
                actual,
                Ok((
                    "",
                    SmartListIR {
                        indent: 0,
                        bullet_kind: BulletKind::Unordered,
                        content_lines: vec![SmartListLine::new(0, "- ", "foo")],
                    }
                ))
            );
        }

        // 2 items.
        {
            let input = "- foo\n  bar";
            let actual = parse_smart_list(input);
            assert_eq2!(
                actual,
                Ok((
                    "",
                    SmartListIR {
                        indent: 0,
                        bullet_kind: BulletKind::Unordered,
                        content_lines: vec![
                            SmartListLine::new(0, "- ", "foo"),
                            SmartListLine::new(0, "- ", "bar"),
                        ],
                    }
                ))
            );
        }
    }

    #[test]
    fn test_invalid_ol_list() {
        let input = "  1.";
        let actual = parse_smart_list(input);
        assert_eq2!(actual.is_err(), true);
    }

    #[test]
    fn test_valid_ol_list() {
        // 1 item.
        {
            let input = "1. foo";
            let actual = parse_smart_list(input);
            assert_eq2!(
                actual,
                Ok((
                    "",
                    SmartListIR {
                        indent: 0,
                        bullet_kind: BulletKind::Ordered(1),
                        content_lines: vec![SmartListLine::new(0, "1. ", "foo")],
                    }
                ))
            );
        }

        // 2 items.
        {
            let input = "1. foo\n   bar";
            let result = parse_smart_list(input);
            assert_eq2!(
                result,
                Ok((
                    "",
                    SmartListIR {
                        indent: 0,
                        bullet_kind: BulletKind::Ordered(1),
                        content_lines: vec![
                            SmartListLine::new(0, "1. ", "foo"),
                            SmartListLine::new(0, "1. ", "bar"),
                        ],
                    }
                ))
            );
        }
    }
}

#[cfg(test)]
mod tests_list_item_lines {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    /// One line: "- foo".
    #[test]
    fn test_one_line() {
        let input = "- foo";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![SmartListLine::new(0, "- ", "foo")],
            }
        );
    }

    /// One line (with trailing newline): "- foo\n".
    #[test]
    fn test_one_line_trailing_newline() {
        let input = "- foo\n";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![SmartListLine::new(0, "- ", "foo")]
            }
        );
    }

    /// 2 lines (last line is empty): "- foo\n\n".
    #[test]
    fn test_two_lines_last_is_empty() {
        let input = "- foo\n\n";
        let actual = parse_smart_list(input);
        let (remainder, actual) = actual.unwrap();
        assert_eq2!(remainder, "\n");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![SmartListLine::new(0, "- ", "foo")]
            }
        );
    }

    /// 2 lines: "- foo\n  bar baz".
    #[test]
    fn test_two_lines() {
        let input = "- foo\n  bar baz";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![
                    SmartListLine::new(0, "- ", "foo"),
                    SmartListLine::new(0, "- ", "bar baz"),
                ]
            }
        );
    }

    /// 3 lines (last line is empty): "- foo\n  bar baz\n".
    #[test]
    fn test_three_lines_last_is_empty() {
        let input = "- foo\n  bar baz\n";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![
                    SmartListLine::new(0, "- ", "foo"),
                    SmartListLine::new(0, "- ", "bar baz"),
                ]
            }
        );
    }

    /// 3 lines: "- foo\n  bar baz\n  qux".
    #[test]
    fn test_three_lines() {
        let input = "- foo\n  bar baz\n  qux";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: vec![
                    SmartListLine::new(0, "- ", "foo"),
                    SmartListLine::new(0, "- ", "bar baz"),
                    SmartListLine::new(0, "- ", "qux"),
                ]
            }
        );
    }
}

#[cfg(test)]
mod tests_bullet_kinds {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_bullet_kinds() {
        // Unordered.
        {
            let input = "- foo";
            let (_remainder, actual) = parse_smart_list(input).unwrap();
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }

        // Ordered.
        {
            let input = "1. foo";
            let (_remainder, actual) = parse_smart_list(input).unwrap();
            assert_eq2!(actual.bullet_kind, BulletKind::Ordered(1));
        }
    }
}

#[cfg(test)]
mod tests_parse_indents {
    use r3bl_rs_utils_core::assert_eq2;

    use super::*;

    #[test]
    fn test_indent() {
        // Indent = 0 Ok.
        {
            let input = "- foo";
            let (_remainder, actual) = parse_smart_list(input).unwrap();
            assert_eq2!(actual.indent, 0);
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }

        // Indent = 1 Fail.
        {
            let input = " - foo";
            let result = parse_smart_list(input);
            assert_eq2!(
                result.err(),
                Some(nom::Err::Error(nom::error::Error::new(
                    "Indent must be a multiple of LIST_PREFIX_INDENT_SIZE",
                    nom::error::ErrorKind::Fail,
                )))
            );
        }

        // Indent = 2 Ok.
        {
            let input = "  - foo";
            let (_remainder, actual) = parse_smart_list(input).unwrap();
            assert_eq2!(actual.indent, 2);
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }
    }
}
