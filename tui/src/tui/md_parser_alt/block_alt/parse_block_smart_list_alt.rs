/*
 *   Copyright (c) 2025 R3BL LLC
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

//! ⚠️  IMPORTANT: CHARACTER-BASED INDEXING THROUGHOUT
//!
//! This module uses AsStrSlice which operates on CHARACTER-BASED indexing for
//! proper Unicode/UTF-8 support. All functions in this file follow these principles:
//!
//! 1. Use [AsStrSlice] methods (`take_from`, `extract_to_line_end`) for slicing.
//! 2. Convert byte positions from nom's [FindSubstring] to character positions.
//! 3. Never use raw slice operators (&str[byte_start..byte_end]) on UTF-8 text.
//! 4. Count characters with `.chars().count()`, not bytes with `.len()`, or just use the
//!    [CharLengthExt] which adds a safe `len_chars()` method on `&str`.
//!
//! This ensures proper handling of emojis and multi-byte UTF-8 characters.
//! See the main function documentation for detailed examples and warnings.

use nom::{branch::alt,
          bytes::complete::{is_not, tag},
          character::complete::{anychar, digit1},
          combinator::{opt, recognize, verify},
          multi::many0,
          sequence::{preceded, terminated},
          FindSubstring,
          IResult,
          Input,
          Parser};
use smallvec::smallvec;

use crate::{list,
            md_parser::constants::{CHECKED,
                                   LIST_PREFIX_BASE_WIDTH,
                                   NEW_LINE,
                                   ORDERED_LIST_PARTIAL_PREFIX,
                                   SPACE,
                                   SPACE_CHAR,
                                   UNCHECKED,
                                   UNORDERED_LIST_PREFIX},
            pad_fmt,
            parse_inline_fragments_until_eol_or_eoi_alt,
            tiny_inline_string,
            AsStrSlice,
            BulletKind,
            CharLengthExt,
            CheckboxParsePolicy,
            InlineString,
            InlineVec,
            Lines,
            List,
            MdLineFragment,
            MdLineFragments,
            NomErr,
            NomError,
            NomErrorKind,
            SmartListIRAlt,
            SmartListLineAlt};

/// Parse a complete smart list block with automatic multi-line advancement.
///
/// ## Purpose
/// This is the **primary public API** for parsing smart lists in the R3BL TUI markdown
/// parser. A "smart list" is a multi-line list structure that can be ordered (numbered),
/// unordered (bulleted), or checkbox-based, with support for nested indentation and
/// multi-line content.
///
/// ## Smart List Types Supported
/// - **Unordered lists**: `- item`, `* item`
/// - **Ordered lists**: `1. item`, `42. item`
/// - **Checkbox lists**: `- [ ] todo`, `- [x] done`
/// - **Nested lists**: With proper indentation handling
///
/// ## Input Format
/// Expects input positioned at the start of a smart list block. The parser will
/// automatically detect the list type and consume all consecutive lines that belong
/// to the list structure, including:
/// - Initial list item line
/// - Continuation lines with proper indentation
/// - Nested list items
/// - Multi-line content within list items
///
/// ## Line Advancement
/// This is a **multi-line block parser that auto-advances**. It consumes all lines
/// that belong to the list structure and positions the remainder at the first line
/// after the list block ends.
///
/// ## Return Value
/// Returns a tuple containing:
/// - `Lines<'a>`: Parsed and formatted list lines ready for rendering
/// - `BulletKind`: The type of list bullets (Ordered(number) or Unordered)
/// - `usize`: The base indentation level of the list
///
/// ## Examples
/// ```text
/// "- item1\n  continuation\n- item2\n"
/// → (formatted_lines, BulletKind::Unordered, 0)
///
/// "  1. nested\n     content\n  2. more\n"
/// → (formatted_lines, BulletKind::Ordered(1), 2)
/// ```
///
/// ## Error Handling
/// Returns `Err` if:
/// - Input doesn't start with a valid list marker
/// - List structure is malformed
/// - Internal parsing of list content fails
pub fn parse_block_smart_list_auto_advance_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, (Lines<'a>, BulletKind, usize)> {
    let (remainder, smart_list_ir) = parse_smart_list_alt(input)?;

    let indent = smart_list_ir.indent;
    let bullet_kind = smart_list_ir.bullet_kind;
    let mut output_lines: Lines<'a> =
        List::with_capacity(smart_list_ir.content_lines.len());

    for (index, line) in smart_list_ir.content_lines.iter().enumerate() {
        // Parse the line as markdown text with checkbox handling.
        let parse_checkbox_policy = determine_checkbox_policy(line.content.clone());
        let (_, fragments_in_line) =
            parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                line.content.clone(),
                parse_checkbox_policy,
            )?;

        // Mark if this is the first line (to show or hide bullet).
        let is_first_line = index == 0;

        // Create bullet fragment and build complete line.
        let bullet_fragments = create_bullet_fragment(bullet_kind, indent, is_first_line);
        let complete_line = build_line_fragments(bullet_fragments, fragments_in_line);

        output_lines.push(complete_line);
    }

    return Ok((remainder, (output_lines, bullet_kind, indent)));

    /// Helper function to determine checkbox parsing policy.
    fn determine_checkbox_policy(content: AsStrSlice<'_>) -> CheckboxParsePolicy {
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

    /// Helper function to create bullet fragment.
    fn create_bullet_fragment<'a>(
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

    /// Helper function to build complete line fragments.
    fn build_line_fragments<'a>(
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
mod tests_parse_block_smart_list_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_with_unicode() {
        as_str_slice_test_case!(input, "- straight 😃 foo bar baz");
        let (remainder, (lines, bullet_kind, indent)) =
            parse_block_smart_list_auto_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(bullet_kind, BulletKind::Unordered);
        assert_eq2!(indent, 0);
        assert_eq2!(lines.len(), 1);
        assert_eq2!(
            &lines[0],
            &list![
                MdLineFragment::UnorderedListBullet {
                    indent: 0,
                    is_first_line: true
                },
                MdLineFragment::Plain("straight 😃 foo bar baz"),
            ]
        );
    }

    #[test]
    fn test_parse_block_smart_list_with_checkbox() {
        // Valid unchecked.
        {
            as_str_slice_test_case!(input, "- [ ] todo");
            let (remainder, (lines, _bullet_kind, _indent)) =
                parse_block_smart_list_auto_advance_alt(input).unwrap();
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, "- [x] done");
            let (remainder, (lines, _bullet_kind, _indent)) =
                parse_block_smart_list_auto_advance_alt(input).unwrap();
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, "- [ ]todo");
            let (remainder, (lines, _bullet_kind, _indent)) =
                parse_block_smart_list_auto_advance_alt(input).unwrap();
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, "- [x]done");
            let (remainder, (lines, _bullet_kind, _indent)) =
                parse_block_smart_list_auto_advance_alt(input).unwrap();
            let first_line = lines.first().unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
        as_str_slice_test_case!(input, "- foo", "  bar baz");
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
        let (remainder, (lines, _bullet_kind, _indent)) =
            parse_block_smart_list_auto_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ul_list_2() {
        as_str_slice_test_case!(input, "- foo", "  bar baz", "- foo1", "  bar1 baz1");
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
        let (remainder, (lines, _bullet_kind, _indent)) =
            parse_block_smart_list_auto_advance_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_line_end(), "- foo1");
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ol_list_1() {
        as_str_slice_test_case!(input, "1. foo", "   bar baz");
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
        let (remainder, (lines, _bullet_kind, _indent)) =
            parse_block_smart_list_auto_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines, expected);
    }

    #[test]
    fn test_valid_ol_list_2() {
        as_str_slice_test_case!(input, "1. foo", "   bar baz", "1. foo", "   bar baz");
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
        let (remainder, (lines, _bullet_kind, _indent)) =
            parse_block_smart_list_auto_advance_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_line_end(), "1. foo");
        assert_eq2!(lines, expected);
    }
}

/// Parses a complete smart list (ordered or unordered) from the input.
/// Returns the parsed [crate::SmartListIR] structure.
///
/// Examples:
/// - "- item\n  continued" → [crate::SmartListIR] with unordered bullet
/// - "1. task\n   details" → [crate::SmartListIR] with ordered bullet
///
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
/// This function parses a smart list from the input text and returns a tuple containing:
/// - The remainder of the input that wasn't consumed
/// - A [crate::SmartListIR] object representing the parsed smart list
///
/// The function handles both ordered and unordered lists, with proper indentation.
pub fn parse_smart_list_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult</* remainder */ AsStrSlice<'a>, SmartListIRAlt<'a>> {
    // Validate indentation and get the trimmed input.
    let (indent, input) = validate_list_indentation(input)?;

    // Try to parse as unordered list.
    if let Ok(result) = parse_unordered_list(input.clone(), indent) {
        return Ok(result);
    }

    // Try to parse as ordered list.
    if let Ok(result) = parse_ordered_list(input.clone(), indent) {
        return Ok(result);
    }

    // If neither unordered nor ordered list prefix matched, return an error.
    Err(NomErr::Error(NomError::new(input, NomErrorKind::Fail)))
}

