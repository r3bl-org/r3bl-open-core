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

use crate::{constants,
            convert_into_code_block_lines,
            md_parser::constants::{AUTHORS, DATE, TAGS, TITLE},
            parse_block_code,
            parse_block_heading_opt_eol,
            parse_block_markdown_text_with_or_without_new_line,
            parse_block_smart_list,
            parse_csv_opt_eol,
            parse_unique_kv_opt_eol,
            BulletKind,
            CodeBlockLine,
            CodeBlockLineContent,
            GCString,
            Lines,
            List,
            MdBlock,
            MdDocument,
            MdLineFragment};

// XMARK: Main Markdown parser entry point

/// This is the main parser entry point, aka, the root parser. It takes a string slice and
/// if it can be parsed, returns a [MdDocument] that represents the parsed Markdown.
///
/// 1. [crate::MdLineFragments] roughly corresponds to a line of parsed text.
/// 2. [MdDocument] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [MdDocument] corresponds to a block of Markdown [MdBlock], which can
/// be one of the following variants:
/// 1. Metadata title. The parsers in [crate::parse_metadata_kv] file handle this.
/// 2. Metadata tags. The parsers in [crate::parse_metadata_kcsv] file handle this.
/// 3. Heading (which contains a [crate::HeadingLevel] & [crate::MdLineFragments]).
/// 4. Smart ordered & unordered list (which itself contains a [Vec] of
///    [crate::MdLineFragments]. The parsers in [mod@parse_block_smart_list] file handle
///    this.
/// 5. Code block (which contains string slices of the language & code). The parsers in
///    [mod@parse_block_code] file handle this.
/// 6. line (which contains a [crate::MdLineFragments]). The parsers in
///    [mod@crate::fragment] handle this.
#[rustfmt::skip]
pub fn parse_markdown_str(input: &str) -> IResult<&str, MdDocument<'_>> {
    let (input, output) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_value,                                  MdBlock::Title),
            map(parse_tags_list,                                    MdBlock::Tags),
            map(parse_authors_list,                                 MdBlock::Authors),
            map(parse_date_value,                                   MdBlock::Date),
            map(parse_block_heading_opt_eol,                        MdBlock::Heading),
            map(parse_block_smart_list,                             MdBlock::SmartList),
            map(parse_block_code,                                   MdBlock::CodeBlock),
            map(parse_block_markdown_text_with_or_without_new_line, MdBlock::Text),
        )),
    ).parse(input)?;

    let it = List::from(output);
    Ok((input, it))
}

/// This is the main parser entry point, aka, the root parser. It takes a string slice and
/// if it can be parsed, returns a [MdDocument] that represents the parsed Markdown.
///
/// 1. [crate::MdLineFragments] roughly corresponds to a line of parsed text.
/// 2. [MdDocument] contains all the blocks that are parsed from a Markdown string slice.
///
/// Each item in this [MdDocument] corresponds to a block of Markdown [MdBlock], which can
/// be one of the following variants:
/// 1. Metadata title. The parsers in [crate::parse_metadata_kv] file handle this.
/// 2. Metadata tags. The parsers in [crate::parse_metadata_kcsv] file handle this.
/// 3. Heading (which contains a [crate::HeadingLevel] & [crate::MdLineFragments]).
/// 4. Smart ordered & unordered list (which itself contains a [Vec] of
///    [crate::MdLineFragments]. The parsers in [mod@parse_block_smart_list] file handle
///    this).
/// 5. Code block (which contains string slices of the language & code). The parsers in
///    [mod@parse_block_code] file handle this.
/// 6. line (which contains a [crate::MdLineFragments]). The parsers in
///    [mod@crate::fragment] handle this.
pub fn parse_markdown<'a>(input: &'a [GCString]) -> IResult<String, MdDocument<'a>> {
    let mut doc = Vec::new();
    let mut rem_string = String::new();

    // Handle empty input
    if input.is_empty() {
        return Ok((rem_string, List::from(doc)));
    }

    // Process the input line by line with look-ahead capability
    let mut line_index = 0;
    while line_index < input.len() {
        let line = &input[line_index];
        let line_str = &line.string;

        // Check if this is the start of a code block
        if line_str.starts_with(constants::CODE_BLOCK_START_PARTIAL) {
            // Try to parse as a code block
            if let Some((code_block, lines_consumed)) =
                try_parse_code_block(&input[line_index..])
            {
                // Add the code block to the document
                doc.push(MdBlock::CodeBlock(code_block));

                // Advance the line index
                line_index += lines_consumed;
            } else {
                // No valid code block found - treat opening line as a regular line
                process_regular_line(line_str, &mut doc, &mut rem_string);
                line_index += 1;
            }
        }
        // Check if this is the start of a smart list
        else if line_str.starts_with(constants::UNORDERED_LIST_PREFIX)
            || is_ordered_list_prefix(line_str)
        {
            // Look ahead to find the end of the smart list
            if let Some((smart_list, lines_consumed)) =
                try_parse_smart_list(&input[line_index..])
            {
                // Add the smart list to the document
                doc.push(MdBlock::SmartList(smart_list));

                // Advance the line index
                line_index += lines_consumed;
            } else {
                // Process as a regular line
                process_regular_line(line_str, &mut doc, &mut rem_string);
                line_index += 1;
            }
        }
        // Regular line processing
        else {
            process_regular_line(line_str, &mut doc, &mut rem_string);
            line_index += 1;
        }
    }

    // Return the accumulated remainders and the document
    Ok((rem_string, List::from(doc)))
}

