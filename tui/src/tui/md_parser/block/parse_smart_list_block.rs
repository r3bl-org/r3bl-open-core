// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use nom::{IResult, Parser,
          branch::alt,
          bytes::complete::{is_not, tag, take_while},
          character::complete::{anychar, digit1, space0},
          combinator::{map, opt, recognize, verify},
          multi::{many0, many1},
          sequence::{preceded, terminated}};
use smallvec::smallvec;

use crate::{BulletKind, CheckboxParsePolicy, InlineVec, Lines, List, MdLineFragment,
            SmartListIRStr, SmartListLine, SmartListLineStr, get_spaces, list,
            md_parser::constants::{CHECKED, LIST_PREFIX_BASE_WIDTH, NEW_LINE,
                                   NEWLINE_OR_NULL, NULL_CHAR,
                                   ORDERED_LIST_PARTIAL_PREFIX, SPACE, SPACE_CHAR,
                                   UNCHECKED, UNORDERED_LIST_PREFIX},
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line,
            parse_null_padded_line::is,
            tiny_inline_string};

/// Public API for parsing a smart list block in markdown.
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The parser uses
/// `NEWLINE_OR_NULL` constant and handles null padding in list item content parsing.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain a valid smart list block.
pub fn parse_smart_list_block(
    input: &str,
) -> IResult<&str, (Lines<'_>, BulletKind, usize)> {
    use parse_block_smart_list_helper::{build_line_fragments, create_bullet_fragment,
                                        determine_checkbox_policy};

    let (remainder, smart_list_ir) = parse_smart_list(input)?;

    let indent = smart_list_ir.indent;
    let bullet_kind = smart_list_ir.bullet_kind;
    let mut output_lines: Lines<'_> =
        List::with_capacity(smart_list_ir.content_lines.len());

    for (index, line) in smart_list_ir.content_lines.iter().enumerate() {
        // Parse the line as markdown text with checkbox handling.
        let line_content = line.content;
        let parse_checkbox_policy = determine_checkbox_policy(line_content);
        let (_, fragments_in_line) =
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
                line_content,
                parse_checkbox_policy,
            )?;

        // Mark if this is the first line (to show or hide bullet).
        let is_first_line = index == 0;

        // Create bullet fragment and build complete line.
        let bullet_fragments = create_bullet_fragment(bullet_kind, indent, is_first_line);
        let complete_line = build_line_fragments(bullet_fragments, fragments_in_line);

        output_lines.push(complete_line);
    }

    Ok((remainder, (output_lines, bullet_kind, indent)))
}

mod parse_block_smart_list_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    // Helper function to determine checkbox parsing policy.
    pub fn determine_checkbox_policy(content: &str) -> CheckboxParsePolicy {
        let checked = tiny_inline_string!("{}{}", CHECKED, SPACE);
        let unchecked = tiny_inline_string!("{}{}", UNCHECKED, SPACE);

        if content.starts_with(checked.as_str())
            || content.starts_with(unchecked.as_str())
        {
            CheckboxParsePolicy::ParseCheckbox
        } else {
            CheckboxParsePolicy::IgnoreCheckbox
        }
    }

    // Helper function to create bullet fragment.
    pub fn create_bullet_fragment<'a>(
        bullet_kind: BulletKind,
        indent: usize,
        is_first_line: bool,
    ) -> List<MdLineFragment<'a>> {
        match bullet_kind {
            BulletKind::Ordered(number) => {
                list![MdLineFragment::OrderedListBullet {
                    indent,
                    number,
                    is_first_line
                }]
            }
            BulletKind::Unordered => {
                list![MdLineFragment::UnorderedListBullet {
                    indent,
                    is_first_line
                }]
            }
        }
    }

    // Helper function to build complete line fragments.
    pub fn build_line_fragments<'a>(
        mut bullet_fragments: List<MdLineFragment<'a>>,
        content_fragments: List<MdLineFragment<'a>>,
    ) -> List<MdLineFragment<'a>> {
        if content_fragments.is_empty() {
            // If the line is empty, then we need to insert a blank line.
            bullet_fragments.push(MdLineFragment::Plain(""));
        } else {
            // Otherwise, we can just append the fragments.
            bullet_fragments += content_fragments;
        }

        bullet_fragments
    }
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_parse_block_smart_list {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_with_unicode() {
        let input = "- straight 😃 foo bar baz\n";
        let result = parse_smart_list_block(input);
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
            let input = ["- [ ] todo"].join("\n");
            let result = parse_smart_list_block(&input);
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
            let input = ["- [x] done"].join("\n");
            let result = parse_smart_list_block(&input);
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
            let input = ["- [ ]todo"].join("\n");
            let result = parse_smart_list_block(&input);
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
            let input = ["- [x]done"].join("\n");
            let result = parse_smart_list_block(&input);
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
        let result = parse_smart_list_block(input);
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
        let result = parse_smart_list_block(input);
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
        let result = parse_smart_list_block(input);
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
        let result = parse_smart_list_block(input);
        let remainder = result.as_ref().unwrap().0;
        let (lines, _bullet_kind, _indent) = result.unwrap().1;
        assert_eq2!(remainder, "1. foo\n   bar baz\n");
        assert_eq2!(lines, expected);
    }
}