/// Validates that the list indentation is a multiple of the base width (e.g., 2 or 4
/// spaces).
///
/// If successful, returns the indent size and trimmed input. Examples:
/// - "    - item" → (4, "- item"),
/// - "  1. task" → (2, "1. task").
fn validate_list_indentation<'a>(
    input: AsStrSlice<'a>,
) -> Result<(usize, AsStrSlice<'a>), NomErr<NomError<AsStrSlice<'a>>>> {
    // Match empty spaces & count them into indent
    let (indent, input) = input.trim_spaces_start_current_line();
    let indent = indent.as_usize();

    // Indent has to be multiple of the base width, otherwise it's not a list item
    if indent % LIST_PREFIX_BASE_WIDTH != 0 {
        return Err(NomErr::Error(NomError::new(input, NomErrorKind::Fail)));
    }

    Ok((indent, input))
}

/// Parse an unordered list item with the given indentation level.
///
/// If successful, returns the parsed [SmartListIR] structure. Examples:
/// - "- item" → [SmartListIR] with [BulletKind::Unordered]
fn parse_unordered_list<'a>(
    input: AsStrSlice<'a>,
    indent: usize,
) -> IResult<AsStrSlice<'a>, SmartListIRAlt<'a>> {
    // Match the bullet: Unordered => "- ".
    let (input, bullet) = tag(UNORDERED_LIST_PREFIX).parse(input)?;

    // Match the rest of the line & other lines that have the same indent.
    let (input, content_lines) =
        parse_smart_list_content_lines_alt(input, indent, bullet)?;

    // Return the result.
    Ok((
        input,
        SmartListIRAlt {
            indent,
            bullet_kind: BulletKind::Unordered,
            content_lines,
        },
    ))
}