/// Attempts to parse a code block starting at the given line index.
///
/// If a valid code block is found (with opening and closing tags), returns:
///   - The parsed code block
///   - Number of lines consumed
///
/// If no valid code block is found (no closing tag), returns None.
fn try_parse_code_block<'a>(
    lines: &'a [GCString],
) -> Option<(List<CodeBlockLine<'a>>, usize)> {
    if lines.is_empty() {
        return None;
    }

    // First line must start with code block marker
    let first_line = &lines[0].string;
    if !first_line.starts_with(constants::CODE_BLOCK_START_PARTIAL) {
        return None;
    }

    // Extract language from the first line
    let lang = if first_line.len() > constants::CODE_BLOCK_START_PARTIAL.len() {
        Some(&first_line[constants::CODE_BLOCK_START_PARTIAL.len()..])
    } else {
        None
    };

    // Look for a closing tag
    let mut code_block_end_index = 0;
    let mut has_closing_tag = false;

    for (idx, potential_end_line) in lines[1..].iter().enumerate() {
        if potential_end_line
            .string
            .starts_with(constants::CODE_BLOCK_END)
        {
            has_closing_tag = true;
            code_block_end_index = 1 + idx; // +1 because we start from index 1
            break;
        }
    }

    // If no closing tag was found, this isn't a valid code block
    if !has_closing_tag {
        return None;
    }

    // Collect content lines (excluding start and end tags)
    let mut content_lines = Vec::new();
    for i in 1..code_block_end_index {
        content_lines.push(lines[i].string.as_str());
    }

    // Create code block
    let code_block = convert_into_code_block_lines(lang, content_lines);

    // Return code block and number of lines consumed (including start and end tags)
    Some((code_block, code_block_end_index + 1))
}

// Helper function to collect a code block
fn collect_code_block<'a>(
    lines: &'a [GCString],
) -> (List<CodeBlockLine<'a>>, usize, bool) {
    if lines.is_empty() {
        return (List::new(), 0, false);
    }

    // Parse the language from the first line
    let first_line = &lines[0].string;
    let lang = if first_line.len() > constants::CODE_BLOCK_START_PARTIAL.len() {
        Some(&first_line[constants::CODE_BLOCK_START_PARTIAL.len()..])
    } else {
        None
    };

    // Collect lines until we find the end tag or run out of lines
    let mut content_lines = Vec::new();
    let mut lines_consumed = 1; // Already consumed the first line
    let mut has_closing_tag = false;

    for line in &lines[1..] {
        lines_consumed += 1;

        // Check if this is the end of the code block
        if line.string.starts_with(constants::CODE_BLOCK_END) {
            has_closing_tag = true;
            break;
        }

        // Add the line to the content
        content_lines.push(line.string.as_str());
    }

    // Create the code block structure manually to handle unclosed blocks properly
    let mut code_block = List::new();

    // Add the start tag
    code_block.push(CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::StartTag,
    });

    // Add content lines
    for line in &content_lines {
        code_block.push(CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::Text(line),
        });
    }

    // Only add the end tag if we found a closing tag
    if has_closing_tag {
        code_block.push(CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::EndTag,
        });
    }

    (code_block, lines_consumed, has_closing_tag)
}

// Helper function to check if a line is an ordered list prefix
fn is_ordered_list_prefix(line: &str) -> bool {
    if let Some(pos) = line.find(constants::ORDERED_LIST_PARTIAL_PREFIX) {
        // Check if the characters before the ". " are digits
        let prefix = &line[..pos];
        prefix.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    }
}