/// First line of `input` looks like this.
///
/// ```text
/// ╭─ Unordered ────────────────┬───── Ordered ────────────────╮
/// │"    - foobar"              │"    100. foobar"             │
/// │ ░░░░▓▓░░░░░░               │ ░░░░▓▓▓▓▓░░░░░░              │
/// │ ┬──┬┬┬┬────┬               │ ┬──┬┬───┬┬────┬              │
/// │ ╰──╯╰╯╰────╯               │ ╰──╯╰───╯╰────╯              │
/// │  │  │  ⎩first line content │  │   │    ⎩first line content│
/// │  │  ⎩bullet.len():  2      │  │   ⎩bullet.len(): 4        │
/// │  ⎩indent: 4                │  ⎩indent: 4                  │
/// ╰────────────────────────────┴──────────────────────────────╯
/// ```
///
/// Rest of the lines of `input` look like this.
///
/// ```text
/// ╭─ Unordered ────────────────┬───── Ordered ────────────────╮
/// │"      foobar"              │"         foobar"             │
/// │ ░░░░▓▓░░░░░░               │ ░░░░▓▓▓▓▓░░░░░░              │
/// │ ┬──┬┬┬┬────┬               │ ┬──┬┬───┬┬────┬              │
/// │ ╰──╯╰╯╰────╯               │ ╰──╯╰───╯╰────╯              │
/// │  │  │  ⎩first line content │  │   │    ⎩first line content│
/// │  │  ⎩bullet.len(): 2       │  │   ⎩bullet.len(): 4        │
/// │  ⎩indent: 4                │  ⎩indent: 4                  │
/// ╰────────────────────────────┴──────────────────────────────╯
/// ```
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain a valid smart list.
#[rustfmt::skip]
pub fn parse_smart_list(
    input: &str
) -> IResult</* remainder */ &str, SmartListIRStr<'_>> {
    // Match empty spaces & count them into indent.
    let (input, indent) = map(
        space0,
        |it: &str| it.len()
    ).parse(input)?;

    // Indent has to be multiple of the base width, otherwise it's not a list item.
    if !indent.is_multiple_of(LIST_PREFIX_BASE_WIDTH) {
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
        ).parse(input)?;

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
        SmartListIRStr { indent, bullet_kind, content_lines },
    ))
}