/// Parse an ordered list item with the given indentation level.
///
/// If successful, returns the parsed [SmartListIR] structure. Examples:
/// - "1. item" → SmartListIR with BulletKind::Ordered(1)
/// - "42. task" → SmartListIR with BulletKind::Ordered(42)
fn parse_ordered_list<'a>(
    input: AsStrSlice<'a>,
    indent: usize,
) -> IResult<AsStrSlice<'a>, SmartListIRAlt<'a>> {
    let input_str = input.extract_to_line_end();

    // Parse and validate bullet.
    let (bullet_str, bullet_kind) =
        parse_ordered_list_bullet(input_str).map_err(|_| {
            nom::Err::Error(nom::error::Error::new(
                input.clone(),
                nom::error::ErrorKind::Digit,
            ))
        })?;

    // Split input at bullet boundary
    let bullet_len = bullet_str.len_chars().as_usize();
    let (bullet_slice, input) = input.take_split(bullet_len);

    // Parse content lines
    let (input, content_lines) =
        parse_smart_list_content_lines_alt(input, indent, bullet_slice)?;

    // Return result
    return Ok((
        input,
        SmartListIRAlt {
            indent,
            bullet_kind,
            content_lines,
        },
    ));

    /// Try to extract and validate ordered list bullet from input string.
    /// Returns the bullet string and [BulletKind]. Examples:
    /// - "1. text" → ("1. ", BulletKind::Ordered(1))
    /// - "123. item" → ("123. ", BulletKind::Ordered(123))
    fn parse_ordered_list_bullet(input_str: &str) -> IResult<&str, BulletKind> {
        // Parse bullet pattern: "123. "
        let bullet_res: IResult<_, _, NomError<_>> =
            recognize(terminated(digit1, tag(ORDERED_LIST_PARTIAL_PREFIX)))
                .parse(input_str);

        let Ok((_, bullet_str)) = bullet_res else {
            return Err(NomErr::Error(NomError::new(input_str, NomErrorKind::Tag)));
        };

        // Validate bullet starts with digit
        if !bullet_str.starts_with(|c: char| c.is_ascii_digit()) {
            return Err(NomErr::Error(NomError::new(input_str, NomErrorKind::Tag)));
        }

        // Parse number from bullet
        let number_str = bullet_str.trim_end_matches(ORDERED_LIST_PARTIAL_PREFIX);
        let Ok(number) = number_str.parse::<usize>() else {
            return Err(NomErr::Error(NomError::new(input_str, NomErrorKind::Digit)));
        };

        Ok((bullet_str, BulletKind::Ordered(number)))
    }
}

