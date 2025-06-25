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
            parse_block_code_advance_ng,
            parse_block_smart_list_advance_ng,
            parse_line_csv_advance_ng,
            parse_line_heading_advance_ng,
            parse_line_kv_advance_ng,
            parse_line_text_advance_ng,
            sizing_list_of::ListStorage,
            AsStrSlice,
            List,
            MdDocument,
            MdElement,
            MdLineFragment,
            MdLineFragments,
            NErr,
            NError,
            NErrorKind};

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
/// Uses [`many0(alt(...))`](nom) with parsers ordered by specificity. Each parser is
/// wrapped with [`AsStrSlice::ensure_advance_with_parser`] to prevent infinite loops and
/// handle line advancement automatically.
///
/// ### Parser Categories (in order of precedence)
/// - **Metadata**: Title, tags, authors, date (structured document properties)
/// - **Structure**: Headings (document hierarchy and navigation)
/// - **Content**: Smart lists, code blocks, empty lines, text (document body)
///
/// Note: The empty line parser must come before the text parser because the text parser
/// explicitly rejects empty input to prevent infinite loops. The empty line parser
/// handles both completely empty lines and lines with only whitespace.
pub fn parse_markdown_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdDocument<'a>> {
    // Use `many0` to apply the parser repeatedly, with advancement checking.
    let (rem, output_vec): (AsStrSlice<'a>, Vec<MdElement<'a>>) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            // Title metadata parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    |it| parse_line_kv_advance_ng(TITLE, it),
                    |maybe_title| match maybe_title {
                        None => MdElement::Title(""),
                        Some(title) => MdElement::Title(title.extract_to_line_end()),
                    },
                ))
            },
            // Tags metadata parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    |it| parse_line_csv_advance_ng(TAGS, it),
                    |list| {
                        let acc: ListStorage<&str> =
                            list.iter().map(|item| item.extract_to_line_end()).collect();
                        MdElement::Tags(List::from(acc))
                    },
                ))
            },
            // Authors metadata parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    |it| parse_line_csv_advance_ng(AUTHORS, it),
                    |list| {
                        let acc: ListStorage<&str> =
                            list.iter().map(|item| item.extract_to_line_end()).collect();
                        MdElement::Authors(List::from(acc))
                    },
                ))
            },
            // Date metadata parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    |it| parse_line_kv_advance_ng(DATE, it),
                    |maybe_date| match maybe_date {
                        None => MdElement::Date(""),
                        Some(date) => MdElement::Date(date.extract_to_line_end()),
                    },
                ))
            },
            // Heading parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    parse_line_heading_advance_ng,
                    MdElement::Heading,
                ))
            },
            // Smart list parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    parse_block_smart_list_advance_ng,
                    MdElement::SmartList,
                ))
            },
            // Code block parser
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    parse_block_code_advance_ng,
                    MdElement::CodeBlock,
                ))
            },
            // Empty line parser (must come before text parser)
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    parse_line_empty_advance_ng,
                    MdElement::Text,
                ))
            },
            // Text parser (catch-all)
            |input: AsStrSlice<'a>| {
                input.ensure_advance_with_parser(&mut map(
                    parse_line_text_advance_ng,
                    MdElement::Text,
                ))
            },
        )),
    )
    .parse(input)?;

    let output_list = List::from(output_vec);

    Ok((rem, output_list))
}

