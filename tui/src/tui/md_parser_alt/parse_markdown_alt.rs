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

use nom::{branch::alt,
          bytes::complete::tag,
          combinator::{map, opt},
          multi::many0,
          IResult,
          Parser};

use crate::{constants::{AUTHORS, DATE, TAGS, TITLE},
            md_parser::constants::NEW_LINE,
            parse_block_code_auto_advance_alt,
            parse_block_smart_list_auto_advance_alt,
            parse_single_line_csv_auto_advance_alt,
            parse_single_line_heading_auto_advance_alt,
            parse_single_line_text_auto_advance_alt,
            parse_unique_kv_opt_eol_alt,
            sizing_list_of::ListStorage,
            AsStrSlice,
            List,
            MdDocument,
            MdElement,
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
/// with [`ensure_progress_fail_safe`] to prevent infinite loops and handle line
/// advancement automatically.
///
/// ### Parser Categories (in order of precedence)
/// - **Metadata**: Title, tags, authors, date (structured document properties)
/// - **Structure**: Headings (document hierarchy and navigation)
/// - **Content**: Smart lists, code blocks, text, empty lines (document body)
pub fn parse_markdown_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdDocument<'a>> {
    // Use `many0` to apply the parser repeatedly, with advancement checking.
    let (rem, output_vec): (AsStrSlice<'a>, Vec<MdElement<'a>>) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            ensure_progress_fail_safe(map(
                |it| parse_unique_kv_opt_eol_alt(TITLE, it),
                |maybe_title| match maybe_title {
                    None => MdElement::Title(""),
                    Some(title) => MdElement::Title(title.extract_to_line_end()),
                },
            )),
            ensure_progress_fail_safe(map(
                |it| parse_single_line_csv_auto_advance_alt(TAGS, it),
                |list| {
                    let acc: ListStorage<&str> =
                        list.iter().map(|item| item.extract_to_line_end()).collect();
                    MdElement::Tags(List::from(acc))
                },
            )),
            ensure_progress_fail_safe(map(
                |it| parse_single_line_csv_auto_advance_alt(AUTHORS, it),
                |list| {
                    let acc: ListStorage<&str> =
                        list.iter().map(|item| item.extract_to_line_end()).collect();
                    MdElement::Authors(List::from(acc))
                },
            )),
            ensure_progress_fail_safe(map(
                |it| parse_unique_kv_opt_eol_alt(DATE, it),
                |maybe_date| match maybe_date {
                    None => MdElement::Date(""),
                    Some(date) => MdElement::Date(date.extract_to_line_end()),
                },
            )),
            ensure_progress_fail_safe(map(
                parse_single_line_heading_auto_advance_alt,
                MdElement::Heading,
            )),
            ensure_progress_fail_safe(map(
                parse_block_smart_list_auto_advance_alt,
                MdElement::SmartList,
            )),
            ensure_progress_fail_safe(map(
                parse_block_code_auto_advance_alt,
                MdElement::CodeBlock,
            )),
            ensure_progress_fail_safe(map(
                parse_single_empty_line_auto_advance_alt,
                MdElement::Text,
            )),
            ensure_progress_fail_safe(map(
                parse_single_line_text_auto_advance_alt,
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
/// The main text parser ([`parse_single_line_text_auto_advance_alt()`])
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
/// ## Example
/// `"   \t  \n"` → `Ok((advanced_input, []))` (empty fragments list)
pub fn parse_single_empty_line_auto_advance_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let current_line = input.extract_to_line_end();
    if current_line.trim().is_empty() {
        // Parse the empty line content and optional newline in one go, like other
        // single-line parsers
        let (remainder, _) = opt(tag(NEW_LINE)).parse(input)?;
        let empty_fragments = List::from(vec![]);
        Ok((remainder, empty_fragments))
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
///   [`parse_block_smart_list_auto_advance_alt`], [`parse_block_code_auto_advance_alt`])
///   - these consume multiple lines and manage their own line advancement internally
/// - **Single-line parsers with auto-advance**: Already advance to next line (e.g.,
///   [`parse_single_line_heading_auto_advance_alt`],
///   [`parse_single_empty_line_auto_advance_alt`]) - these are structural parsers that
///   inherently consume the entire line including line termination, so they naturally
///   advance to the next line as part of their parsing logic
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
/// Returns an error if no progress is detected, breaking the [`many0`] loop to prevent
/// infinite parsing.
pub fn ensure_progress_fail_safe<'a, F, O>(
    mut parser: F,
) -> impl FnMut(AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, O>
where
    F: Parser<AsStrSlice<'a>, Output = O, Error = nom::error::Error<AsStrSlice<'a>>>,
{
    move |input: AsStrSlice<'a>| {
        // Store the initial position to check for advancement
        let initial_position = input.current_taken;
        let initial_line = input.line_index;
        let initial_char = input.char_index;

        // Apply the parser
        let result = parser.parse(input.clone());

        match result {
            Ok((mut remainder, output)) => {
                // First, check if the parser already made sufficient progress.
                let made_progress = remainder.current_taken > initial_position
                    || remainder.line_index > initial_line
                    || remainder.char_index > initial_char;

                if made_progress {
                    // Parser already made progress, return the result.
                    return Ok((remainder, output));
                }

                // Parser succeeded but didn't make progress - try to advance to next line
                // This handles single-line parsers that parse successfully but don't
                // advance.
                if remainder.line_index < remainder.lines.len().saturating_sub(1).into() {
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

                    // Now advance past the end of line to get to the next line.
                    remainder.advance();

                    // Verify we actually made progress after line advancement.
                    if remainder.current_taken > initial_position
                        || remainder.line_index > initial_line
                        || remainder.char_index != initial_char
                    {
                        return Ok((remainder, output));
                    }
                }

                // If we still haven't made progress, return an error to break the many0
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
                list,
                HeadingData,
                HeadingLevel,
                MdLineFragment};

    #[test]
    fn test_no_line() {
        as_str_slice_test_case!(input, "Something");
        let (remainder, blocks) = parse_markdown_alt(input).unwrap();
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
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
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
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
}