#[cfg(test)]
mod tests_parse_smart_list_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_with_unicode_emoji() {
        as_str_slice_test_case!(input, "- emoji 😀🎉 content", "  more emoji 🚀 content");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            actual.content_lines[0].content.extract_to_line_end(),
            "emoji 😀🎉 content"
        );
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "more emoji 🚀 content"
        );
    }

    #[test]
    fn test_stops_at_new_list_item() {
        as_str_slice_test_case!(input, "- first item", "  continuation", "- second item");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_line_end(), "- second item");
        assert_eq2!(actual.content_lines.len(), 2);
    }

    #[test]
    fn test_multi_digit_ordered() {
        as_str_slice_test_case!(input, "123. large number", "     continuation");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();

        // Check remainder
        assert_eq2!(remainder.is_empty(), true);

        // Check top-level fields
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Ordered(123));
        assert_eq2!(actual.content_lines.len(), 2);

        // Check first line (bullet line)
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "123. ");
        assert_eq2!(
            actual.content_lines[0].content.extract_to_line_end(),
            "large number"
        );

        // Check second line (continuation line)
        assert_eq2!(actual.content_lines[1].indent, 0);
        assert_eq2!(actual.content_lines[1].bullet_str, "123. ");
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "continuation"
        );
    }

    #[test]
    fn test_mixed_content_with_unicode() {
        as_str_slice_test_case!(input, "- 中文 content", "  العربية text", "  🌍 global");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();

        // Check remainder
        assert_eq2!(remainder.is_empty(), true);

        // Check top-level fields
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 3);

        // Check first line (Chinese content)
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[0].content.extract_to_line_end(),
            "中文 content"
        );

        // Check second line (Arabic content)
        assert_eq2!(actual.content_lines[1].indent, 0);
        assert_eq2!(actual.content_lines[1].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "العربية text"
        );

        // Check third line (Emoji content)
        assert_eq2!(actual.content_lines[2].indent, 0);
        assert_eq2!(actual.content_lines[2].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[2].content.extract_to_line_end(),
            "🌍 global"
        );
    }

    #[test]
    fn test_invalid_ul_list() {
        as_str_slice_test_case!(input, "  -");
        let actual = parse_smart_list_alt(input);
        assert_eq2!(actual.is_err(), true);
    }

    #[test]
    fn test_valid_ul_list() {
        // 1 item.
        {
            as_str_slice_test_case!(input, "- foo");
            let actual = parse_smart_list_alt(input);
            let (remainder, result) = actual.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(result.indent, 0);
            assert_eq2!(result.bullet_kind, BulletKind::Unordered);
            assert_eq2!(result.content_lines.len(), 1);
            assert_eq2!(result.content_lines[0].indent, 0);
            assert_eq2!(result.content_lines[0].bullet_str, "- ");
            assert_eq2!(result.content_lines[0].content.extract_to_line_end(), "foo");
        }

        // 2 items.
        {
            as_str_slice_test_case!(input, "- foo", "  bar");
            let actual = parse_smart_list_alt(input);
            let (remainder, result) = actual.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(result.indent, 0);
            assert_eq2!(result.bullet_kind, BulletKind::Unordered);
            assert_eq2!(result.content_lines.len(), 2);
            assert_eq2!(result.content_lines[0].indent, 0);
            assert_eq2!(result.content_lines[0].bullet_str, "- ");
            assert_eq2!(result.content_lines[0].content.extract_to_line_end(), "foo");
            assert_eq2!(result.content_lines[1].indent, 0);
            assert_eq2!(result.content_lines[1].bullet_str, "- ");
            assert_eq2!(result.content_lines[1].content.extract_to_line_end(), "bar");
        }
    }

    #[test]
    fn test_invalid_ol_list() {
        as_str_slice_test_case!(input, "  1.");
        let actual = parse_smart_list_alt(input);
        assert_eq2!(actual.is_err(), true);
    }

    #[test]
    fn test_valid_ol_list() {
        // 1 item.
        {
            as_str_slice_test_case!(input, "1. foo");
            let actual = parse_smart_list_alt(input);
            let (remainder, result) = actual.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(result.indent, 0);
            assert_eq2!(result.bullet_kind, BulletKind::Ordered(1));
            assert_eq2!(result.content_lines.len(), 1);
            assert_eq2!(result.content_lines[0].indent, 0);
            assert_eq2!(result.content_lines[0].bullet_str, "1. ");
            assert_eq2!(result.content_lines[0].content.extract_to_line_end(), "foo");
        }

        // 2 items.
        {
            as_str_slice_test_case!(input, "1. foo", "   bar");
            let actual = parse_smart_list_alt(input);
            let (remainder, result) = actual.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(result.indent, 0);
            assert_eq2!(result.bullet_kind, BulletKind::Ordered(1));
            assert_eq2!(result.content_lines.len(), 2);
            assert_eq2!(result.content_lines[0].indent, 0);
            assert_eq2!(result.content_lines[0].bullet_str, "1. ");
            assert_eq2!(result.content_lines[0].content.extract_to_line_end(), "foo");
            assert_eq2!(result.content_lines[1].indent, 0);
            assert_eq2!(result.content_lines[1].bullet_str, "1. ");
            assert_eq2!(result.content_lines[1].content.extract_to_line_end(), "bar");
        }
    }

    #[test]
    fn test_one_line() {
        as_str_slice_test_case!(input, "- foo");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 1);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
    }

    #[test]
    fn test_one_line_trailing_new_line() {
        as_str_slice_test_case!(input, "- foo", "");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), false);
        assert_eq2!(remainder.extract_to_line_end(), "");
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 1);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
    }

    #[test]
    fn test_two_lines_last_is_empty() {
        as_str_slice_test_case!(input, "- foo", "");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), false);
        assert_eq2!(remainder.extract_to_line_end(), "");
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 1);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
    }

    #[test]
    fn test_two_lines() {
        as_str_slice_test_case!(input, "- foo", "  bar baz");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 2);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
        assert_eq2!(actual.content_lines[1].indent, 0);
        assert_eq2!(actual.content_lines[1].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "bar baz"
        );
    }

    #[test]
    fn test_three_lines_last_is_empty() {
        as_str_slice_test_case!(input, "- foo", "  bar baz", "");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), false);
        assert_eq2!(remainder.extract_to_line_end(), "");
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 2);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
        assert_eq2!(actual.content_lines[1].indent, 0);
        assert_eq2!(actual.content_lines[1].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "bar baz"
        );
    }

    #[test]
    fn test_three_lines() {
        as_str_slice_test_case!(input, "- foo", "  bar baz", "  qux");
        let (remainder, actual) = parse_smart_list_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(actual.indent, 0);
        assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        assert_eq2!(actual.content_lines.len(), 3);
        assert_eq2!(actual.content_lines[0].indent, 0);
        assert_eq2!(actual.content_lines[0].bullet_str, "- ");
        assert_eq2!(actual.content_lines[0].content.extract_to_line_end(), "foo");
        assert_eq2!(actual.content_lines[1].indent, 0);
        assert_eq2!(actual.content_lines[1].bullet_str, "- ");
        assert_eq2!(
            actual.content_lines[1].content.extract_to_line_end(),
            "bar baz"
        );
        assert_eq2!(actual.content_lines[2].indent, 0);
        assert_eq2!(actual.content_lines[2].bullet_str, "- ");
        assert_eq2!(actual.content_lines[2].content.extract_to_line_end(), "qux");
    }

    #[test]
    fn test_bullet_kinds() {
        // Unordered.
        {
            as_str_slice_test_case!(input, "- foo");
            let (_remainder, actual) = parse_smart_list_alt(input).unwrap();
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }

        // Ordered.
        {
            as_str_slice_test_case!(input, "1. foo");
            let (_remainder, actual) = parse_smart_list_alt(input).unwrap();
            assert_eq2!(actual.bullet_kind, BulletKind::Ordered(1));
        }
    }

    #[test]
    fn test_indent() {
        // Indent = 0 Ok.
        {
            as_str_slice_test_case!(input, "- foo");
            let (_remainder, actual) = parse_smart_list_alt(input).unwrap();
            assert_eq2!(actual.indent, 0);
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }

        // Indent = 1 Fail.
        {
            as_str_slice_test_case!(input, " - foo");
            let result = parse_smart_list_alt(input);
            assert_eq2!(result.is_err(), true);
        }

        // Indent = 2 Ok.
        {
            as_str_slice_test_case!(input, "  - foo");
            let (_remainder, actual) = parse_smart_list_alt(input).unwrap();
            assert_eq2!(actual.indent, 2);
            assert_eq2!(actual.bullet_kind, BulletKind::Unordered);
        }
    }
}

/// Parses content lines that belong to a smart list item.
///
/// This function takes an input string slice, an indent level, and a bullet string,
/// and returns a vector of `SmartListLine` objects representing the parsed content.
/// It handles both the first line of a list item and any continuation lines that
/// follow it.
///
/// # ⚠️ Critical: Character-Based vs Byte-Based Indexing
///
/// This implementation **exclusively uses character-based indexing** throughout.
/// This is critical for proper Unicode/UTF-8 support, especially when handling
/// emojis and other multi-byte characters.
///
/// **Key principles followed:**
/// - Always use `AsStrSlice::take_from(char_count)` for character-based slicing
/// - Always use `AsStrSlice::extract_to_line_end()` to extract content
/// - Never use raw slice operators like `&str[byte_start..byte_end]`
/// - Convert byte positions from `find_substring()` to character positions before slicing
///
/// **Why this matters:**
/// ```no_run
/// # use r3bl_tui::{AsStrSlice, GCString, as_str_slice_test_case};
/// # use nom::Input;
/// // ❌ WRONG - byte-based slicing can panic or corrupt UTF-8
/// let text = "😀hello";
/// let bad = &text[1..]; // PANIC! Splits UTF-8 sequence
///
/// // ✅ CORRECT - character-based slicing with AsStrSlice
/// as_str_slice_test_case!(slice, "😀hello");
/// let good = slice.take_from(1).extract_to_line_end(); // "hello"
/// ```
///
/// Earlier versions of this function had bugs where byte positions from
/// `find_substring()` were used as character offsets in `take_from()`, causing
/// incorrect slicing for multi-byte characters.
///
/// # Parameters
/// - `input`: The input text to parse
/// - `indent`: The indentation level of the list item (number of spaces)
/// - `bullet`: The bullet string (e.g., "- ", "1. ") that precedes the list item
///
/// # Returns
/// A tuple containing:
/// - The remainder of the input that wasn't consumed
/// - A vector of `SmartListLine` objects representing the parsed content
pub fn parse_smart_list_content_lines_alt<'a>(
    input: AsStrSlice<'a>,
    indent: usize,
    bullet: AsStrSlice<'a>,
) -> IResult<
    /* remainder */ AsStrSlice<'a>,
    /* lines */ InlineVec<SmartListLineAlt<'a>>,