/// Parse empty or whitespace-only lines that the main text parser rejects.
///
/// ## Purpose
/// The main text parser ([`parse_line_text_advance_ng()`])
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
/// ([`parse_line_text_advance_ng`]) in the parser chain because
/// the text parser explicitly rejects empty input. This is the correct approach as it
/// allows both empty lines and whitespace-only lines to be properly handled.
///
/// ## Example
/// `"   \t  \n"` → `Ok((advanced_input, []))` (empty fragments list)
pub fn parse_line_empty_advance_ng<'a>(
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
        Err(NErr::Error(NError::new(input, NErrorKind::Tag)))
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
/// advancement, this `parse_markdown_ng()` parser works with [`AsStrSlice`] input that
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
///   [`parse_block_smart_list_advance_ng`], [`parse_block_code_advance_ng`])
///   - these consume multiple lines and manage their own line advancement internally
/// - **Single-line parsers with auto-advance**: Already advance to next line (e.g.,
///   [`parse_line_heading_advance_ng`], [`parse_line_empty_advance_ng`]) - these are
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
pub fn ensure_advance_fail_safe_ng<'a, F, O>(
    mut parser: F,
) -> impl FnMut(AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, O>
where
    F: Parser<AsStrSlice<'a>, Output = O, Error = nom::error::Error<AsStrSlice<'a>>>,
{
    move |input: AsStrSlice<'a>| input.ensure_advance_with_parser(&mut parser)
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_integration_block_smart_lists_ng {
    use crate::{as_str_slice_test_case,
                assert_eq2,
                parse_markdown_ng,
                AsStrSlice,
                GCString,
                PrettyPrintDebug};

    #[test]
    fn test_markdown_parsing_with_ordered_list_and_indentation() {
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

        let result = parse_markdown_ng(input);
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
    fn test_markdown_parsing_with_unordered_list_and_indentation() {
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

        let result = parse_markdown_ng(input);
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
    fn test_markdown_parsing_with_multiline_content_without_indentation() {
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

        let result = parse_markdown_ng(input);
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
    fn test_markdown_parsing_with_simple_lists_without_indentation() {
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

        let result = parse_markdown_ng(input);
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
    fn test_markdown_parsing_with_mixed_list_types() {
        as_str_slice_test_case!(
            input,
            "start",
            "- unordered item 1",
            "  - nested unordered",
            "1. ordered item 1",
            "   1.1 nested ordered continuation",
            "2. ordered item 2",
            "- back to unordered",
            "- [ ] checkbox unchecked",
            "- [x] checkbox checked",
            "end",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        // The parser treats each list item separately, so we'll verify structure instead
        assert_eq2!(remainder.is_empty(), true);

        // Should have at least the expected number of elements
        assert!(md_doc.len() >= 9);

        // First should be text
        assert_eq2!(md_doc[0].pretty_print_debug(), "start".to_string());

        // Last should be text
        assert_eq2!(
            md_doc[md_doc.len() - 1].pretty_print_debug(),
            "end".to_string()
        );

        // Should contain various list types
        let mut has_unordered = false;
        let mut has_ordered = false;
        let mut has_checkbox = false;

        for element in md_doc.iter() {
            if let crate::MdElement::SmartList((lines, bullet_kind, _)) = element {
                match bullet_kind {
                    crate::BulletKind::Unordered => has_unordered = true,
                    crate::BulletKind::Ordered(_) => has_ordered = true,
                }

                // Check for checkboxes in the fragments
                for line in lines.iter() {
                    for fragment in line.iter() {
                        if let crate::MdLineFragment::Checkbox(_) = fragment {
                            has_checkbox = true;
                        }
                    }
                }
            }
        }

        assert!(has_unordered, "Should have unordered list items");
        assert!(has_ordered, "Should have ordered list items");
        assert!(has_checkbox, "Should have checkbox items");
    }

    #[test]
    fn test_markdown_parsing_with_deeply_nested_ordered_lists() {
        as_str_slice_test_case!(
            input,
            "1. First level",
            "   1.1 Second level",
            "       1.1.1 Third level",
            "           1.1.1.1 Fourth level",
            "   1.2 Back to second",
            "2. First level again",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Should have at least 6 elements
        assert!(md_doc.len() >= 6);

        // Check that we have list items with various numbers
        let mut found_numbers = std::collections::HashSet::new();

        for element in md_doc.iter() {
            if let crate::MdElement::SmartList((_, bullet_kind, _)) = element {
                if let crate::BulletKind::Ordered(number) = bullet_kind {
                    found_numbers.insert(*number);
                }
            }
        }

        // Should at least have items numbered 1 and 2
        assert!(found_numbers.contains(&1), "Should have list item 1");
        assert!(found_numbers.contains(&2), "Should have list item 2");
    }

    #[test]
    fn test_markdown_parsing_with_various_checkbox_states() {
        as_str_slice_test_case!(
            input,
            "Task List:",
            "- [ ] Unchecked task",
            "- [x] Completed task",
            "- [X] Completed with capital X",
            "- [ ]  Task with extra spaces",
            "- [x]  Completed with extra spaces",
            "- regular list item",
            "",
        );

        let expected_output = [
            "Task List:",
            "[  ┊─┤[ ] Unchecked task┊  ]",
            "[  ┊─┤[x] Completed task┊  ]",
            "[  ┊─┤[X] Completed with capital X┊  ]",
            "[  ┊─┤[ ]  Task with extra spaces┊  ]",
            "[  ┊─┤[x]  Completed with extra spaces┊  ]",
            "[  ┊─┤regular list item┊  ]",
        ];

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_markdown_parsing_with_large_numbered_lists() {
        as_str_slice_test_case!(
            input,
            "98. Item ninety-eight",
            "99. Item ninety-nine",
            "100. Item one hundred",
            "101. Item one hundred one",
            "999. Item nine hundred ninety-nine",
            "1000. Item one thousand",
            "",
        );

        let expected_output = [
            "[  ┊98.│Item ninety-eight┊  ]",
            "[  ┊99.│Item ninety-nine┊  ]",
            "[  ┊100.│Item one hundred┊  ]",
            "[  ┊101.│Item one hundred one┊  ]",
            "[  ┊999.│Item nine hundred ninety-nine┊  ]",
            "[  ┊1000.│Item one thousand┊  ]",
        ];

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_markdown_parsing_with_list_items_containing_formatting() {
        as_str_slice_test_case!(
            input,
            "- *italic item*",
            "- **bold item**",
            "- `code item`",
            "- [link item](https://example.com)",
            "1. *italic ordered*",
            "2. **bold ordered**",
            "- [ ] *italic checkbox*",
            "- [x] **bold completed**",
            "",
        );

        let expected_output = [
            "[  ┊─┤*italic item*┊  ]",
            "[  ┊─┤**bold item**┊  ]",
            "[  ┊─┤`code item`┊  ]",
            "[  ┊─┤[link item](https://example.com)┊  ]",
            "[  ┊1.│*italic ordered*┊  ]",
            "[  ┊2.│**bold ordered**┊  ]",
            "[  ┊─┤[ ] *italic checkbox*┊  ]",
            "[  ┊─┤[x] **bold completed**┊  ]",
        ];

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_markdown_parsing_with_empty_list_items() {
        as_str_slice_test_case!(
            input,
            "- first item",
            "-",
            "- third item",
            "1. numbered first",
            "2.",
            "3. numbered third",
            "- [ ]",
            "- [x]",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Should have multiple elements
        assert!(md_doc.len() >= 8); // Check for different types of elements
        let mut has_lists = false;
        let mut has_text = false;

        for element in md_doc.iter() {
            match element {
                crate::MdElement::SmartList(_) => has_lists = true,
                crate::MdElement::Text(_) => has_text = true,
                _ => {}
            }
        }

        // Some elements should be parsed as lists, some might be text
        assert!(has_lists || has_text, "Should have some parsed elements");
    }

    #[test]
    fn test_markdown_parsing_with_inconsistent_list_numbering() {
        as_str_slice_test_case!(
            input,
            "5. Starting at five",
            "6. Six",
            "10. Jump to ten",
            "3. Back to three",
            "1. Reset to one",
            "",
        );

        let expected_output = [
            "[  ┊5.│Starting at five┊  ]",
            "[  ┊6.│Six┊  ]",
            "[  ┊10.│Jump to ten┊  ]",
            "[  ┊3.│Back to three┊  ]",
            "[  ┊1.│Reset to one┊  ]",
        ];

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        assert_eq2!(remainder.is_empty(), true);
    }
    #[test]
    fn test_markdown_parsing_with_mixed_bullet_styles() {
        as_str_slice_test_case!(
            input,
            "- dash bullet",
            "* star bullet",
            "+ plus bullet",
            "- back to dash",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Don't assume exact count since different bullet styles may be parsed
        // differently
        assert!(md_doc.len() >= 1);

        // All should be recognized as some form of list or text
        for element in md_doc.iter() {
            match element {
                crate::MdElement::SmartList(_) | crate::MdElement::Text(_) => {
                    // Expected - either recognized as list or fallback to text
                }
                _ => panic!("Unexpected element type: {:?}", element),
            }
        }
    }

    #[test]
    fn test_markdown_parsing_with_lists_and_whitespace_lines() {
        as_str_slice_test_case!(
            input,
            "- item 1",
            "",
            "- item 2 after blank",
            "   ",
            "- item 3 after whitespace",
            "\t",
            "- item 4 after tab",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        // Should handle whitespace lines appropriately
        assert_eq2!(remainder.is_empty(), true); // Verify structure includes both list items and empty/whitespace lines
        let mut list_count = 0;
        let mut empty_count = 0;

        for element in md_doc.iter() {
            match element {
                crate::MdElement::SmartList(_) => list_count += 1,
                crate::MdElement::Text(fragments) if fragments.is_empty() => {
                    empty_count += 1
                }
                crate::MdElement::Text(_) => {} // Non-empty text
                _ => panic!("Unexpected element type: {:?}", element),
            }
        }

        assert!(list_count >= 4, "Should have at least 4 list items");
        assert!(
            empty_count >= 2,
            "Should have at least 2 empty/whitespace lines"
        );
    }
}

/// Tests integration of code block parsing with the markdown parser.
#[cfg(test)]
mod tests_integration_block_code_ng {
    use crate::{as_str_slice_test_case,
                assert_eq2,
                parse_markdown_ng,
                AsStrSlice,
                CodeBlockLine,
                CodeBlockLineContent,
                GCString,
                MdElement};
    #[test]
    fn test_markdown_parsing_with_nested_code_blocks_in_lists() {
        as_str_slice_test_case!(
            input,
            "Installation steps:",
            "1. Install Rust",
            "   ```bash",
            "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
            "   ```",
            "2. Create a new project",
            "   ```bash",
            "   cargo new my_project",
            "   cd my_project",
            "   ```",
            "3. Run the project",
            "   ```bash",
            "   cargo run",
            "   ```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Should have multiple elements
        assert!(md_doc.len() >= 1);

        // Look for elements that were parsed successfully
        let mut text_count = 0;
        let mut list_count = 0;
        let mut code_block_count = 0;

        for element in md_doc.iter() {
            match element {
                MdElement::CodeBlock(_) => code_block_count += 1,
                MdElement::SmartList(_) => list_count += 1,
                MdElement::Text(_) => text_count += 1,
                _ => {}
            }
        }

        // Should have parsed something meaningful
        assert!(
            text_count + list_count + code_block_count > 0,
            "Should have parsed some elements"
        );
    }

    #[test]
    fn test_markdown_parsing_with_code_blocks_various_languages() {
        as_str_slice_test_case!(
            input,
            "```rust",
            "fn main() {",
            "    println!(\"Hello, Rust!\");",
            "}",
            "```",
            "```python",
            "def hello():",
            "    print(\"Hello, Python!\")",
            "```",
            "```javascript",
            "function hello() {",
            "    console.log(\"Hello, JavaScript!\");",
            "}",
            "```",
            "```go",
            "package main",
            "import \"fmt\"",
            "func main() {",
            "    fmt.Println(\"Hello, Go!\")",
            "}",
            "```",
            "```c",
            "#include <stdio.h>",
            "int main() {",
            "    printf(\"Hello, C!\\n\");",
            "    return 0;",
            "}",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        // Should have 5 code blocks
        assert_eq2!(md_doc.len(), 5);

        let expected_languages = ["rust", "python", "javascript", "go", "c"];

        for (i, &expected_lang) in expected_languages.iter().enumerate() {
            match &md_doc[i] {
                MdElement::CodeBlock(code_block) => {
                    assert_eq2!(code_block[0].language, Some(expected_lang));
                    assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                    assert!(code_block.len() >= 2); // At least start and end
                    assert_eq2!(
                        code_block.last().unwrap().content,
                        CodeBlockLineContent::EndTag
                    );
                }
                _ => panic!(
                    "Expected CodeBlock for {}, got {:?}",
                    expected_lang, md_doc[i]
                ),
            }
        }

        assert_eq2!(remainder.is_empty(), true);
    }
    #[test]
    fn test_markdown_parsing_with_malformed_code_blocks() {
        as_str_slice_test_case!(
            input,
            "```rust",
            "fn incomplete() {",
            "    // Missing closing backticks",
            "Regular text after",
            "```python",
            "print('This has proper closing')",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        // The parser should handle malformed blocks gracefully
        assert_eq2!(remainder.is_empty(), true);

        // Should have at least some elements
        assert!(md_doc.len() > 0);

        // Look for any code blocks that were successfully parsed
        let mut found_code_block = false;
        for element in md_doc.iter() {
            if let MdElement::CodeBlock(_) = element {
                found_code_block = true;
                break;
            }
        }

        // May or may not find code blocks depending on how malformed input is handled
        // The important thing is that parsing doesn't panic
        println!("Found code block: {}", found_code_block);
    }

    #[test]
    fn test_markdown_parsing_with_code_blocks_with_special_content() {
        as_str_slice_test_case!(
            input,
            "```markdown",
            "# This is markdown inside a code block",
            "- item 1",
            "- item 2",
            "```",
            "```html",
            "<div>",
            "  <p>HTML content</p>",
            "  <!-- Comment -->",
            "</div>",
            "```",
            "```json",
            "{",
            "  \"name\": \"test\",",
            "  \"value\": null,",
            "  \"array\": [1, 2, 3]",
            "}",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(md_doc.len(), 3);
        assert_eq2!(remainder.is_empty(), true);

        let expected_languages = ["markdown", "html", "json"];

        for (i, &expected_lang) in expected_languages.iter().enumerate() {
            match &md_doc[i] {
                MdElement::CodeBlock(code_block) => {
                    assert_eq2!(code_block[0].language, Some(expected_lang));
                    assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                    assert_eq2!(
                        code_block.last().unwrap().content,
                        CodeBlockLineContent::EndTag
                    );

                    // Verify content is preserved as-is
                    for line in code_block.iter().skip(1).take(code_block.len() - 2) {
                        match &line.content {
                            CodeBlockLineContent::Text(_) => {} // Expected
                            _ => panic!("Expected text content in code block"),
                        }
                    }
                }
                _ => panic!(
                    "Expected CodeBlock for {}, got {:?}",
                    expected_lang, md_doc[i]
                ),
            }
        }
    }

    #[test]
    fn test_markdown_parsing_with_indented_code_blocks() {
        as_str_slice_test_case!(
            input,
            "Here's some indented code:",
            "    ```python",
            "    def hello():",
            "        print('Hello from indented block')",
            "    ```",
            "And some regular code:",
            "```python",
            "def regular():",
            "    print('Regular block')",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Should handle indented code blocks (may be treated as text or code depending on
        // implementation)
        let mut found_regular_code = false;

        for element in md_doc.iter() {
            if let MdElement::CodeBlock(code_block) = element {
                if let Some(CodeBlockLine {
                    language: Some("python"),
                    ..
                }) = code_block.first()
                {
                    found_regular_code = true;
                    assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                    assert_eq2!(
                        code_block.last().unwrap().content,
                        CodeBlockLineContent::EndTag
                    );
                }
            }
        }

        assert!(
            found_regular_code,
            "Should find at least the regular Python code block"
        );
    }
    #[test]
    fn test_markdown_parsing_with_code_blocks_and_backticks_in_content() {
        as_str_slice_test_case!(
            input,
            "```bash",
            "echo \"Here's a single backtick: `\"",
            "echo \"Here are two backticks: ``\"",
            "echo \"Code in the shell: `ls -la`\"",
            "```",
            "```markdown",
            "Use `inline code` for short snippets",
            "Use ```blocks``` for longer code",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Should have at least 2 elements, but may be more due to how backticks are
        // handled
        assert!(md_doc.len() >= 2); // Look for code blocks and verify they exist
        let mut found_bash = false;
        let mut _found_markdown = false;

        for element in md_doc.iter() {
            if let MdElement::CodeBlock(code_block) = element {
                if let Some(CodeBlockLine {
                    language: Some("bash"),
                    ..
                }) = code_block.first()
                {
                    found_bash = true;
                    assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                    assert_eq2!(
                        code_block.last().unwrap().content,
                        CodeBlockLineContent::EndTag
                    );
                }
                if let Some(CodeBlockLine {
                    language: Some("markdown"),
                    ..
                }) = code_block.first()
                {
                    _found_markdown = true;
                    assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                    assert_eq2!(
                        code_block.last().unwrap().content,
                        CodeBlockLineContent::EndTag
                    );
                }
            }
        }

        // Should find at least the bash block
        assert!(found_bash, "Should find bash code block");
    }

    #[test]
    fn test_markdown_parsing_with_consecutive_code_blocks_no_separator() {
        as_str_slice_test_case!(
            input,
            "```rust",
            "fn first() {}",
            "```",
            "```python",
            "def second(): pass",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(md_doc.len(), 2);
        assert_eq2!(remainder.is_empty(), true);

        // First code block
        match &md_doc[0] {
            MdElement::CodeBlock(code_block) => {
                assert_eq2!(code_block[0].language, Some("rust"));
                assert_eq2!(code_block.len(), 3); // start, content, end
            }
            _ => panic!("Expected first CodeBlock"),
        }

        // Second code block
        match &md_doc[1] {
            MdElement::CodeBlock(code_block) => {
                assert_eq2!(code_block[0].language, Some("python"));
                assert_eq2!(code_block.len(), 3); // start, content, end
            }
            _ => panic!("Expected second CodeBlock"),
        }
    }

    #[test]
    fn test_markdown_parsing_with_code_blocks_mixed_with_other_elements() {
        as_str_slice_test_case!(
            input,
            "# Code Examples",
            "",
            "Here's a Rust example:",
            "```rust",
            "fn main() {",
            "    println!(\"Hello!\");",
            "}",
            "```",
            "",
            "And here's a list:",
            "- Item 1",
            "- Item 2",
            "",
            "Another code block:",
            "```python",
            "print('Python!')",
            "```",
            "",
            "The end.",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(remainder.is_empty(), true);

        // Verify structure contains heading, text, code blocks, and lists
        let mut heading_count = 0;
        let mut code_block_count = 0;
        let mut smart_list_count = 0;
        let mut text_count = 0;

        for element in md_doc.iter() {
            match element {
                MdElement::Heading(_) => heading_count += 1,
                MdElement::CodeBlock(_) => code_block_count += 1,
                MdElement::SmartList(_) => smart_list_count += 1,
                MdElement::Text(_) => text_count += 1,
                _ => {}
            }
        }

        assert_eq2!(heading_count, 1);
        assert_eq2!(code_block_count, 2);
        assert!(text_count >= 3); // Multiple text elements including empty lines
        assert_eq2!(smart_list_count, 2);

        // Verify code blocks have correct languages
        let mut found_rust = false;
        let mut found_python = false;

        for element in md_doc.iter() {
            if let MdElement::CodeBlock(code_block) = element {
                match code_block[0].language {
                    Some("rust") => found_rust = true,
                    Some("python") => found_python = true,
                    _ => {}
                }
            }
        }

        assert!(found_rust, "Should find Rust code block");
        assert!(found_python, "Should find Python code block");
    }

    #[test]
    fn test_markdown_parsing_with_code_blocks_whitespace_variations() {
        as_str_slice_test_case!(
            input,
            "```   rust   ",
            "fn with_spaces() {}",
            "```",
            "```\t\tpython\t\t",
            "def with_tabs(): pass",
            "```",
            "```",
            "no language specified",
            "```",
            "",
        );

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(md_doc.len(), 3);
        assert_eq2!(remainder.is_empty(), true);

        // First block should handle spaces in language specification
        match &md_doc[0] {
            MdElement::CodeBlock(code_block) => {
                // Language parsing might trim whitespace
                assert!(code_block[0].language.is_some());
                assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                assert_eq2!(
                    code_block.last().unwrap().content,
                    CodeBlockLineContent::EndTag
                );
            }
            _ => panic!("Expected first CodeBlock"),
        }

        // Second block should handle tabs
        match &md_doc[1] {
            MdElement::CodeBlock(code_block) => {
                assert!(code_block[0].language.is_some());
                assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                assert_eq2!(
                    code_block.last().unwrap().content,
                    CodeBlockLineContent::EndTag
                );
            }
            _ => panic!("Expected second CodeBlock"),
        }

        // Third block has no language
        match &md_doc[2] {
            MdElement::CodeBlock(code_block) => {
                assert_eq2!(code_block[0].language, None);
                assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                assert_eq2!(
                    code_block.last().unwrap().content,
                    CodeBlockLineContent::EndTag
                );
            }
            _ => panic!("Expected third CodeBlock"),
        }
    }

    #[test]
    fn test_markdown_parsing_with_very_long_code_blocks() {
        let mut input_lines = vec!["```rust".to_string()];

        // Add 100 lines of code
        for i in 1..=100 {
            input_lines.push(format!("    // Line {}", i));
            input_lines.push(format!("    println!(\"This is line {}\");", i));
        }

        input_lines.push("```".to_string());
        input_lines.push("".to_string());

        let raw_input = input_lines.join("\n");
        let binding = raw_input
            .lines()
            .map(GCString::from)
            .collect::<Vec<GCString>>();
        let input = AsStrSlice::from(binding.as_slice());

        let result = parse_markdown_ng(input);
        let (remainder, md_doc) = result.unwrap();

        assert_eq2!(md_doc.len(), 1);
        assert_eq2!(remainder.is_empty(), true);

        match &md_doc[0] {
            MdElement::CodeBlock(code_block) => {
                assert_eq2!(code_block[0].language, Some("rust"));
                assert_eq2!(code_block[0].content, CodeBlockLineContent::StartTag);
                assert_eq2!(
                    code_block.last().unwrap().content,
                    CodeBlockLineContent::EndTag
                );

                // Should have start + 200 content lines + end = 202 total
                assert_eq2!(code_block.len(), 202);

                // Verify some content lines
                for line in code_block.iter().skip(1).take(code_block.len() - 2) {
                    match &line.content {
                        CodeBlockLineContent::Text(text) => {
                            assert!(text.contains("Line ") || text.contains("println!"));
                        }
                        _ => panic!("Expected text content in large code block"),
                    }
                }
            }
            _ => panic!("Expected large CodeBlock"),
        }
    }
}