#[cfg(test)]
mod tests_bullet_kinds {
    use super::*;
    use crate::assert_eq2;

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
mod tests_parse_smart_list {
    use super::*;
    use crate::{SmartListIR, assert_eq2};

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
                    SmartListIRStr {
                        indent: 0,
                        bullet_kind: BulletKind::Unordered,
                        content_lines: smallvec![SmartListLineStr::new(0, "- ", "foo")],
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
                    SmartListIRStr {
                        indent: 0,
                        bullet_kind: BulletKind::Unordered,
                        content_lines: smallvec![
                            SmartListLineStr::new(0, "- ", "foo"),
                            SmartListLineStr::new(0, "- ", "bar"),
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
                        content_lines: smallvec![SmartListLine::new(0, "1. ", "foo")],
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
                        content_lines: smallvec![
                            SmartListLine::new(0, "1. ", "foo"),
                            SmartListLine::new(0, "1. ", "bar"),
                        ],
                    }
                ))
            );
        }
    }

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
                content_lines: smallvec![SmartListLine::new(0, "- ", "foo")],
            }
        );
    }

    /// One line with trailing [`NEW_LINE`]: "- foo\n".
    #[test]
    fn test_one_line_trailing_new_line() {
        let input = "- foo\n";
        let (remainder, actual) = parse_smart_list(input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            actual,
            SmartListIR {
                indent: 0,
                bullet_kind: BulletKind::Unordered,
                content_lines: smallvec![SmartListLine::new(0, "- ", "foo")]
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
                content_lines: smallvec![SmartListLine::new(0, "- ", "foo")]
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
                content_lines: smallvec![
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
                content_lines: smallvec![
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
                content_lines: smallvec![
                    SmartListLine::new(0, "- ", "foo"),
                    SmartListLine::new(0, "- ", "bar baz"),
                    SmartListLine::new(0, "- ", "qux"),
                ]
            }
        );
    }

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

/// Represents the structure of smart list input for parsing.
enum InputStructure<'a> {
    /// Single line with no newline (e.g., "- foo")
    SingleLine(&'a str),
    /// Multi-line with first line and remainder (e.g., "- foo\n  bar")
    MultiLine {
        first_line_just_before_crlf: &'a str,
        remainder_after_crlf: &'a str,
    },
}

/// Parses smart list content lines for both single-line and multi-line smart lists.
///
/// This function handles two cases:
/// 1. **Single-line lists**: No newline found (e.g., "- foo") - returns immediately with
///    one line
/// 2. **Multi-line lists**: Newline found (e.g., "- foo\n  bar") - parses continuation
///    lines
///
/// For multi-line lists, continuation lines must:
/// - Have the correct indentation (indent + `bullet.len()` spaces)
/// - Not start with list prefixes ("- " or "1. " etc.)
/// - Not be empty (only whitespace)
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain valid smart list content
/// lines.
pub fn parse_smart_list_content_lines<'a>(
    input: &'a str,
    indent: usize,
    bullet: &'a str,
) -> IResult</* remainder */ &'a str, /* lines */ InlineVec<SmartListLineStr<'a>>> {
    let indent_padding = get_spaces(indent);

    // Analyze input structure to determine if single-line or multi-line.
    let input_structure = match input.find(NEW_LINE) {
        None => InputStructure::SingleLine(input),
        Some(new_line_index) => InputStructure::MultiLine {
            first_line_just_before_crlf: &input[..new_line_index],
            remainder_after_crlf: &input[new_line_index..],
        },
    };

    match input_structure {
        InputStructure::SingleLine(content) => {
            // SINGLE-LINE CASE: No newline found, return immediately.
            // Examples: "- foo", "1. bar" (no continuation lines).
            Ok((
                "",
                smallvec![SmartListLine {
                    indent,
                    bullet_str: bullet,
                    content
                }],
            ))
        }
        InputStructure::MultiLine {
            first_line_just_before_crlf,
            remainder_after_crlf,
        } => {
            // MULTI-LINE CASE: Found a newline, parse continuation lines.
            parse_multi_line_content(
                first_line_just_before_crlf,
                remainder_after_crlf,
                indent,
                bullet,
                &indent_padding,
            )
        }
    }
}

/// Handles parsing of multi-line smart list content.
fn parse_multi_line_content<'a>(
    first_line_just_before_crlf: &'a str,
    remainder_after_crlf: &'a str,
    indent: usize,
    bullet: &'a str,
    indent_padding: &str,
) -> IResult<&'a str, InlineVec<SmartListLineStr<'a>>> {
    // Skip the newline and any null padding that follow to prepare for parsing
    // continuation lines.
    let (remaining_input_after_first_line_no_ws, _discarded) = (
        tag(NEW_LINE),
        /* zero or more */ take_while(is(NULL_CHAR)),
    )
        .parse(remainder_after_crlf)?;

    // Parse any continuation lines that belong to this list item.
    // Uses many0() so it gracefully handles cases with no continuation lines (empty
    // match).
    let (remainder, rest) = many0((
        verify(
            // FIRST STEP: Match the ul or ol list item line.
            preceded(
                // Match the indent.
                tag(indent_padding),
                // Match the rest of the line.
                /* output */
                alt((is_not(NEWLINE_OR_NULL), recognize(many1(anychar)))),
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
            },
        ),
        opt((tag(NEW_LINE), take_while(is(NULL_CHAR)))),
    ))
    .parse(remaining_input_after_first_line_no_ws)?;

    // Build the final output: first line + any continuation lines found.
    let output_lines: InlineVec<SmartListLineStr<'_>> = {
        let mut it = InlineVec::with_capacity(rest.len() + 1);

        // Always include the first line
        it.push(SmartListLineStr {
            indent,
            bullet_str: bullet,
            content: first_line_just_before_crlf,
        });

        // Add any continuation lines
        it.extend(rest.iter().map(
            // Skip "bullet's width" number of spaces at the start of the line.
            |(rest_line_content, _)| SmartListLineStr {
                indent,
                bullet_str: bullet,
                content: &rest_line_content[bullet.len()..],
            },
        ));

        it
    };

    Ok((remainder, output_lines))
}