> {
    let mut indent_padding = InlineString::with_capacity(indent);
    pad_fmt!(
        fmt: indent_padding,
        pad_str: SPACE,
        repeat_count: indent
    );
    let indent_padding = indent_padding.as_str();

    // Early return if there are no more lines after the first one.
    let Some(first_line_end) = input.find_substring(NEW_LINE) else {
        // Return an empty remainder for single-line inputs
        let empty_remainder = input.take_from(input.input_len());
        return Ok((
            empty_remainder,
            smallvec![SmartListLineAlt {
                indent,
                bullet_str: bullet.extract_to_line_end(),
                content: input.limit_to_line_end()
            }],
        ));
    };

    // Keep the first line. There may be more than 1 line.
    let first = input.take(first_line_end);

    // ⚠️ CRITICAL: Convert byte position to character position.
    // `find_substring()` returns a BYTE offset, but `take_from()` expects a CHARACTER
    // offset. For Unicode/multi-byte characters like emojis, these are different!
    // We must count characters, not bytes, to avoid slicing UTF-8 sequences incorrectly.
    let first_line_char_count = first.extract_to_line_end().len_chars().as_usize();

    // We need to skip the first line + the newline character.
    let input = input.take_from(first_line_char_count + 1);

    // Match the rest of the lines.
    let (remainder, rest) = many0((
        verify(
            // FIRST STEP: Match the ul or ol list item line.
            preceded(
                // Match the indent.
                tag(indent_padding),
                // Match the rest of the line.
                /* output */
                alt((is_not(NEW_LINE), recognize(many0(anychar)))),
            ),
            // SECOND STEP: Verify it to make sure no ul or ol list prefix.
            |it: &AsStrSlice<'a>| {
                // `it` must not *just* have spaces.
                if it.trim_start_current_line_is_empty() {
                    return false;
                }

                // `it` must start w/ *exactly* the correct number of spaces.
                if !verify_rest::must_start_with_correct_num_of_spaces(
                    it.clone(),
                    bullet.clone(),
                ) {
                    return false;
                }

                // `it` must not start w/ the ul list prefix.
                // `it` must not start w/ the ol list prefix.
                verify_rest::list_contents_does_not_start_with_list_prefix(it.clone())
            },
        ),
        opt(tag(NEW_LINE)),
    ))
    .parse(input)?;

    // Convert `rest` into a Vec<&str> that contains the output lines.
    let output_lines: InlineVec<SmartListLineAlt<'_>> = {
        let mut it = InlineVec::with_capacity(rest.len() + 1);

        it.push(SmartListLineAlt {
            indent,
            bullet_str: bullet.extract_to_line_end(),
            content: first.limit_to_line_end(),
        });
        it.extend(rest.iter().map(
            // Skip "bullet's width" number of characters at the start of the line.
            |(rest_line_content, _newline)| {
                let bullet_len = bullet.input_len();

                // ⚠️ CRITICAL: Use character-aware slicing, not byte slicing
                // bullet.input_len() returns character count, so this is safe
                // Using take_from() ensures we skip the correct number of Unicode
                // characters.
                let content_slice = rest_line_content.take_from(bullet_len);
                let content = content_slice.limit_to_line_end();

                SmartListLineAlt {
                    indent,
                    bullet_str: bullet.extract_to_line_end(),
                    content,
                }
            },
        ));

        it
    };

    // Return the remainder as is - it contains the unparsed content
    Ok((remainder, output_lines))
}

