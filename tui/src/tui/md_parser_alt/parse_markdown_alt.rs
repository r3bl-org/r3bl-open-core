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

use nom::{branch::alt, combinator::map, multi::many0, IResult, Parser};

use crate::{constants::{AUTHORS, DATE, TAGS, TITLE},
            idx,
            parse_block_code_advance_alt,
            parse_block_smart_list_advance_alt,
            parse_line_csv_advance_alt,
            parse_line_heading_advance_alt,
            parse_line_kv_advance_alt,
            parse_line_text_advance_alt,
            sizing_list_of::ListStorage,
            AsStrSlice,
            List,
            MdDocument,
            MdElement,
            MdLineFragment,
            MdLineFragments,
            NomErr,
            NomError,
            NomErrorKind};

/// Primary public API for parsing markdown documents in the R3BL TUI editor component.
///
/// This is the main entry point used by the editor component to render
/// [`crate::EditorContent`] when any changes are made. The function is designed for
/// **high-performance operation** with minimal memory allocation and fast memory access
/// patterns, which is critical for large documents that require quick parsing for syntax
/// highlighting and rendering.
///
/// ## Performance Characteristics
/// - **Zero-allocation parsing**: Uses [`AsStrSlice`] for virtual array access without
///   copying
/// - **Fast memory access**: Optimized for editor component real-time rendering
///   requirements
/// - **Unicode-safe**: Full support for UTF-8 and multi-byte grapheme cluster segments
/// - **Panic-free**: Robust handling of Unicode emoji and complex text without crashes
///
/// ## Data Bridge Architecture
/// The [`AsStrSlice`] input provides a crucial bridge between how data is stored in
/// memory by the editor and how the nom parser expects to access it:
/// - Takes output from [`str::lines()`] and creates a virtual array interface
/// - Generates synthetic newlines to maintain line boundaries
/// - Implements the [`nom::Input`] trait for seamless nom parser integration
/// - Enables byte-level parsing while preserving the editor's line-based data structure
///
/// ## Parser Chain Design
/// Uses [`many0(alt(...))`](nom) with parsers ordered by specificity. Each parser wrapped
/// with [`ensure_advance_fail_safe_alt`] to prevent infinite loops and handle line
/// advancement automatically.
///
/// ### Parser Categories (in order of precedence)
/// - **Metadata**: Title, tags, authors, date (structured document properties)
/// - **Structure**: Headings (document hierarchy and navigation)
/// - **Content**: Smart lists, code blocks, empty lines, text (document body)
///
/// Note: The empty line parser must come before the text parser because the text parser
/// explicitly rejects empty input to prevent infinite loops. The empty line parser
/// handles both completely empty lines and lines with only whitespace.
pub fn parse_markdown_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdDocument<'a>> {
    // Use `many0` to apply the parser repeatedly, with advancement checking.
    let (rem, output_vec): (AsStrSlice<'a>, Vec<MdElement<'a>>) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            ensure_advance_fail_safe_alt(map(
                |it| parse_line_kv_advance_alt(TITLE, it),
                |maybe_title| match maybe_title {
                    None => MdElement::Title(""),
                    Some(title) => MdElement::Title(title.extract_to_line_end()),
                },
            )),
            ensure_advance_fail_safe_alt(map(
                |it| parse_line_csv_advance_alt(TAGS, it),
                |list| {
                    let acc: ListStorage<&str> =
                        list.iter().map(|item| item.extract_to_line_end()).collect();
                    MdElement::Tags(List::from(acc))
                },
            )),
            ensure_advance_fail_safe_alt(map(
                |it| parse_line_csv_advance_alt(AUTHORS, it),
                |list| {
                    let acc: ListStorage<&str> =
                        list.iter().map(|item| item.extract_to_line_end()).collect();
                    MdElement::Authors(List::from(acc))
                },
            )),
            ensure_advance_fail_safe_alt(map(
                |it| parse_line_kv_advance_alt(DATE, it),
                |maybe_date| match maybe_date {
                    None => MdElement::Date(""),
                    Some(date) => MdElement::Date(date.extract_to_line_end()),
                },
            )),
            ensure_advance_fail_safe_alt(map(
                parse_line_heading_advance_alt,
                MdElement::Heading,
            )),
            ensure_advance_fail_safe_alt(map(
                parse_block_smart_list_advance_alt,
                MdElement::SmartList,
            )),
            ensure_advance_fail_safe_alt(map(
                parse_block_code_advance_alt,
                MdElement::CodeBlock,
            )),
            ensure_advance_fail_safe_alt(map(
                parse_line_empty_advance_alt,
                MdElement::Text,
            )),
            ensure_advance_fail_safe_alt(map(
                parse_line_text_advance_alt,
                MdElement::Text,
            )),
        )),
    )
    .parse(input)?;

    let output_list = List::from(output_vec);

    Ok((rem, output_list))
}