// Helper function to process a regular line
fn process_regular_line<'a>(
    line_str: &'a str,
    doc: &mut Vec<MdBlock<'a>>,
    rem_string: &mut String,
) {
    // Parse the line using the many0 block of code
    let res = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_value, MdBlock::Title),
            map(parse_tags_list, MdBlock::Tags),
            map(parse_authors_list, MdBlock::Authors),
            map(parse_date_value, MdBlock::Date),
            map(parse_block_heading_opt_eol, MdBlock::Heading),
            // Skip smart list and code block parsers as we handle them separately
            map(
                parse_block_markdown_text_with_or_without_new_line,
                MdBlock::Text,
            ),
        )),
    )
    .parse(line_str);

    match res {
        Ok((line_rem, line_output)) => {
            // Add the parsed output to the document
            for block in line_output {
                doc.push(block);
            }

            // Handle non-empty remainder
            if !line_rem.is_empty() {
                rem_string.push_str(line_rem);
                rem_string.push('\n');
            }
        }
        Err(_) => {
            // Add the entire line to the remainder
            rem_string.push_str(line_str);
            rem_string.push('\n');
        }
    }
}

fn try_parse_smart_list<'a>(
    lines: &'a [GCString],
) -> Option<((Lines<'a>, BulletKind, usize), usize)> {
    if lines.is_empty() {
        return None;
    }

    // Determine the initial list type and indentation from the first line
    let first_line = &lines[0].string;
    let (bullet_kind, initial_indent) = identify_list_type(first_line)?;

    // Prepare to collect parsed fragments for all list items
    let mut parsed_fragments = Lines::new();
    let mut lines_consumed = 0;

    // Keep track of the current item's fragments and indentation
    let mut current_item_fragments = Vec::new();
    let mut current_indent = initial_indent;

    // Process each line
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i].string;

        // Check if this line continues the list
        if line.is_empty() {
            // Empty line - could be a separator between list items or end of list
            if i + 1 < lines.len() {
                // Look ahead to see if the next line continues the list
                let next_line = &lines[i + 1].string;
                if is_list_continuation(next_line, initial_indent, &bullet_kind) {
                    // This empty line is part of the list - add it as plain text to
                    // current item
                    if !current_item_fragments.is_empty() {
                        current_item_fragments.push(MdLineFragment::Plain(""));
                        lines_consumed += 1;
                        i += 1;
                        continue;
                    }
                } else {
                    // This empty line marks the end of the list
                    break;
                }
            } else {
                // Last line is empty - end of list
                break;
            }
        }

        // Check if this line starts a new list item at the same level
        if is_new_list_item(line, initial_indent, &bullet_kind) {
            // If we have fragments from a previous item, add them to the result
            if !current_item_fragments.is_empty() {
                parsed_fragments.push(List::from(current_item_fragments));
                current_item_fragments = Vec::new();
            }

            // Parse this new list item
            let (item_fragments, item_indent) = parse_list_item_line(line, bullet_kind)?;
            current_item_fragments = item_fragments;
            current_indent = item_indent;
            lines_consumed += 1;
            i += 1;
        }
        // Check if this line is a continuation of the current list item (indented line)
        else if is_continuation_line(line, current_indent) {
            // Parse as continuation text
            let content = extract_continuation_content(line, current_indent);
            current_item_fragments.push(MdLineFragment::Plain(content));
            lines_consumed += 1;
            i += 1;
        }
        // This line is not part of the list - end of list
        else {
            break;
        }
    }

    // Add the last item if we have one
    if !current_item_fragments.is_empty() {
        parsed_fragments.push(List::from(current_item_fragments));
    }

    // If we didn't parse any lines or items, it's not a valid list
    if lines_consumed == 0 || parsed_fragments.is_empty() {
        return None;
    }

    // Return the parsed list, its type, indentation, and number of lines consumed
    Some((
        (parsed_fragments, bullet_kind, initial_indent),
        lines_consumed,
    ))
}

// Helper function to identify list type from the first line
fn identify_list_type(line: &str) -> Option<(BulletKind, usize)> {
    // Count leading whitespace as indent
    let indent = count_leading_whitespace(line);

    // Check for unordered list
    if line[indent..].starts_with(constants::UNORDERED_LIST_PREFIX) {
        return Some((BulletKind::Unordered, indent));
    }

    // Check for ordered list
    let non_whitespace = &line[indent..];
    if let Some(pos) = non_whitespace.find(constants::ORDERED_LIST_PARTIAL_PREFIX) {
        if pos > 0 {
            // Check if the characters before ". " are digits
            let prefix = &non_whitespace[..pos];
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(number) = prefix.parse::<usize>() {
                    return Some((BulletKind::Ordered(number), indent));
                }
            }
        }
    }

    None
}