#[cfg(test)]
mod tests_parse_smart_list_content_lines_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_single_line_no_newline() {
        as_str_slice_test_case!(input, "foo bar");
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 1);
        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "foo bar");
    }

    #[test]
    fn test_single_line_with_newline() {
        as_str_slice_test_case!(input, "foo bar");
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 1);
        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "foo bar");
    }

    #[test]
    fn test_multiple_lines_unordered() {
        as_str_slice_test_case!(input, "first line", "  second line", "  third line");
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_multiple_lines_ordered() {
        as_str_slice_test_case!(input, "first line", "   second line", "   third line");
        let indent = 0;
        as_str_slice_test_case!(bullet, "1. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_with_indent_unordered() {
        as_str_slice_test_case!(input, "first line", "    second line", "    third line");
        let indent = 2;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_with_indent_ordered() {
        as_str_slice_test_case!(
            input,
            "first line",
            "     second line",
            "     third line"
        );
        let indent = 2;
        as_str_slice_test_case!(bullet, "1. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 2);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_stops_at_new_list_item_unordered() {
        as_str_slice_test_case!(
            input,
            "first line",
            "  second line",
            "- new item",
            "  its content"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "- new item\n  its content\n"
        );
        assert_eq2!(lines.len(), 2);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");
    }

    #[test]
    fn test_stops_at_new_list_item_ordered() {
        as_str_slice_test_case!(
            input,
            "first line",
            "   second line",
            "2. new item",
            "   its content"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "1. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "2. new item\n   its content\n"
        );
        assert_eq2!(lines.len(), 2);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "second line");
    }

    #[test]
    fn test_stops_at_different_indent_list() {
        as_str_slice_test_case!(input, "first line", "  second line", "  - nested item");
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "  - nested item\n"
        );
        assert_eq2!(lines.len(), 2);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");
    }

    #[test]
    fn test_with_trailing_newlines() {
        as_str_slice_test_case!(
            input,
            "first line",
            "  second line",
            "",
            "other content"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "\nother content\n"
        );
        assert_eq2!(lines.len(), 2);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");
    }

    #[test]
    fn test_empty_continuation_lines() {
        as_str_slice_test_case!(input, "first line", "  ", "  third line");
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "  \n  third line\n"
        );
        assert_eq2!(lines.len(), 1);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");
    }

    #[test]
    fn test_insufficient_indent() {
        as_str_slice_test_case!(input, "first line", " second line"); // Only 1 space instead of 2
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.extract_to_slice_end().as_ref(), " second line\n");
        assert_eq2!(lines.len(), 1);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");
    }

    #[test]
    fn test_too_much_indent() {
        as_str_slice_test_case!(input, "first line", "   second line"); // 3 spaces instead of 2
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "   second line\n"
        );
        assert_eq2!(lines.len(), 1);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");
    }

    #[test]
    fn test_double_digit_ordered_list() {
        as_str_slice_test_case!(input, "first line", "    second line", "    third line");
        let indent = 0;
        as_str_slice_test_case!(bullet, "10. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "10. ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "10. ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "10. ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_triple_digit_ordered_list() {
        as_str_slice_test_case!(
            input,
            "first line",
            "     second line",
            "     third line"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "100. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "100. ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "100. ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "100. ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }

    #[test]
    fn test_unicode_content() {
        as_str_slice_test_case!(
            input,
            "😃 unicode",
            "  more 🎉 unicode",
            "  final 🚀 line"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "😃 unicode");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "more 🎉 unicode");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "final 🚀 line");
    }

    #[test]
    fn test_complex_multiline_with_mixed_indent() {
        as_str_slice_test_case!(
            input,
            "item1",
            "  continuation",
            "  more continuation",
            "- new item"
        );
        let indent = 0;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(remainder.extract_to_slice_end().as_ref(), "- new item\n");
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "item1");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "continuation");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "more continuation");
    }

    #[test]
    fn test_ordered_list_with_varying_numbers() {
        as_str_slice_test_case!(input, "item1", "   continuation", "5. different number");
        let indent = 0;
        as_str_slice_test_case!(bullet, "1. ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "5. different number\n"
        );
        assert_eq2!(lines.len(), 2);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "item1");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 0);
        assert_eq2!(*bullet_str, "1. ");
        assert_eq2!(content.extract_to_line_end(), "continuation");
    }

    #[test]
    fn test_complex_indented_scenario() {
        as_str_slice_test_case!(
            input,
            "first line",
            "      second line",
            "      third line",
            "    - nested list"
        );
        let indent = 4;
        as_str_slice_test_case!(bullet, "- ");

        let (remainder, lines) =
            parse_smart_list_content_lines_alt(input, indent, bullet).unwrap();

        assert_eq2!(
            remainder.extract_to_slice_end().as_ref(),
            "    - nested list\n"
        );
        assert_eq2!(lines.len(), 3);

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[0];
        assert_eq2!(*indent, 4);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "first line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[1];
        assert_eq2!(*indent, 4);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "second line");

        let SmartListLineAlt {
            indent,
            bullet_str,
            content,
        } = &lines[2];
        assert_eq2!(*indent, 4);
        assert_eq2!(*bullet_str, "- ");
        assert_eq2!(content.extract_to_line_end(), "third line");
    }
}

mod verify_rest {
    use super::*;

    /// Checks if the input string does not start with a list prefix.
    ///
    /// This function is used to determine if a line is a continuation of a list item
    /// rather than the start of a new list item. It ensures that the line doesn't
    /// start with an unordered list prefix (like "- ") or an ordered list prefix
    /// (like "1. ").
    ///
    /// ⚠️ **Character-Based Processing**: This function uses character-aware operations
    /// to safely handle Unicode text. It avoids byte-based slicing that could split
    /// UTF-8 sequences.
    ///
    /// # Return value
    /// Returns true if:
    /// - No unordered list items (at any indent level)
    /// - No ordered list items (with any number or indent level)
    ///
    /// # Parameters
    /// - `input`: The input text to check
    pub fn list_contents_does_not_start_with_list_prefix<'a>(
        input: AsStrSlice<'a>,
    ) -> bool {
        let trimmed_input = input.trim_start_current_line();

        // Check for unordered list prefix
        if trimmed_input.starts_with(UNORDERED_LIST_PREFIX) {
            return false;
        }

        // Check for ordered list prefix (digit(s) followed by ". ")
        let input_str = trimmed_input.extract_to_line_end();

        // Find the position of the first non-digit character using CHARACTER indices
        let mut digit_end = 0;
        for (char_position, c) in input_str.chars().enumerate() {
            if !c.is_ascii_digit() {
                // chars().enumerate() gives us character positions directly
                digit_end = char_position;
                break;
            }
            // If we reach the end of the string, all characters are digits
            digit_end = char_position + 1;
        }

        // If we found at least one digit, check if it's followed by ". "
        if digit_end > 0 {
            // ⚠️ Use character-aware slicing instead of raw slice operator
            let remaining_slice = trimmed_input.take_from(digit_end);
            let rest = remaining_slice.extract_to_line_end();
            if rest.starts_with(ORDERED_LIST_PARTIAL_PREFIX) {
                return false;
            }
        }

        // No list prefix found
        true
    }

    /// Verifies that a line starts with exactly the correct number of spaces
    /// for continuation of a list item. The number of spaces should match
    /// the length of the bullet string (which includes the bullet marker and following
    /// space).
    ///
    /// ⚠️ **Character-Based Counting**: Both space counting and bullet length use
    /// character-based counting, making this safe for Unicode bullet strings.
    ///
    /// # Examples
    /// - `"  content"` with bullet `"- "` (len=2) => true (2 spaces match bullet length)
    /// - `"    content"` with bullet `"1. "` (len=3) => false (4 spaces != 3)
    /// - `" content"` with bullet `"- "` (len=2) => false (1 space != 2)
    /// - `"content"` with bullet `"- "` (len=2) => false (0 spaces != 2)
    pub fn must_start_with_correct_num_of_spaces<'a>(
        input: AsStrSlice<'a>,
        my_bullet: AsStrSlice<'a>,
    ) -> bool {
        let it_spaces_at_start = count_spaces_at_start(input);
        let expected_spaces = my_bullet.input_len();
        it_spaces_at_start == expected_spaces
    }

    /// Counts the number of space characters at the start of the input string.
    ///
    /// This function is used to determine the indentation level of a line.
    /// It only counts space characters (ASCII 32), not other whitespace like tabs.
    ///
    /// ⚠️ **Character-Based Iteration**: Uses AsStrSlice's character-aware iteration
    /// to safely count spaces in Unicode text.
    ///
    /// # Parameters
    /// - `input`: The input text to analyze
    ///
    /// # Returns
    /// The number of space characters at the beginning of the input.
    /// If the input is all spaces or empty, returns the total length of the input.
    pub fn count_spaces_at_start<'a>(input: AsStrSlice<'a>) -> usize {
        let mut count: usize = 0;
        let mut current = input.clone();

        while let Some(ch) = current.current_char() {
            if ch == SPACE_CHAR {
                count += 1;
                current.advance();
            } else {
                break;
            }
        }

        count
    }
}