/// Parse empty or whitespace-only lines that the main text parser rejects.
///
/// ## Purpose
/// The main text parser ([`parse_line_text_advance_alt()`])
/// explicitly rejects empty input to prevent infinite loops, so this specialized parser
/// handles empty or whitespace-only lines that need to be preserved in the document
/// structure.
///
/// ## Input format
/// Accepts any line that contains only whitespace characters (spaces,
/// tabs) or is completely empty. The line may or may not end with a newline character.
///
/// ## Line advancement
/// This is a **single-line parser that auto-advances**. It consumes
/// the optional trailing newline if present, making it consistent with other
/// single-line parsers like headings and metadata parsers. The parser properly
/// advances the line position when a newline is encountered.
///
/// ## Returns
/// - Either `Ok((advanced_input, empty_MdLineFragments))` for whitespace-only lines,
///   where the returned fragments list is empty.
/// - Or `Err` if the current line contains any non-whitespace characters.
///
/// ## Parser Ordering
/// This parser must come before the general text parser
/// ([`parse_line_text_advance_alt`]) in the parser chain because
/// the text parser explicitly rejects empty input. This is the correct approach as it
/// allows both empty lines and whitespace-only lines to be properly handled.
///
/// ## Example
/// `"   \t  \n"` → `Ok((advanced_input, []))` (empty fragments list)
pub fn parse_line_empty_advance_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let current_line = input.extract_to_line_end();
    if current_line.trim().is_empty() {
        // Create a mutable copy to advance to the next line
        let mut remainder = input;

        // Advance to the end of the current line
        let current_line_len = remainder
            .lines
            .get(remainder.line_index.as_usize())
            .map(|line| line.string.chars().count())
            .unwrap_or(0);

        // Advance through all characters in the current line
        for _ in remainder.char_index.as_usize()..current_line_len {
            remainder.advance();
        }

        // Advance past the end of the line to move to the next line
        remainder.advance();

        // Check if the line is completely empty or just contains whitespace
        let fragments = if current_line.is_empty() {
            // For completely empty lines, return an empty fragment list
            List::from(vec![])
        } else {
            // For lines with just whitespace, preserve the whitespace
            List::from(vec![MdLineFragment::Plain(current_line)])
        };
        Ok((remainder, fragments))
    } else {
        Err(NomErr::Error(NomError::new(input, NomErrorKind::Tag)))
    }
}