// Count leading whitespace in a string
fn count_leading_whitespace(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

// Check if a line is the start of a new list item at the same level
fn is_new_list_item(line: &str, initial_indent: usize, bullet_kind: &BulletKind) -> bool {
    let indent = count_leading_whitespace(line);

    // Must have the same indentation as the first item
    if indent != initial_indent {
        return false;
    }

    let non_whitespace = &line[indent..];

    match bullet_kind {
        BulletKind::Unordered => {
            non_whitespace.starts_with(constants::UNORDERED_LIST_PREFIX)
        }
        BulletKind::Ordered(_) => {
            // For ordered lists, check for any number followed by ". "
            if let Some(pos) = non_whitespace.find(constants::ORDERED_LIST_PARTIAL_PREFIX)
            {
                if pos > 0 {
                    let prefix = &non_whitespace[..pos];
                    prefix.chars().all(|c| c.is_ascii_digit())
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}

// Check if a line is a continuation of the current list item
fn is_continuation_line(line: &str, current_indent: usize) -> bool {
    let indent = count_leading_whitespace(line);

    // Continuation lines must have more indentation than the list item marker
    indent > current_indent
}

// Check if the next line after an empty line continues the list
fn is_list_continuation(
    line: &str,
    initial_indent: usize,
    bullet_kind: &BulletKind,
) -> bool {
    // Check if it's a new list item
    if is_new_list_item(line, initial_indent, bullet_kind) {
        return true;
    }

    // Check if it's an indented continuation line
    let indent = count_leading_whitespace(line);
    indent > initial_indent
}

// Parse a list item line into fragments
fn parse_list_item_line<'a>(
    line: &'a str,
    bullet_kind: BulletKind,
) -> Option<(Vec<MdLineFragment<'a>>, usize)> {
    let indent = count_leading_whitespace(line);
    let non_whitespace = &line[indent..];

    let mut fragments = Vec::new();
    let (content_start, item_number) = match bullet_kind {
        BulletKind::Unordered => {
            // Add the bullet fragment
            fragments.push(MdLineFragment::UnorderedListBullet {
                indent,
                is_first_line: true,
            });

            // Skip the "- " prefix
            (constants::UNORDERED_LIST_PREFIX.len(), 0)
        }
        BulletKind::Ordered(start_number) => {
            // Find where the content starts after the number and ". "
            if let Some(pos) = non_whitespace.find(constants::ORDERED_LIST_PARTIAL_PREFIX)
            {
                let prefix = &non_whitespace[..pos];
                if let Ok(number) = prefix.parse::<usize>() {
                    // Add the bullet fragment
                    fragments.push(MdLineFragment::OrderedListBullet {
                        indent,
                        number,
                        is_first_line: true,
                    });

                    // Skip the number and ". " prefix
                    (pos + constants::ORDERED_LIST_PARTIAL_PREFIX.len(), number)
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    };

    // Add the content as plain text
    if content_start < non_whitespace.len() {
        let content = &non_whitespace[content_start..];
        fragments.push(MdLineFragment::Plain(content));
    }

    // Return the fragments and the indent
    Some((fragments, indent))
}

// Extract content from a continuation line
fn extract_continuation_content(line: &str, item_indent: usize) -> &str {
    let total_indent = count_leading_whitespace(line);

    // The content starts after the indentation
    &line[total_indent..]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2,
                convert_into_code_block_lines,
                list,
                BulletKind,
                GCString,
                HeadingData,
                HeadingLevel,
                HyperlinkData,
                MdLineFragment};

    #[test]
    fn test_no_line() {
        let input = [GCString::new("Something")];
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(
            blocks[0],
            MdBlock::Text(list![MdLineFragment::Plain("Something")])
        );
    }

    #[test]
    fn test_one_line() {
        let input = [GCString::new("Something")];
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("remainder: {remainder:?}");
        println!("blocks: {blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(
            blocks[0],
            MdBlock::Text(list![MdLineFragment::Plain("Something")])
        );
    }

    #[test]
    fn test_parse_markdown_with_invalid_text_in_heading() {
        let input = [
            GCString::new("# LINE 1"),
            GCString::new(""),
            GCString::new("##% LINE 2 FOO_BAR:"),
            GCString::new(""),
        ];
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{remainder:?}");
        println!("\nblocks:\n{blocks:#?}");
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 3);
        assert_eq2!(
            blocks[0],
            MdBlock::Heading(HeadingData {
                heading_level: HeadingLevel { level: 1 },
                text: "LINE 1",
            })
        );
        assert_eq2!(
            blocks[1],
            MdBlock::Text(list![]), // Empty line.
        );
        assert_eq2!(
            blocks[2],
            MdBlock::Text(list![
                MdLineFragment::Plain("##% LINE 2 FOO"),
                MdLineFragment::Plain("_"),
                MdLineFragment::Plain("BAR:"),
            ])
        );
    }

    #[test]
    fn test_parse_markdown_single_line_plain_text() {
        let input = [
            GCString::new("_this should not be italic"),
            GCString::new(""),
        ];
        let (remainder, blocks) = parse_markdown(&input).unwrap();
        println!("\nremainder:\n{remainder:?}");
        println!("\nblocks:\n{blocks:?}");
        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 1);
        assert_eq2!(
            blocks[0],
            MdBlock::Text(list![
                MdLineFragment::Plain("_"),
                MdLineFragment::Plain("this should not be italic"),
            ])
        );
    }

    #[test]
    fn test_parse_markdown_valid() {
        let input = vec![
            GCString::new("@title: Something"),
            GCString::new("@tags: tag1, tag2, tag3"),
            GCString::new("# Foobar"),
            GCString::new(""),
            GCString::new("Foobar is a Python library for dealing with word pluralization."),
            GCString::new(""),
            GCString::new("```bash"),
            GCString::new("pip install foobar"),
            GCString::new("```"),
            GCString::new("```fish"),
            GCString::new("```"),
            GCString::new("```python"),
            GCString::new(""),
            GCString::new("```"),
            GCString::new("## Installation"),
            GCString::new(""),
            GCString::new("Use the package manager [pip](https://pip.pypa.io/en/stable/) to install foobar."),
            GCString::new("```python"),
            GCString::new("import foobar"),
            GCString::new(""),
            GCString::new("foobar.pluralize('word') # returns 'words'"),
            GCString::new("foobar.pluralize('goose') # returns 'geese'"),
            GCString::new("foobar.singularize('phenomena') # returns 'phenomenon'"),
            GCString::new("```"),
            GCString::new("- ul1"),
            GCString::new("- ul2"),
            GCString::new("1. ol1"),
            GCString::new("2. ol2"),
            GCString::new("- [ ] todo"),
            GCString::new("- [x] done"),
            GCString::new("end"),
            GCString::new(""),
        ];

        let (remainder, list_block) = parse_markdown(&input).unwrap();

        let vec_block = &[
            MdBlock::Title("Something"),
            MdBlock::Tags(list!["tag1", "tag2", "tag3"]),
            MdBlock::Heading(HeadingData {
                heading_level: HeadingLevel { level: 1 },
                text: "Foobar",
            }),
            MdBlock::Text(list![]), /* Empty line */
            MdBlock::Text(list![MdLineFragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            MdBlock::Text(list![]), /* Empty line */
            MdBlock::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            MdBlock::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            MdBlock::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            MdBlock::Heading(HeadingData {
                heading_level: HeadingLevel { level: 2 },
                text: "Installation",
            }),
            MdBlock::Text(list![]), /* Empty line */
            MdBlock::Text(list![
                MdLineFragment::Plain("Use the package manager "),
                MdLineFragment::Link(HyperlinkData::from((
                    "pip",
                    "https://pip.pypa.io/en/stable/",
                ))),
                MdLineFragment::Plain(" to install foobar."),
            ]),
            MdBlock::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
            MdBlock::SmartList((
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
            MdBlock::SmartList((
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
            MdBlock::SmartList((
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
            MdBlock::SmartList((
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
            MdBlock::SmartList((
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
            MdBlock::SmartList((
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
            MdBlock::Text(list![MdLineFragment::Plain("end")]),
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
            GCString::new("@tags: [foo, bar"),
            GCString::new(""),
            GCString::new("```rs"),
            GCString::new("let a=1;"),
            GCString::new("```"),
            GCString::new(""),
            GCString::new("*italic* **bold** [link](https://example.com)"),
            GCString::new(""),
            GCString::new("`inline code`"),
        ];

        let (remainder, blocks) = parse_markdown(&input).unwrap();

        // println!("🍎input: '{}'", input);
        // println!("🍎remainder: {:?}", remainder);
        // println!("🍎blocks: {:#?}", blocks);

        assert_eq2!(remainder, "");
        assert_eq2!(blocks.len(), 7);
    }
}