#[cfg(test)]
mod tests_parse_smart_list_content_lines {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_single_line_no_newline() {
        let input = "foo bar";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "foo bar"));
    }

    #[test]
    fn test_single_line_with_newline() {
        let input = "foo bar\n";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "foo bar"));
    }

    #[test]
    fn test_multiple_lines_unordered() {
        let input = "first line\n  second line\n  third line";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "- ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(0, "- ", "third line"));
    }

    #[test]
    fn test_multiple_lines_ordered() {
        let input = "first line\n   second line\n   third line";
        let indent = 0;
        let bullet = "1. ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "1. ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "1. ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(0, "1. ", "third line"));
    }

    #[test]
    fn test_with_indent_unordered() {
        let input = "first line\n    second line\n    third line";
        let indent = 2;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(2, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(2, "- ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(2, "- ", "third line"));
    }

    #[test]
    fn test_with_indent_ordered() {
        let input = "first line\n     second line\n     third line";
        let indent = 2;
        let bullet = "1. ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(2, "1. ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(2, "1. ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(2, "1. ", "third line"));
    }

    #[test]
    fn test_stops_at_new_list_item_unordered() {
        let input = "first line\n  second line\n- new item\n  its content";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "- new item\n  its content");
        assert_eq2!(lines.len(), 2);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "- ", "second line"));
    }

    #[test]
    fn test_stops_at_new_list_item_ordered() {
        let input = "first line\n   second line\n2. new item\n   its content";
        let indent = 0;
        let bullet = "1. ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "2. new item\n   its content");
        assert_eq2!(lines.len(), 2);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "1. ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "1. ", "second line"));
    }

    #[test]
    fn test_stops_at_different_indent_list() {
        let input = "first line\n  second line\n  - nested item";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "  - nested item");
        assert_eq2!(lines.len(), 2);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "- ", "second line"));
    }

    #[test]
    fn test_with_trailing_newlines() {
        let input = "first line\n  second line\n\nother content";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "\nother content");
        assert_eq2!(lines.len(), 2);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "- ", "second line"));
    }

    #[test]
    fn test_empty_continuation_lines() {
        let input = "first line\n  \n  third line";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "  \n  third line");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
    }

    #[test]
    fn test_insufficient_indent() {
        let input = "first line\n second line"; // Only 1 space instead of 2
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, " second line");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
    }

    #[test]
    fn test_too_much_indent() {
        let input = "first line\n   second line"; // 3 spaces instead of 2
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "   second line");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
    }

    #[test]
    fn test_double_digit_ordered_list() {
        let input = "first line\n    second line\n    third line";
        let indent = 0;
        let bullet = "10. ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "10. ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "10. ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(0, "10. ", "third line"));
    }

    #[test]
    fn test_triple_digit_ordered_list() {
        let input = "first line\n     second line\n     third line";
        let indent = 0;
        let bullet = "100. ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "100. ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "100. ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(0, "100. ", "third line"));
    }

    #[test]
    fn test_unicode_content() {
        let input = "😃 unicode\n  more 🎉 unicode\n  final 🚀 line";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "😃 unicode"));
        assert_eq2!(lines[1], SmartListLineStr::new(0, "- ", "more 🎉 unicode"));
        assert_eq2!(lines[2], SmartListLineStr::new(0, "- ", "final 🚀 line"));
    }

    #[test]
    fn test_mixed_list_types_in_content() {
        // Content that looks like list items but shouldn't be parsed as such
        let input = "first line\n  - not a list item\n  1. also not a list item";
        let indent = 0;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "  - not a list item\n  1. also not a list item");
        assert_eq2!(lines.len(), 1);
        assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
    }

    #[test]
    fn test_complex_indented_scenario() {
        let input = "first line\n      second line\n      third line\n    - nested list";
        let indent = 4;
        let bullet = "- ";

        let (remainder, lines) =
            parse_smart_list_content_lines(input, indent, bullet).unwrap();

        assert_eq2!(remainder, "    - nested list");
        assert_eq2!(lines.len(), 3);
        assert_eq2!(lines[0], SmartListLineStr::new(4, "- ", "first line"));
        assert_eq2!(lines[1], SmartListLineStr::new(4, "- ", "second line"));
        assert_eq2!(lines[2], SmartListLineStr::new(4, "- ", "third line"));
    }

    #[test]
    fn test_parse_smart_list_with_null_padding() {
        use crate::assert_eq2;

        // Simple test with null padding right after list
        {
            let input = "- item\n\0\0\0rest";
            let (remainder, smart_list_ir) = parse_smart_list(input).unwrap();
            assert_eq2!(remainder, "rest");
            assert_eq2!(smart_list_ir.content_lines.len(), 1);
            assert_eq2!(smart_list_ir.bullet_kind, BulletKind::Unordered);
        }

        // Test content line parsing with null padding
        {
            let input = "first line\n\0\0\0";
            let indent = 0;
            let bullet = "- ";
            let (remainder, lines) =
                parse_smart_list_content_lines(input, indent, bullet).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(lines.len(), 1);
            assert_eq2!(lines[0], SmartListLineStr::new(0, "- ", "first line"));
        }
    }
}

mod verify_rest {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Return true if:
    /// - No ul items (at any indent).
    /// - No other ol items with same indent + number.
    /// - No other ol items with any indent or number.
    pub fn list_contents_does_not_start_with_list_prefix(it: &str) -> bool {
        let result: IResult<&str, &str> = recognize(alt((
            tag(UNORDERED_LIST_PREFIX),
            terminated(digit1, tag(ORDERED_LIST_PARTIAL_PREFIX)),
        )))
        .parse(it.trim_start());
        let starts_with_list_prefix = result.is_ok();
        !starts_with_list_prefix
    }

    fn count_spaces_at_start(it: &str) -> usize {
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

    /// - Eg: "  - ul2", indent: 0, `my_bullet_str_len`: 2 => true
    /// - Eg: "  ul2.1", indent: 2, `my_bullet_str_len`: 4 => true
    /// - Eg: "  u13.1", indent: 4, `my_bullet_str_len`: 6 => true
    pub fn must_start_with_correct_num_of_spaces(it: &str, my_bullet_str: &str) -> bool {
        let it_spaces_at_start = count_spaces_at_start(it);
        it_spaces_at_start == my_bullet_str.len()
    }
}