/// This is a failsafe wrapper that ensures the parser consumes the entire
/// input line without stalling the progress of the parser.
///
/// ## Ensures parser progress and handles automatic line advancement
///
/// **Why manual line advancement is needed**: Unlike the original
/// [`crate::parse_markdown()`] parser which operated on a single string slice `&str` with
/// embedded [crate::constants::NEW_LINE] characters that naturally provided line
/// advancement, this `parse_markdown_alt()` parser works with [`AsStrSlice`] input that
/// comes from [`crate::EditorContent`].
/// - The [`crate::EditorContent`] is created from [`str::lines()`] which strips out
///   newline characters, resulting in an array of line strings without embedded
///   [crate::constants::NEW_LINE].
/// - This input structure (from disk via [`String`] or in-memory via `&[GCString]`).
/// - This requires explicit line-to-line advancement that single-line parsers in this
///   file don't handle automatically, since they only consume characters within the
///   current line.
///
/// ## Prevents infinite loops by verifying position advancement after parsing
///
/// Normalizes different parser advancement behaviors:
///
/// - **Block parsers**: Handle their own multi-line advancement (e.g.,
///   [`parse_block_smart_list_advance_alt`], [`parse_block_code_advance_alt`])
///   - these consume multiple lines and manage their own line advancement internally
/// - **Single-line parsers with auto-advance**: Already advance to next line (e.g.,
///   [`parse_line_heading_advance_alt`], [`parse_line_empty_advance_alt`]) - these are
///   structural parsers that inherently consume the entire line including line
///   termination, so they naturally advance to the next line as part of their parsing
///   logic
/// - **Single-line parsers without auto-advance**: Only consume characters within line
///   - these are designed to extract specific content from within a line without
///     consuming
///   the line boundary, allowing for potential composition or partial line parsing
///
/// For parsers that don't auto-advance, automatically moves to the next line after
/// successful parsing. This serves as a failsafe for future parsers that might not
/// implement auto-advance behavior.
///
/// ## Error Handling
///
/// **End-of-Input Detection**: Uses dual criteria to detect when parsing should
/// terminate:
/// 1. **Line exhaustion**: `line_index >= lines.len()` - no more lines available to
///    parse.
/// 2. **Character exhaustion**: `current_taken >= total_size` - all characters consumed.
///
/// This function checks BOTH line exhaustion AND character exhaustion for end-of-input
/// detection. When either condition is met, returns EOF error to terminate [`many0`]
/// parsing. This ensures robust end-of-input detection whether input is exhausted by line
/// boundaries or character consumption.
///
/// Returns an error if no progress is detected, breaking the [`many0`] loop to prevent
/// infinite parsing.
pub fn ensure_advance_fail_safe_alt<'a, F, O>(
    mut parser: F,
) -> impl FnMut(AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, O>
where
    F: Parser<AsStrSlice<'a>, Output = O, Error = nom::error::Error<AsStrSlice<'a>>>,
{
    move |input: AsStrSlice<'a>| {
        // Check if we're at the end of input before trying to parse
        // We're at the end of input if we've gone past the last line OR consumed all
        // available characters
        if input.line_index >= input.lines.len().into()
            || input.current_taken >= input.total_size
        {
            return Err(NomErr::Error(NomError::new(input, NomErrorKind::Eof)));
        }

        // Store the initial position to check for advancement
        let initial_position = input.current_taken;
        let initial_line = input.line_index;
        let initial_char = input.char_index;

        // Apply the parser
        let result = parser.parse(input.clone());

        match result {
            Ok((mut remainder, output)) => {
                // Check if the parser advanced to a new line (which is ideal)
                let advanced_to_new_line = remainder.line_index > initial_line;

                if advanced_to_new_line {
                    // Parser already made proper line advancement, return the result.
                    return Ok((remainder, output));
                }

                // Check if parser made progress within the current line
                let made_char_progress = remainder.current_taken > initial_position
                    || remainder.char_index > initial_char;

                // For empty lines, even if no char progress was made, we should still
                // advance
                let current_line = remainder
                    .lines
                    .get(remainder.line_index.as_usize())
                    .map(|line| line.string.as_str())
                    .unwrap_or("");
                let is_empty_line = current_line.trim().is_empty();

                // For single-line parsers that made progress OR successfully parsed an
                // empty line, we need to manually advance to the next
                // line
                if (made_char_progress || is_empty_line)
                    && remainder.line_index < remainder.lines.len().into()
                {
                    // Calculate how many characters we need to advance to get to the
                    // start of the next line
                    let current_line_len = remainder
                        .lines
                        .get(remainder.line_index.as_usize())
                        .map(|line| line.string.chars().count())
                        .unwrap_or(0);

                    // If we're still within the current line content, advance to the end
                    // of the line.
                    if remainder.char_index.as_usize() < current_line_len {
                        let chars_to_advance =
                            current_line_len - remainder.char_index.as_usize();
                        for _ in 0..chars_to_advance {
                            remainder.advance();
                        }
                    }

                    // Check if we're at a valid position to advance to next line
                    if remainder.line_index.as_usize() < remainder.lines.len() - 1 {
                        // Create a fresh AsStrSlice at the next line with no max_len
                        // constraint This prevents issues where
                        // max_len=0 causes the parser to think input is empty
                        let next_line_index = remainder.line_index + idx(1);
                        remainder = AsStrSlice::with_limit(
                            remainder.lines,
                            next_line_index,
                            idx(0), // Start at beginning of next line
                            None,   // Remove max_len constraint
                        );
                    }

                    // Return the result with proper line advancement
                    return Ok((remainder, output));
                }

                // Check if we're at the end of input (no more content to parse)
                // Only consider it end-of-input if we've exhausted all lines OR consumed
                // all characters
                if remainder.line_index >= remainder.lines.len().into()
                    || remainder.current_taken >= remainder.total_size
                {
                    return Err(NomErr::Error(NomError::new(input, NomErrorKind::Eof)));
                }

                // If no progress was made at all, return an error to break the many0
                // loop.
                Err(NomErr::Error(NomError::new(input, NomErrorKind::Verify)))
            }
            Err(e) => Err(e),
        }
    }
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_integration_block_smart_lists_alt {
    use crate::{as_str_slice_test_case,
                assert_eq2,
                parse_markdown_alt,
                AsStrSlice,
                GCString,
                PrettyPrintDebug};

    #[test]
    fn test_parse_valid_md_ol_with_indent() {
        let raw_input =
            "start\n1. ol1\n  2. ol2\n     ol2.1\n    3. ol3\n       ol3.1\n       ol3.2\nend\n";
        let binding = raw_input
            .lines()
            .map(GCString::from)
            .collect::<Vec<GCString>>();
        let input = AsStrSlice::from(binding.as_slice());

        let expected_output = [
            "start",
            "[  ┊1.│ol1┊  ]",
            "[  ┊  2.│ol2┊ → ┊    │ol2.1┊  ]",
            "[  ┊    3.│ol3┊ → ┊      │ol3.1┊ → ┊      │ol3.2┊  ]",
            "end",
        ];

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_parse_valid_md_ul_with_indent() {
        let raw_input =
            "start\n- ul1\n  - ul2\n    ul2.1\n    - ul3\n      ul3.1\n      ul3.2\nend\n";
        let binding = raw_input
            .lines()
            .map(GCString::from)
            .collect::<Vec<GCString>>();
        let input = AsStrSlice::from(binding.as_slice());

        let expected_output = [
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊───┤ul2┊ → ┊   │ul2.1┊  ]",
            "[  ┊─────┤ul3┊ → ┊     │ul3.1┊ → ┊     │ul3.2┊  ]",
            "end",
        ];

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_parse_valid_md_multiline_no_indent() {
        as_str_slice_test_case!(
            input,
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
        );

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

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_parse_valid_md_no_indent() {
        as_str_slice_test_case!(
            input,
            "start",
            "- ul1",
            "- ul2",
            "1. ol1",
            "2. ol2",
            "- [ ] todo",
            "- [x] done",
            "end",
            "",
        );

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

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }
}

#[cfg(test)]
mod tests_parse_markdown_alt {
    use super::*;
    use crate::{as_str_slice_test_case,
                assert_eq2,
                convert_into_code_block_lines,
                list,
                BulletKind,
                HeadingData,
                HeadingLevel,
                HyperlinkData,
                MdLineFragment};

    #[test]
    fn test_no_line() {
        as_str_slice_test_case!(input, "Something");
        let (remainder, blocks) = parse_markdown_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![MdLineFragment::Plain("Something")])
        );
    }

    #[test]
    fn test_one_line() {
        as_str_slice_test_case!(input, "Something", "");
        let (remainder, blocks) = parse_markdown_alt(input).unwrap();
        println!("DEBUG test_one_line: remainder.is_empty()={}, remainder.line_index={}, remainder.char_index={}, remainder.lines.len()={}, remainder.current_taken={}, remainder.total_size={}",
                 remainder.is_empty(),
                 remainder.line_index.as_usize(),
                 remainder.char_index.as_usize(),
                 remainder.lines.len(),
                 remainder.current_taken.as_usize(),
                 remainder.total_size.as_usize());
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![MdLineFragment::Plain("Something")])
        );
    }
    #[test]
    fn test_parse_markdown_with_invalid_text_in_heading() {
        as_str_slice_test_case!(input, "# LINE 1", "", "##% LINE 2 FOO_BAR:", "");
        let res = parse_markdown_alt(input);
        let (remainder, blocks) = res.unwrap();
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
        assert_eq2!(blocks.len(), 3);
        assert_eq2!(remainder.extract_to_line_end(), "");
    }

    #[test]
    fn test_parse_markdown_single_line_plain_text() {
        as_str_slice_test_case!(input, "_this should not be italic", "");
        let (remainder, blocks) = parse_markdown_alt(input).unwrap();
        assert_eq2!(
            blocks[0],
            MdElement::Text(list![
                MdLineFragment::Plain("_"),
                MdLineFragment::Plain("this should not be italic"),
            ])
        );
        assert_eq2!(blocks.len(), 1);
        assert_eq2!(remainder.extract_to_line_end(), "");
    }

    #[test]
    fn test_parse_markdown_valid() {
        as_str_slice_test_case!(input,
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
            ""
        );

        let (remainder, list_block) = parse_markdown_alt(input).unwrap();

        assert_eq2!(list_block.len(), 20);

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
            MdElement::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            MdElement::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            MdElement::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
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
            MdElement::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
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

        assert_eq2!(remainder.extract_to_line_end(), "");
        assert_eq2!(list_block.len(), vec_block.len());

        list_block
            .iter()
            .zip(vec_block.iter())
            .for_each(|(lhs, rhs)| assert_eq2!(lhs, rhs));
    }

    #[test]
    fn test_debug_line_advancement() {
        as_str_slice_test_case!(input, "line1", "", "line3");

        println!("=== DEBUG LINE ADVANCEMENT ===");
        println!("Starting to parse input with {} lines", input.lines.len());
        for (i, line) in input.lines.iter().enumerate() {
            println!("Line {}: '{}'", i, line.string);
        }

        let result = parse_markdown_alt(input);
        match result {
            Ok((remainder, list_block)) => {
                println!("Parsed {} blocks", list_block.len());
                println!("Remainder lines count: {}", remainder.lines.len());
                println!("Remainder is empty: {}", remainder.is_empty());
                println!(
                    "Remainder current line index: {}",
                    remainder.line_index.as_usize()
                );
                println!(
                    "Remainder current char index: {}",
                    remainder.char_index.as_usize()
                );
                if !remainder.lines.is_empty() {
                    println!("Remainder line content:");
                    for (i, line) in remainder.lines.iter().enumerate() {
                        println!("  Line {}: '{}'", i, line.string);
                    }
                }

                for (i, block) in list_block.iter().enumerate() {
                    println!("Block {}: {:?}", i, block);
                }

                // This should parse all 3 lines
                assert_eq2!(list_block.len(), 3);
                assert_eq2!(remainder.is_empty(), true);
            }
            Err(e) => {
                println!("Parse failed with error: {:?}", e);
                panic!("Parse failed");
            }
        }
    }
}

#[cfg(test)]
mod tests_debug_empty_line_issue {
    use super::*;
    use crate::as_str_slice_test_case;

    #[test]
    fn test_debug_simple_case_with_empty_line() {
        println!("=== DEBUGGING SIMPLE CASE ===");
        as_str_slice_test_case!(input, "# Heading", "", "```bash", "echo test", "```");

        let result = parse_markdown_alt(input);
        match result {
            Ok((remainder, blocks)) => {
                println!("SUCCESS: Parsed {} blocks", blocks.len());
                println!("Remainder has {} lines", remainder.lines.len());
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                panic!("Should not fail");
            }
        }
    }
}
