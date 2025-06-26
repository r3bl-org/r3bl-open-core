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
            list,
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
/// ## Overview
///
/// There are two main paths for parsing markdown in the R3BL TUI editor, from the
/// common source of truth, which is [`crate::EditorContent`], which uses
/// [`crate::sizing::VecEditorContentLines`] internally to store the data, which is just
/// an inline vec of [`crate::GCString`].
/// 1. NG parser path: Convert `&[GCString]` to [`AsStrSlice`] (🐇 no copy) ->
///    parse_markdown_ng
/// 2. Legacy parser path: &[crate::GCString] -> materialized string (🦥 full copy) ->
///    parse_markdown
///
/// ## Drop-in replacement
///
/// This function is a drop-in replacement for the legacy `parse_markdown` function,
/// with full compatibility for all edge cases.
///
/// ## Architectural features
///
/// ### Line advancement infrastructure
/// - Unified advancement via ensure_advance_with_parser
/// - Consistent edge case handling
/// - Robust last line processing
///
/// ### Input exhaustion detection
/// - Line-based detection (line_index >= lines.len())
/// - Processes all lines including trailing empty ones
/// - Complete input consumption guaranteed
///
/// ### Empty line parser
/// - Infrastructure-based approach (uses state detection in [mod@crate::as_str_slice].
/// - Leverages `ensure_advance_with_parser`.
/// - Simple, robust, maintainable.
///
/// ## Performance characteristics
/// - **Zero-allocation parsing**: Uses [`AsStrSlice`] for virtual array access without
///   copying.
/// - **Fast memory access**: Optimized for editor component real-time rendering
///   requirements.
/// - **Unicode-safe**: Full support for UTF-8 and multi-byte grapheme cluster segments.
/// - **Panic-free**: Robust handling of Unicode emoji and complex text without crashes.
/// - **Enhanced reliability**: Improved line advancement prevents infinite loops.
///
/// ## Data bridge architecture
/// The [`AsStrSlice`] input provides a crucial bridge between how data is stored in
/// memory by the editor and how the nom parser expects to access it:
/// - Takes output from [`str::lines()`] and creates a virtual array interface
/// - Generates synthetic newlines to maintain line boundaries
/// - Implements the [`nom::Input`] trait for seamless nom parser integration
/// - Enables byte-level parsing while preserving the editor's line-based data structure
///
/// ## Enhanced parser chain design
/// Uses [`many0(alt(...))`](nom) with parsers ordered by specificity. Each parser is
/// wrapped with [`AsStrSlice::ensure_advance_with_parser`] to prevent infinite loops and
/// handle line advancement automatically with enhanced edge case support.
///
/// ### Parser categories (in order of precedence)
/// - **Metadata**: Title, tags, authors, date (structured document properties)
/// - **Structure**: Headings (document hierarchy and navigation)
/// - **Content**: Smart lists, code blocks, empty lines, text (document body)
///
/// ### Critical parser ordering
/// The empty line parser **must** come before the text parser because:
/// 1. Text parser explicitly rejects empty input to prevent infinite loops
/// 2. Empty line parser handles completely empty lines that would otherwise be missed
/// 3. This ordering ensures all edge cases with trailing/consecutive empty lines work
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

/// Parse empty lines that the main text parser rejects with enhanced edge case handling.
///
/// ## Critical purpose
/// This parser handles the essential case of completely empty lines (no content) that
/// the main text parser ([`parse_line_text_advance_ng()`]) explicitly rejects to
/// prevent infinite loops. This function is **critical for ensuring that the NG parser
/// produces identical output to the legacy parser**, especially for edge cases involving
/// trailing empty lines.
///
/// ## Major architectural rewrite
/// This function underwent a **complete rewrite** to fix compatibility issues. The change
/// from custom advancement logic to infrastructure-based advancement was essential for
/// achieving true drop-in replacement compatibility.
///
/// ## Enhanced algorithm
/// The algorithm is remarkably simple and robust:
///
/// 1. Check if current line is completely empty (not whitespace-only).
/// 2. If empty: return success with empty fragments list.
/// 3. If not empty: return error (let other parsers handle).
/// 4. Let ensure_advance_with_parser handle ALL advancement logic.
///
/// ## Edge cases
///
/// ### Trailing empty lines
/// Input: `"Line 1\n\n\nLine 2\n\n"`
/// Lines: `["Line 1", "", "", "Line 2", ""]`
///
/// Infrastructure processes all lines
/// - Handles lines 0-4 including final empty line
/// - Output: 5 elements (matches legacy parser exactly)
///
/// ### Only newlines input
/// Input: `\n\n\n`
/// Lines: `["", "", ""]`
///
/// Infrastructure handles all three empty lines
/// - Processes each empty line → empty `Text([])`
/// - Output: 3 empty `Text([])` elements (matches legacy parser)
///
/// ### Complex mixed content
/// Input: `# Title\n\nContent\n\n## Section\n\n`
///
/// Every empty line consistently produces `Text([])` element
///
/// ## Integration with ensure_advance_with_parser
/// This function demonstrates the correct pattern for using `ensure_advance_with_parser`:
/// 1. **Parser checks**: Determine if it can handle the current input.
/// 2. **Success case**: Create appropriate output, return same input.
/// 3. **Infrastructure handles**: All advancement, state management, edge cases.
/// 4. **Consistent behavior**: Across all edge cases and line sequences.
///
/// ## Critical parser ordering
/// This parser **must** come before the general text parser
/// ([`parse_line_text_advance_ng`]) in the parser chain because:
/// - Text parser explicitly rejects empty input to prevent infinite loops.
/// - Empty line parser handles the rejected empty lines.
/// - This ordering ensures complete input coverage.
pub fn parse_line_empty_advance_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    // Use the existing line advancement system instead of custom logic
    input.ensure_advance_with_parser(&mut |input: AsStrSlice<'a>| {
        // Only handle completely empty lines, not whitespace-only lines.
        let current_line = input.extract_to_line_end();
        if !current_line.is_empty() {
            return Err(NErr::Error(NError::new(input, NErrorKind::Tag)));
        }

        // Create empty fragments for empty lines.
        let fragments = list![];

        // Return success with the same input (let ensure_advance_with_parser handle
        // advancement).
        Ok((input, fragments))
    })
}