#[cfg(test)]
mod tests_verify_rest {
    use super::*;
    use crate::as_str_slice_test_case;

    #[test]
    fn test_list_contents_does_not_start_with_list_prefix() {
        // Test content that does NOT start with list prefixes (should return true)

        // Regular text content
        {
            as_str_slice_test_case!(input, "regular text content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content starting with spaces but no list prefix
        {
            as_str_slice_test_case!(input, "  indented content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content with dash but not followed by space
        {
            as_str_slice_test_case!(input, "-notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content with number but not followed by ". "
        {
            as_str_slice_test_case!(input, "123notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content with number followed by dot but no space
        {
            as_str_slice_test_case!(input, "1.notalist");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Empty string
        {
            as_str_slice_test_case!(input, "");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Only spaces
        {
            as_str_slice_test_case!(input, "   ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Test content that DOES start with list prefixes (should return false)

        // Unordered list prefix at start
        {
            as_str_slice_test_case!(input, "- this is a list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Unordered list prefix with leading spaces
        {
            as_str_slice_test_case!(input, "  - indented list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Ordered list prefix at start - single digit
        {
            as_str_slice_test_case!(input, "1. this is ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Ordered list prefix at start - multiple digits
        {
            as_str_slice_test_case!(input, "123. this is ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Ordered list prefix with leading spaces
        {
            as_str_slice_test_case!(input, "    2. indented ordered list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Just the unordered list prefix
        {
            as_str_slice_test_case!(input, "- ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Just the ordered list prefix
        {
            as_str_slice_test_case!(input, "42. ");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Multiple spaces before unordered list prefix
        {
            as_str_slice_test_case!(input, "      - deeply indented list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Multiple spaces before ordered list prefix
        {
            as_str_slice_test_case!(input, "        99. deeply indented ordered");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Edge case: zero as ordered list number
        {
            as_str_slice_test_case!(input, "0. zero numbered list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Edge case: very large number
        {
            as_str_slice_test_case!(input, "999999. large numbered list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Mixed whitespace before list prefix (tabs and spaces)
        {
            as_str_slice_test_case!(input, " \t - mixed whitespace");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Test edge cases with special characters

        // Content starting with asterisk (not unordered list prefix)
        {
            as_str_slice_test_case!(input, "*not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content starting with plus (not unordered list prefix)
        {
            as_str_slice_test_case!(input, "+not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content with Unicode characters
        {
            as_str_slice_test_case!(input, "😀 emoji content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // List prefix in middle of line (should not be detected)
        {
            as_str_slice_test_case!(input, "some text - not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // List prefix in middle of line (ordered)
        {
            as_str_slice_test_case!(input, "text 1. not a list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }
    }

    #[test]
    fn test_list_contents_does_not_start_with_list_prefix_unicode() {
        // Test Unicode content that does NOT start with list prefixes (should return
        // true)

        // Unicode content without list prefix
        {
            as_str_slice_test_case!(input, "😀 emoji content");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Multi-byte Unicode characters
        {
            as_str_slice_test_case!(input, "🎉🚀 multiple emojis");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Unicode with spaces before but no list prefix
        {
            as_str_slice_test_case!(input, "  🌟 indented emoji");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Unicode text that looks like it might contain numbers but doesn't form list
        // prefix
        {
            as_str_slice_test_case!(input, "👨‍👩‍👧‍👦 family emoji");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Unicode content starting with dash-like character but not ASCII dash
        {
            as_str_slice_test_case!(input, "— em dash not list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Unicode content starting with bullet-like character but not ASCII
        {
            as_str_slice_test_case!(input, "• bullet point not list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Test Unicode content that DOES start with list prefixes (should return false)

        // Unicode content with ASCII list prefix at start
        {
            as_str_slice_test_case!(input, "- 😀 emoji list item");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Unicode content with ordered list prefix
        {
            as_str_slice_test_case!(input, "1. 🎉 emoji ordered list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Unicode content with indented list prefix
        {
            as_str_slice_test_case!(input, "  - 🚀 indented emoji list");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Unicode content with indented ordered list
        {
            as_str_slice_test_case!(input, "    42. 🌟 indented emoji ordered");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Edge case: Unicode digits in ordered list prefix
        {
            as_str_slice_test_case!(input, "123. 🎯 numbered with emoji");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Unicode content with very long emoji sequence after list prefix
        {
            as_str_slice_test_case!(input, "- 👨‍👩‍👧‍👦🏠🌳 complex emoji family");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Mixed Unicode and ASCII in content but with ASCII list prefix
        {
            as_str_slice_test_case!(input, "99. Hello 世界 mixed languages");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(!result);
        }

        // Test edge cases with Unicode that might confuse byte/character indexing

        // Content starting with multi-byte character that looks like digit
        {
            as_str_slice_test_case!(input, "①not a list"); // Unicode digit one
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }

        // Content with Unicode spaces before list prefix - should NOT be detected as list
        // because Markdown only recognizes ASCII spaces for indentation
        {
            as_str_slice_test_case!(input, " \u{2003}- wide space before list"); // em space
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result); // Unicode spaces don't count as valid indentation
        }

        // Very long Unicode sequence without list prefix
        {
            as_str_slice_test_case!(input, "🏴󠁧󠁢󠁳󠁣󠁴󠁿 flag emoji sequence");
            let result =
                verify_rest::list_contents_does_not_start_with_list_prefix(input);
            assert!(result);
        }
    }

    #[test]
    fn test_count_whitespace_at_start() {
        // Test with no leading spaces
        {
            as_str_slice_test_case!(input, "hello world");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 0);
        }

        // Test with leading spaces
        {
            as_str_slice_test_case!(input, "   hello world");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 3);
        }

        // Test with all spaces
        {
            as_str_slice_test_case!(input, "    ");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 4);
        }

        // Test with empty string
        {
            as_str_slice_test_case!(input, "");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 0);
        }

        // Test with single space
        {
            as_str_slice_test_case!(input, " ");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 1);
        }

        // Test with tabs and spaces (should only count spaces)
        {
            as_str_slice_test_case!(input, "  \thello");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 2); // Only spaces, not tabs
        }

        // Test with non-space whitespace at start
        {
            as_str_slice_test_case!(input, "\t  hello");
            let count = verify_rest::count_spaces_at_start(input);
            assert_eq!(count, 0); // Tab is not a space character
        }
    }

    #[test]
    fn test_must_start_with_correct_num_of_spaces() {
        // Test case: 2 spaces with bullet "- " (length 2) => should be true
        {
            as_str_slice_test_case!(content, "  some content");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test case: 3 spaces with bullet "1. " (length 3) => should be true
        {
            as_str_slice_test_case!(content, "   more content");
            as_str_slice_test_case!(bullet, "1. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test case: 4 spaces with bullet "1. " (length 3) => should be false
        {
            as_str_slice_test_case!(content, "    too many spaces");
            as_str_slice_test_case!(bullet, "1. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result);
        }

        // Test case: 1 space with bullet "- " (length 2) => should be false
        {
            as_str_slice_test_case!(content, " not enough spaces");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result);
        }

        // Test case: 0 spaces with bullet "- " (length 2) => should be false
        {
            as_str_slice_test_case!(content, "no spaces");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result);
        }

        // Test case: exact match with longer bullet "10. " (length 4)
        {
            as_str_slice_test_case!(content, "    content here");
            as_str_slice_test_case!(bullet, "10. ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test case: empty content with empty bullet
        {
            as_str_slice_test_case!(content, "");
            as_str_slice_test_case!(bullet, "");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test case: content with only spaces matching bullet length
        {
            as_str_slice_test_case!(content, "  ");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test case: content with only spaces not matching bullet length
        {
            as_str_slice_test_case!(content, "   ");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result);
        }

        // Test case: content with mixed whitespace at start (only spaces should count)
        {
            as_str_slice_test_case!(content, " \tcontent");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result); // Only 1 space, not 2
        }
    }

    #[test]
    fn test_must_start_with_correct_num_of_spaces_edge_cases() {
        // Test with Unicode characters in content (should not affect space counting)
        {
            as_str_slice_test_case!(content, "  😀 emoji content");
            as_str_slice_test_case!(bullet, "- ");
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result);
        }

        // Test with Unicode characters in bullet
        {
            as_str_slice_test_case!(content, "   content");
            as_str_slice_test_case!(bullet, "● "); // bullet character + space = 2 chars
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result); // 3 spaces != 2 char bullet length
        }

        // Test with very long bullet
        {
            as_str_slice_test_case!(content, "      content");
            as_str_slice_test_case!(bullet, "100. "); // 5 characters
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(!result); // 6 spaces != 5 char bullet length
        }

        // Test with very long bullet - correct match
        {
            as_str_slice_test_case!(content, "     content");
            as_str_slice_test_case!(bullet, "100. "); // 5 characters
            let result =
                verify_rest::must_start_with_correct_num_of_spaces(content, bullet);
            assert!(result); // 5 spaces == 5 char bullet length
        }
    }
}

/// Parse markdown text with a specific checkbox policy until the end of line or input.
/// This function is used as a utility for parsing markdown text that may contain checkboxes.
/// It returns a list of markdown line fragments [MdLineFragments].
///
/// Does not consume the end of line character if it exists. If an EOL character
/// [crate::constants::NEW_LINE] is found:
/// - The EOL character is not included in the output.
/// - The EOL character is not consumed, and is part of the remainder.
#[rustfmt::skip]
pub fn parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt<'a>(
    input: AsStrSlice<'a>,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi_alt(it, checkbox_policy)
    ).parse(input)?;

    let it = List::from(output);

    Ok((input, it))
}

#[cfg(test)]
mod tests_checkbox_policy {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, list, MdLineFragment};

    #[test]
    fn test_ignore_checkbox_empty_string() {
        {
            as_str_slice_test_case!(input, "");
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(fragments, list![]);
        }
    }

    #[test]
    fn test_ignore_checkbox_non_empty_string() {
        {
            as_str_slice_test_case!(
                input,
                "here is some plaintext *but what if we italicize?"
            );
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(
                fragments,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Plain("*"),
                    MdLineFragment::Plain("but what if we italicize?"),
                ]
            );
        }
    }
}
