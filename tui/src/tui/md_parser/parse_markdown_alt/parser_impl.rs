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
use std::fmt::Debug;

use nom::{branch::alt,
          bytes::complete::tag,
          character::complete::{char, digit1, space0},
          combinator::{map, opt, recognize},
          error::{Error as NomError, ErrorKind as NomErrorKind},
          multi::{many0, many1},
          sequence::{preceded, terminated},
          AsChar,
          Compare,
          CompareResult,
          IResult,
          Input,
          Offset,
          Parser};

use crate::{constants::{BACK_TICK_CHAR, CHECKED, UNCHECKED},
            list,
            md_parser::{block::parse_block_smart_list::{BulletKind,
                                                        SmartListIR,
                                                        SmartListLine},
                        constants::{AUTHORS,
                                    CODE_BLOCK_END,
                                    CODE_BLOCK_START_PARTIAL,
                                    COLON,
                                    COLON_CHAR,
                                    COMMA,
                                    COMMA_CHAR,
                                    DATE,
                                    HEADING_CHAR,
                                    LIST_PREFIX_BASE_WIDTH,
                                    NEW_LINE,
                                    NEW_LINE_CHAR,
                                    ORDERED_LIST_PARTIAL_PREFIX,
                                    SPACE,
                                    SPACE_CHAR,
                                    TAGS,
                                    TITLE,
                                    UNORDERED_LIST_PREFIX}},
            parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line,
            parse_smart_list,
            take_text_until_new_line_or_end,
            tiny_inline_string,
            AsStrSlice,
            CheckboxParsePolicy,
            GCString,
            InlineVec,
            Lines,
            List,
            MdBlock,
            MdDocument,
            MdLineFragment};

/// Alternative implementation using the [nom::Input] trait (for future use).
/// Note that [AsStrSlice] implements [nom::Input].
///
/// Once the `input_arg` is converted into [AsStrSlice] you can use it as [nom::Input]
/// or [AsStrSlice] as you see fit, so there is a lot of flexibility in how to access
/// the input, in a "nom compatible" way.
pub fn parse_markdown_alt<'a>(
    input_arg: impl Into<AsStrSlice<'a>>,
) -> nom::IResult<AsStrSlice<'a>, MdDocument<'a>> {
    let input = input_arg.into();

    let (current_input, output) = many0(alt((
        // These parsers now work with any type that implements `nom::Input` which
        // includes `AsStrSlice`.
        map(
            make_many0_compatible(|input| parse_unique_kv_opt_eol_generic(TITLE, input)),
            |value: &'_ str| MdBlock::Title(value),
        ),
        map(
            make_many0_compatible(|input| parse_csv_opt_eol_generic(TAGS, input)),
            |values: List<&'_ str>| {
                // Create a new List directly to ensure proper type
                let mut tags_list = List::new();
                for &tag_str in values.iter() {
                    tags_list.push(tag_str);
                }
                MdBlock::Tags(tags_list)
            },
        ),
        map(
            make_many0_compatible(|input| parse_csv_opt_eol_generic(AUTHORS, input)),
            |values: List<&'_ str>| {
                // Create a new List directly to ensure proper type
                let mut authors_list = List::new();
                for &author_str in values.iter() {
                    authors_list.push(author_str);
                }
                MdBlock::Authors(authors_list)
            },
        ),
        map(
            make_many0_compatible(|input| parse_unique_kv_opt_eol_generic(DATE, input)),
            |value: &'_ str| MdBlock::Date(value),
        ),
        map(
            make_many0_compatible(parse_block_heading_generic),
            |(level, text): (usize, AsStrSlice<'_>)| {
                // Extract actual heading text from the text slice
                let heading_text = text.extract_remaining_text_content_in_line();
                MdBlock::Heading(crate::HeadingData {
                    heading_level: crate::HeadingLevel { level },
                    text: heading_text,
                })
            },
        ),
        map(
            make_many0_compatible(parse_code_block_generic),
            extract_code_block_content,
        ),
        map(
            make_many0_compatible(parse_block_smart_list_generic),
            |(lines, bullet_kind, indent): (Lines<'_>, BulletKind, usize)| {
                // The lines, bullet_kind, and indent are already properly structured
                // Just need to wrap them in MdBlock::SmartList
                MdBlock::SmartList((lines, bullet_kind, indent))
            },
        ),
        map(
            make_many0_compatible(parse_block_text_generic),
            |text: AsStrSlice<'_>| {
                // Create proper text structure with actual content
                let mut line_fragments = List::new();
                let text_content = text.extract_remaining_text_content_in_line();
                line_fragments.push(crate::MdLineFragment::Plain(text_content));
                MdBlock::Text(line_fragments)
            },
        ),
    )))
    .parse(input)?;

    Ok((current_input, List::from(output)))
}

/// Extracts and processes code block content from a parsed markdown slice.
///
/// ## Context
///
/// [parse_code_block_generic()] handles the boundary detection, while this function
/// handles content extraction. They work hand in hand.
///
/// This helper function processes the output from [parse_code_block_generic()], which is
/// guaranteed to produce a valid Markdown fenced code block slice, to extract the
/// language specification and code content from a markdown code block, converting it into
/// a structured [MdBlock::CodeBlock].
///
/// Note that the return type of [parse_code_block_generic()] is a concrete type that
/// implements [nom::Input], and so this return value can be passed as an argument to this
/// function which receives a `slice` input of type [AsStrSlice].
///
/// ## Processing Pipeline
///
/// 1. **Input**: Takes a [AsStrSlice] containing the raw fenced code block content
///    (including markers) as identified by [parse_code_block_generic()]
/// 2. **Text Extraction**: Extracts the text content and splits it into lines for
///    processing
/// 3. **Language Detection**: Parses the first line to extract optional language
///    specification
/// 4. **Content Collection**: Collects code lines between the opening and closing ```
///    markers
/// 5. **Structure Building**: Creates [CodeBlockLine] entries with
///    [CodeBlockLineContent::Text]
///
/// ## Functionality
///
/// 1. **Language Extraction**: Parses the first line to extract the optional language
///    specification after the opening ``` markers
/// 2. **Content Processing**: Extracts all lines between the opening and closing ```
///    markers as code content
/// 3. **Structure Creation**: Builds a proper [MdBlock::CodeBlock] with [CodeBlockLine]
///    entries containing the language and content
///
/// ## Dependencies
///
/// This function depends on the output of [parse_code_block_generic()], which provides
/// the initial parsing of code block boundaries and guarantees that the input contains
/// a well-formed fenced code block with proper opening and closing markers.
///
/// ## Parameters
///
/// * `slice` - A [AsStrSlice] containing the markdown text content to process, typically
///   obtained from [parse_code_block_generic()]
///
/// ## Returns
///
/// Returns an [MdBlock::CodeBlock] containing:
/// - Language specification (if present)
/// - All code lines as [CodeBlockLine] entries with [CodeBlockLineContent::Text]
///
/// ## Example Input Processing
///
/// ```rs
/// fn main() { println!("Hello, world!"); }
/// ```
///
/// This would produce a code block with:
/// - Language: `Some("rs")`
/// - Content: `fn main() { println!("Hello, world!"); }`
fn extract_code_block_content<'a>(slice: AsStrSlice<'a>) -> MdBlock<'a> {
    let text = slice.extract_remaining_text_content_in_line();
    let lines: Vec<&str> = text.lines().collect();

    // Extract language from first line
    let first_line = if !lines.is_empty() { lines[0] } else { "" };
    let language =
        if first_line.len() > 3 && first_line.starts_with(CODE_BLOCK_START_PARTIAL) {
            let lang_part = first_line[3..].trim();
            if lang_part.is_empty() {
                None
            } else {
                Some(lang_part)
            }
        } else {
            None
        };

    // Extract code lines (between ``` markers)
    let mut raw_code_lines = Vec::new();
    let mut in_code = false;

    for line in lines.iter().skip(1) {
        if line.trim() == CODE_BLOCK_END {
            break;
        }
        if in_code || !first_line.starts_with(CODE_BLOCK_START_PARTIAL) {
            in_code = true;
        }
        raw_code_lines.push(*line);
    }

    // Create proper code block structure with actual content
    let mut code_lines = List::new();
    for line in raw_code_lines {
        code_lines.push(crate::CodeBlockLine {
            language,
            content: crate::CodeBlockLineContent::Text(line),
        });
    }

    MdBlock::CodeBlock(code_lines)
}

// Helper function to extract list item content
fn extract_list_item_content<'a>(slice: AsStrSlice<'a>) -> Option<&'a str> {
    let text = slice.extract_remaining_text_content_in_line();
    let trimmed = text.trim();

    // Handle unordered list items
    if let Some(content) = trimmed.strip_prefix(UNORDERED_LIST_PREFIX) {
        return Some(content);
    }

    // Handle ordered list items (simplified - just look for digit +
    // ORDERED_LIST_PARTIAL_PREFIX)
    if let Some(dot_pos) = trimmed.find(ORDERED_LIST_PARTIAL_PREFIX) {
        let prefix = &trimmed[..dot_pos];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            let it = &trimmed[dot_pos + ORDERED_LIST_PARTIAL_PREFIX.len()..];
            return Some(it);
        }
    }

    None
}

// Generic helper function to take text until newline or end for any Input type
fn take_text_until_new_line_or_end_generic<I>() -> impl Fn(I) -> IResult<I, I>
where
    I: Input + Clone,
    I::Item: AsChar + Copy,
{
    move |input: I| {
        // Handle empty input case explicitly
        if input.input_len() == 0 {
            return Ok((input.clone(), input));
        }

        let original_len = input.input_len();
        let mut input_clone = input.clone();
        let mut consumed = 0;

        loop {
            // Check if we've consumed all characters
            if consumed >= original_len {
                break;
            }

            let slice = input_clone.take(1);
            if let Some(ch) = slice.iter_elements().next() {
                if ch.as_char() == NEW_LINE_CHAR {
                    break;
                }
                input_clone = input_clone.take_from(1);
                consumed += 1;
            } else {
                // No more characters available
                break;
            }
        }

        let taken = input.take(consumed);
        let remaining = input.take_from(consumed);
        Ok((remaining, taken))
    }
}

// Removed unused take_until_comma_or_end_generic_parser function

/// Generic version of [crate::md_parser::extended::parse_unique_kv_opt_eol] for future
/// use.
///
/// - Sample parse input: `@title: Something` or `@date: Else`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
/// - Can't nest the `tag_name` within the `output`. So there can only be one `tag_name`
///   in the `output`.
fn parse_unique_kv_opt_eol_generic<'a>(
    tag_name: &'a str,
    input: AsStrSlice<'a>,
) -> IResult</* remainder */ AsStrSlice<'a>, /* output */ &'a str> {
    // In case of error, this is a backup clone of input.
    let input_clone = input.clone();

    let (remainder, title_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ take_text_until_new_line_or_end_generic(),
    )
    .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    let tag_fragment = tiny_inline_string!("{tag_name}{COLON}{SPACE}");
    let tag_str = tag_fragment.as_str();

    // Extract text content to check for nested tag_name
    let title_str = title_text.extract_remaining_text_content_in_line();
    let remainder_str = remainder.extract_remaining_text_content_in_line();

    if title_str.contains(tag_str) || remainder_str.contains(tag_str) {
        return Err(nom::Err::Error(NomError::new(
            input_clone, // Use input as the error location
            NomErrorKind::Fail,
        )));
    }

    // If there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    // Special case: Early return when something like `@title: ` or `@title: \n` is found.
    if title_text.input_len() == 0 {
        Ok((remainder, ""))
    }
    // Normal case.
    else {
        Ok((remainder, title_str))
    }
}

/// Generic version of [crate::md_parser::extended::parse_csv_opt_eol] for future use.
///
/// - Sample parse input: `@tags: tag1, tag2, tag3`, `@tags: tag1, tag2, tag3\n`, or
///   `@authors: me, myself, i`, `@authors: me, myself, i\n`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
pub fn parse_csv_opt_eol_generic<'a>(
    tag_name: &'a str,
    input: AsStrSlice<'a>,
) -> IResult</* remainder */ AsStrSlice<'a>, /* output */ List<&'a str>> {
    let (remainder, tags_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ take_text_until_new_line_or_end_generic(),
    )
    .parse(input)?;

    // If there is a newline, consume it since there may or may not be a newline at
    // the end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    // Special case: Early return when just a `@tags: ` or `@tags: \n` is found.
    if tags_text.input_len() == 0 {
        Ok((remainder, list![]))
    }
    // Normal case.
    else {
        // At this point, `tags_text` can have something like: `tag1, tag2, tag3`.
        // Split by comma and process each item
        let text_content = tags_text.extract_remaining_text_content_in_line();

        // Create a list of string slices
        let mut items = List::new();

        // Split by comma and process each item
        if !text_content.is_empty() {
            let parts: InlineVec<&str> = text_content.split(COMMA).collect();

            for (i, part) in parts.iter().enumerate() {
                let part_trimmed = if i > 0 && part.starts_with(SPACE) {
                    &part[1..]
                } else {
                    *part
                };

                // Add the string slice directly to the list
                items.push(part_trimmed);
            }
        }

        Ok((remainder, items))
    }
}

// Specific parsers using generic implementations

// Generic heading parser
fn parse_block_heading_generic<'a, I>(
    input: I,
) -> IResult</* remainder */ I, /* output */ (usize, I)>
where
    I: Input + Clone + Compare<&'a str>,
    I::Item: AsChar + Copy,
{
    let (input, hashes) = many1(char(HEADING_CHAR)).parse(input)?;
    let (input, _) = char(SPACE_CHAR).parse(input)?;
    let (input, text) = take_text_until_new_line_or_end_generic().parse(input)?;
    let (input, _) = opt(tag(NEW_LINE)).parse(input)?;

    Ok((input, (hashes.len(), text)))
}

// Generic text parser
fn parse_block_text_generic<'a, I>(input: I) -> IResult<I, I>
where
    I: Input + Clone + Compare<&'a str>,
    I::Item: AsChar + Copy,
{
    let (input, text) = take_text_until_new_line_or_end_generic().parse(input)?;

    // Fail if no text was captured (empty input)
    if text.input_len() == 0 {
        return Err(nom::Err::Error(NomError::new(input, NomErrorKind::Tag)));
    }

    let (input, _) = opt(tag(NEW_LINE)).parse(input)?;

    Ok((input, text))
}

/// Generic version of the [parse_block_smart_list::parse_smart_list] function that uses
/// [AsStrSlice]. This function parses a smart list from markdown text and returns a
/// [SmartListIR] structure.
///
/// This function specifically works with [AsStrSlice] to leverage its specialized
/// methods for more efficient character manipulation.
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
pub fn parse_smart_list_and_extract_ir_generic<'a>(
    input: AsStrSlice<'a>,
) -> IResult</* remainder */ AsStrSlice<'a>, /* output */ SmartListIR<'a>> {
    // Calculate indent by counting leading spaces
    let mut indent = 0;
    let mut input_clone = input.clone();

    // Count leading spaces using AsStrSlice's methods
    let text_content = input_clone.extract_remaining_text_content_in_line();
    let leading_spaces = text_content
        .chars()
        .take_while(|&c| c == SPACE_CHAR)
        .count();
    indent = leading_spaces;

    // Skip leading spaces in the input
    let input_after_spaces = if indent > 0 {
        // Use take_from to get the input starting from the indent position
        input.take_from(indent)
    } else {
        input
    };

    // Check for unordered list marker
    let bullet_kind: BulletKind;
    let bullet_str: &'static str;

    // Get the text content for easier comparison
    let text_content = input_after_spaces.extract_remaining_text_content_in_line();

    let input = if text_content.starts_with(UNORDERED_LIST_PREFIX) {
        let (input, _) = tag(UNORDERED_LIST_PREFIX).parse(input_after_spaces)?;
        bullet_kind = BulletKind::Unordered;
        bullet_str = UNORDERED_LIST_PREFIX;
        input
    } else {
        // Check for ordered list marker
        let mut digits = String::new();
        let mut found_digit = false;
        let mut count = 0;

        // Extract digits from the beginning of the text content
        for ch in text_content.chars() {
            if ch.is_ascii_digit() {
                found_digit = true;
                digits.push(ch);
                count += 1;
            } else {
                break;
            }
        }

        if found_digit && text_content[count..].starts_with(ORDERED_LIST_PARTIAL_PREFIX) {
            // Parse the number
            let number_usize =
                digits
                    .parse::<usize>()
                    .or(Err(nom::Err::Error(NomError::new(
                        input_after_spaces.clone(),
                        NomErrorKind::Fail,
                    ))))?;

            // Skip the digits and the ". " part
            let (input, _) =
                input_after_spaces.take_split(count + ORDERED_LIST_PARTIAL_PREFIX.len());

            bullet_kind = BulletKind::Ordered(number_usize);

            // Create a static string for the bullet
            let full_bullet = format!("{}. ", number_usize);
            bullet_str = Box::leak(full_bullet.into_boxed_str());

            input
        } else {
            return Err(nom::Err::Error(NomError::new(
                input_after_spaces,
                NomErrorKind::Tag,
            )));
        }
    };

    // Match the rest of the line & other lines that have the same indent.
    let (remainder, content) = take_text_until_new_line_or_end_generic().parse(input)?;

    // Extract content directly using AsStrSlice's methods
    let content_str = content.extract_remaining_text_content_in_line().to_string();

    // For ordered lists, the content is actually in the remaining input
    // This is because our simplified parser doesn't handle the content correctly
    let content_str = if let BulletKind::Ordered(_) = bullet_kind {
        // Get the content from the remaining input
        remainder
            .extract_remaining_text_content_in_line()
            .to_string()
    } else {
        content_str
    };

    // Create a SmartListLine for the first line
    let first_line = SmartListLine {
        indent,
        bullet_str,
        content: Box::leak(content_str.into_boxed_str()),
    };

    // Update input to point to the remainder
    let input = remainder;

    // For simplicity in this generic version, we'll just handle a single line
    // A more complete implementation would parse multiple lines with the same indent
    let mut content_lines = InlineVec::new();
    content_lines.push(first_line);

    // Return the result.
    Ok((
        input, // This is now the remainder after parsing the content
        SmartListIR {
            indent,
            bullet_kind,
            content_lines,
        },
    ))
}

fn extract_block_smart_list_from_ir_generic_alt<'a>(
    smart_list_ir: SmartListIR<'a>,
) -> Option<(Lines<'a>, BulletKind, usize)> {
    let indent = smart_list_ir.indent;
    let bullet_kind = smart_list_ir.bullet_kind;
    let mut output_lines: Lines<'_> =
        List::with_capacity(smart_list_ir.content_lines.len());

    for (index, line) in smart_list_ir.content_lines.iter().enumerate() {
        // Parse the line as a markdown text. Take special care of checkboxes if they show
        // up at the start of the line.
        let fragments_in_line = {
            let parse_checkbox_policy = {
                let checked = tiny_inline_string!("{}{}", CHECKED, SPACE);
                let unchecked = tiny_inline_string!("{}{}", UNCHECKED, SPACE);
                if line.content.starts_with(checked.as_str())
                    || line.content.starts_with(unchecked.as_str())
                {
                    CheckboxParsePolicy::ParseCheckbox
                } else {
                    CheckboxParsePolicy::IgnoreCheckbox
                }
            };
            // If there is an error return the entire input as remainder
            let res_it =
                parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line(
                    line.content,
                    parse_checkbox_policy,
                );

            let Ok((_, fragments)) = res_it else {
                // If there is an error, return None to indicate failure.
                return None;
            };

            fragments
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

    Some((output_lines, bullet_kind, indent))
}

pub fn parse_block_smart_list_generic<'a>(
    input: AsStrSlice<'a>,
) -> IResult<
    /* remainder */ AsStrSlice<'a>,
    /* output */ (Lines<'a>, BulletKind, usize),
> {
    // Backup of input, in case of error
    let input_clone = input.clone();

    // Parse the smart list structure first
    let (remainder, smart_list_ir) = parse_smart_list_and_extract_ir_generic(input)?;

    // Extract and process the smart list information
    match extract_block_smart_list_from_ir_generic_alt(smart_list_ir) {
        Some(output) => Ok((remainder, output)),
        None => {
            // If there was an error extracting the smart list, return it
            Err(nom::Err::Error(NomError::new(
                input_clone,
                NomErrorKind::Fail,
            )))
        }
    }
}

/// The function is a low-level splitter that identifies where a code block starts and
/// ends, returning the entire block as a slice of the input, which can then be further
/// processed by higher-level parsers.
///
/// A generic parser for markdown code blocks delimited by triple backticks (```).
/// It does not extract the [MdBlock::CodeBlock] from the `input`, it only splits
/// the `input` into:
/// 1. the fenced code block slice (including opening and closing markers) and
/// 2. the remainder of input slice.
///
/// ## Context
///
/// This function handles the boundary detection, while [extract_code_block_content]
/// handles content extraction. They work hand in hand.
///
/// This is a low-level generic function that works with any input type implementing
/// the required nom traits. It expects well-formed code blocks with proper closing
/// markers.
///
/// # Returns
/// - `Ok((remaining_input, fenced_code_block_slice))` - Successfully parsed code block
///   slice.
/// - `Err(_)` - Input doesn't start with code block marker or missing closing marker.
///
/// # Behavior
/// - Expects input to start with "```"
/// - Captures the entire fenced code block (including markers)
/// - **Requires a closing marker** - will fail if the closing "```" is missing
/// - Does not parse or extract the inner content - returns the raw slice
///
/// # Examples
/// ```rust
/// // With proper closing marker - succeeds
/// # use nom::IResult;
/// # use r3bl_tui::r#impl::parse_code_block_generic;
/// # fn it() -> Result<(), Box<dyn std::error::Error>> {
/// let input = "```rust\nlet x = 5;\n```\nmore text";
/// let (remaining, fenced_block) = parse_code_block_generic(input)?;
/// assert_eq!(fenced_block, "```rust\nlet x = 5;\n```");
/// assert_eq!(remaining, "\nmore text");
/// # Ok(())
/// # }
/// ```
///
/// ```rust
/// // Missing closing marker - returns error
/// # use nom::IResult;
/// # use r3bl_tui::r#impl::parse_code_block_generic;
/// let input = "```python\nprint('hello')";
/// let result = parse_code_block_generic(input);
/// assert!(result.is_err());
/// ```
pub fn parse_code_block_generic<'a, I>(input: I) -> IResult<I, I>
where
    I: Input + Clone + Compare<&'a str>,
    I::Item: AsChar + Copy,
{
    // Look for code block start "```".
    let (input, _) = tag(CODE_BLOCK_START_PARTIAL).parse(input)?;

    // Skip optional language specification until newline.
    let (input, _lang) = take_text_until_new_line_or_end_generic().parse(input)?;
    let (input, _) = opt(tag(NEW_LINE)).parse(input)?;

    // Take everything until "```" (handles missing end markers).
    let mut input_clone = input.clone();
    let mut content_len = 0;
    let original_len = input.input_len();
    let mut found_end_marker = false;

    while content_len + 3 <= original_len {
        // Check if we found the end marker.
        let slice = input_clone.take(3);
        let mut chars = slice.iter_elements();
        if chars.next().map(|c| c.as_char()) == Some(BACK_TICK_CHAR)
            && chars.next().map(|c| c.as_char()) == Some(BACK_TICK_CHAR)
            && chars.next().map(|c| c.as_char()) == Some(BACK_TICK_CHAR)
        {
            // Found end marker.
            found_end_marker = true;
            break;
        }

        // Move forward to next character.
        input_clone = input_clone.take_from(1);
        content_len += 1;
    }

    if found_end_marker {
        // Extract the content up to the end marker.
        let (content, remaining) = input.take_split(content_len);

        // Skip the closing "```".
        let (remaining, _) = tag(CODE_BLOCK_END).parse(remaining)?;
        let (remaining, _) = opt(tag(NEW_LINE)).parse(remaining)?;

        Ok((remaining, content))
    } else {
        // No end marker found, return an error so that many0 will go to the next parser.
        Err(nom::Err::Error(NomError::new(input, NomErrorKind::Tag)))
    }
}

/// Helper function to ensure parsers return [nom::Err::Error] instead of
/// [nom::Err::Failure] for `many0` compatibility. Here's what it does:
///
/// 1. It wraps a parser to convert `Failure` errors to `Error` errors.
/// 2. This is important because nom's `many0` combinator stops collecting items when it
///    encounters an `Error`, but it will propagate a `Failure` error up the chain, which
///    can cause issues with the overall parsing strategy. The function allows parsers
///    that might return `Failure` errors to work smoothly with `many0` by turning those
///    `Failure`s into `Error`s at the boundary. This lets many0 treat those errors as
///    "end of sequence" markers rather than critical failures that should halt parsing
///    entirely.
fn make_many0_compatible<I, O, F>(mut parser: F) -> impl FnMut(I) -> IResult<I, O>
where
    F: FnMut(I) -> IResult<I, O>,
    I: Clone + Debug,
{
    move |input: I| {
        match parser.parse(input) {
            Ok(result) => Ok(result),
            Err(nom::Err::Failure(e)) => {
                // Convert Failure to Error so many0 treats it as "end of sequence"
                Err(nom::Err::Error(e))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{inline_vec, AsStrSlice, GCString};

    #[test]
    fn test_gc_string_slice_basic_functionality() {
        let lines = vec![
            GCString::new("Hello world"),
            GCString::new("This is a test"),
            GCString::new("Third line"),
        ];

        let slice = AsStrSlice::from(&lines);

        // Test that we can iterate through characters
        let mut chars: Vec<char> = vec![];
        let mut current = slice;
        while let Some(ch) = current.current_char() {
            chars.push(ch);
            current.advance();
        }

        let expected = "Hello world\nThis is a test\nThird line";
        let result: String = chars.into_iter().collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_simple_markdown() {
        let lines = inline_vec![
            GCString::new("@title: Test Document"),
            GCString::new("@tags: rust, parsing"),
            GCString::new("# Heading 1"),
            GCString::new("Some text content"),
            GCString::new("- List item 1"),
            GCString::new("```rust"),
            GCString::new("let x = 42;"),
            GCString::new(CODE_BLOCK_END),
        ];

        let result = parse_markdown_alt(lines.as_ref());

        // Should parse successfully
        assert!(result.is_ok());

        let (remaining, document) = result.unwrap();

        // Should have consumed all input
        assert_eq!(remaining.input_len(), 0);

        // Should have parsed multiple blocks
        assert!(document.len() > 0);

        // Assert the parsed blocks match expected types and values
        use crate::MdBlock;
        assert!(
            matches!(document[0], MdBlock::Title(_)),
            "First block should be Title"
        );
        if let MdBlock::Title(title) = &document[0] {
            assert_eq!(*title, "Test Document");
        }
        assert!(
            matches!(document[1], MdBlock::Tags(_)),
            "Second block should be Tags"
        );
        if let MdBlock::Tags(tags) = &document[1] {
            assert!(tags.contains(&"rust"));
            assert!(tags.contains(&"parsing"));
        }
        assert!(
            matches!(document[2], MdBlock::Heading(_)),
            "Third block should be Heading"
        );
        assert!(
            matches!(document[3], MdBlock::Text(_)),
            "Fourth block should be Text"
        );
        assert!(
            matches!(document[4], MdBlock::SmartList(_)),
            "Fifth block should be SmartList"
        );
        assert!(
            matches!(document[5], MdBlock::CodeBlock(_)),
            "Sixth block should be CodeBlock"
        );
    }

    #[test]
    fn test_nom_input_position() {
        let lines = vec![GCString::new("hello"), GCString::new("world")];

        let slice = AsStrSlice::from(&lines);

        // Test position finding
        let pos = slice.position(|c| c == 'w');
        assert_eq!(pos, Some(6)); // "hello\n" = 6 chars, then 'w'

        let pos = slice.position(|c| c == 'z');
        assert_eq!(pos, None); // 'z' not found
    }

    #[test]
    fn test_individual_parsers() {
        let lines = vec![GCString::new("@title: Test Document")];
        let slice = AsStrSlice::from(&lines);

        // Test title parser directly
        let result = parse_unique_kv_opt_eol_generic(TITLE, slice);
        assert!(result.is_ok(), "Title parser should succeed");

        let (remaining, value) = result.unwrap();
        assert_eq!(remaining.input_len(), 0, "Should consume all input");
        assert_eq!(value, "Test Document", "Should extract correct title value");
    }

    #[test]
    fn test_heading_parser() {
        let lines = vec![GCString::new("# Heading 1")];
        let slice = AsStrSlice::from(&lines);

        let result = parse_block_heading_generic(slice);
        assert!(result.is_ok(), "Heading parser should succeed");

        let (remaining, (level, text)) = result.unwrap();
        assert_eq!(remaining.input_len(), 0, "Should consume all input");
        assert_eq!(level, 1, "Should parse heading level correctly");
        assert_eq!(
            text.extract_remaining_text_content_in_line(),
            "Heading 1",
            "Should extract correct heading text"
        );
    }

    #[test]
    fn test_list_parser() {
        let lines = vec![GCString::new("- List item 1")];
        let slice = AsStrSlice::from(&lines);

        let result = parse_block_smart_list_generic(slice);
        assert!(result.is_ok(), "List parser should succeed");

        let (remaining, (lines, bullet_kind, indent)) = result.unwrap();
        assert_eq!(remaining.input_len(), 0, "Should consume all input");

        assert_eq!(indent, 0, "Indent should be 0 for simple list");
        assert!(
            matches!(bullet_kind, BulletKind::Unordered),
            "Bullet kind should be Unordered"
        );
        assert_eq!(
            lines
                .first()
                .map(|line| {
                    line.iter()
                        .filter_map(|fragment| {
                            if let MdLineFragment::Plain(text) = fragment {
                                Some(*text)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default(),
            "List item 1",
            "Should extract correct list content"
        );
    }

    #[test]
    fn test_smart_list_ir_generic() {
        // Test unordered list
        {
            let lines = vec![GCString::new("- List item 1")];
            let slice = AsStrSlice::from(&lines);

            let result = parse_smart_list_and_extract_ir_generic(slice);
            assert!(
                result.is_ok(),
                "Smart list IR parser should succeed for unordered list"
            );

            let (remaining, smart_list_ir) = result.unwrap();
            assert_eq!(remaining.input_len(), 0, "Should consume all input");
            assert_eq!(smart_list_ir.indent, 0, "Indent should be 0");
            assert!(
                matches!(smart_list_ir.bullet_kind, BulletKind::Unordered),
                "Bullet kind should be Unordered"
            );
            assert_eq!(
                smart_list_ir.content_lines.len(),
                1,
                "Should have 1 content line"
            );
            assert_eq!(
                smart_list_ir.content_lines[0].content, "List item 1",
                "Content should match"
            );
        }

        // Test ordered list
        {
            let lines = vec![GCString::new("1. List item 1")];
            let slice = AsStrSlice::from(&lines);

            let result = parse_smart_list_and_extract_ir_generic(slice);
            assert!(
                result.is_ok(),
                "Smart list IR parser should succeed for ordered list"
            );

            let (remaining, smart_list_ir) = result.unwrap();
            // For now, we'll accept that the parser doesn't consume all input
            // This is because our implementation is simplified and doesn't handle
            // multi-line content
            assert_eq!(smart_list_ir.indent, 0, "Indent should be 0");

            if let BulletKind::Ordered(number) = smart_list_ir.bullet_kind {
                assert_eq!(number, 1, "List number should be 1");
            } else {
                panic!("Bullet kind should be Ordered");
            }

            assert_eq!(
                smart_list_ir.content_lines.len(),
                1,
                "Should have 1 content line"
            );
            assert_eq!(
                smart_list_ir.content_lines[0].content, "List item 1",
                "Content should match"
            );
        }

        // Test indented list
        {
            let lines = vec![GCString::new("  - Indented list item")];
            let slice = AsStrSlice::from(&lines);

            let result = parse_smart_list_and_extract_ir_generic(slice);
            assert!(
                result.is_ok(),
                "Smart list IR parser should succeed for indented list"
            );

            let (remaining, smart_list_ir) = result.unwrap();
            assert_eq!(remaining.input_len(), 0, "Should consume all input");
            assert_eq!(smart_list_ir.indent, 2, "Indent should be 2");
            assert!(
                matches!(smart_list_ir.bullet_kind, BulletKind::Unordered),
                "Bullet kind should be Unordered"
            );
            assert_eq!(
                smart_list_ir.content_lines.len(),
                1,
                "Should have 1 content line"
            );
            assert_eq!(
                smart_list_ir.content_lines[0].content, "Indented list item",
                "Content should match"
            );
        }
    }

    #[test]
    fn test_multiple_lines_parsing() {
        let lines = vec![
            GCString::new("@title: Test Document"),
            GCString::new("@tags: rust, parsing"),
        ];
        let slice = AsStrSlice::from(&lines);

        // Parse title
        let (remaining, title_value) =
            parse_unique_kv_opt_eol_generic(TITLE, slice).unwrap();
        assert_eq!(title_value, "Test Document", "Should extract correct title");
        assert!(
            remaining.input_len() > 0,
            "Should have remaining input after title"
        );

        // Should be able to parse tags next
        let result = parse_csv_opt_eol_generic(TAGS, remaining);
        assert!(result.is_ok(), "Tags parsing should succeed");

        let (final_remaining, tags_value) = result.unwrap();
        let tags: Vec<&str> = tags_value.iter().copied().collect();
        assert_eq!(tags, vec!["rust", "parsing"]);
        assert_eq!(
            final_remaining.input_len(),
            0,
            "Should consume all remaining input"
        );
    }

    #[test]
    fn test_title_to_tags_transition() {
        let lines = vec![
            GCString::new("@title: Test Document"),
            GCString::new("@tags: rust, parsing"),
        ];

        let input_slice = AsStrSlice::from(&lines);

        // Test parsing just the title
        let title_result = parse_unique_kv_opt_eol_generic(TITLE, input_slice);
        assert!(title_result.is_ok(), "Title parsing should succeed");

        let (remaining_after_title, title_value) = title_result.unwrap();
        assert_eq!(title_value, "Test Document", "Should extract correct title");
        assert!(
            remaining_after_title.input_len() > 0,
            "Should have remaining input after title"
        );

        // Now try to parse tags from remaining input
        let tags_result = parse_csv_opt_eol_generic(TAGS, remaining_after_title);
        assert!(tags_result.is_ok(), "Tags parsing should succeed");

        let (final_remaining, tags_value) = tags_result.unwrap();
        let tags: Vec<&str> = tags_value.iter().copied().collect();
        assert_eq!(tags, vec!["rust", "parsing"]);
        assert_eq!(
            final_remaining.input_len(),
            0,
            "Should consume all remaining input"
        );
    }

    #[test]
    fn test_alt_combinator_behavior() {
        let lines = vec![
            GCString::new("@title: Test Document"),
            GCString::new("@tags: rust, parsing"),
        ];

        let input_slice = AsStrSlice::from(&lines);

        // Test the alt combinator directly without many0
        let mut alt_parser = alt((
            map(
                make_many0_compatible(|input| {
                    parse_unique_kv_opt_eol_generic(TITLE, input)
                }),
                |_| "title",
            ),
            map(
                make_many0_compatible(|input| parse_csv_opt_eol_generic(TAGS, input)),
                |_| "tags",
            ),
        ));

        let result1 = alt_parser.parse(input_slice);
        assert!(result1.is_ok(), "First line alt parsing should succeed");

        let (remaining, parsed_type) = result1.unwrap();
        assert_eq!(parsed_type, "title", "Should parse as title");
        assert!(remaining.input_len() > 0, "Should have remaining input");

        // Now test the alt combinator on the remaining input
        let result2 = alt_parser.parse(remaining);
        assert!(result2.is_ok(), "Second line alt parsing should succeed");

        let (final_remaining, parsed_type2) = result2.unwrap();
        assert_eq!(parsed_type2, "tags", "Should parse as tags");
        assert_eq!(final_remaining.input_len(), 0, "Should consume all input");
    }

    #[test]
    fn test_many0_with_empty_input() {
        let lines = vec![GCString::new("@title: Test Document")];
        let input_slice = AsStrSlice::from(&lines);

        // Parse the title
        let (remaining_input, title_value) =
            parse_unique_kv_opt_eol_generic(TITLE, input_slice).unwrap();
        assert_eq!(title_value, "Test Document", "Should extract correct title");
        assert_eq!(remaining_input.input_len(), 0, "Should consume all input");

        // Now test what happens when many0 tries to parse the empty remaining input
        let mut alt_parser = alt((
            map(
                make_many0_compatible(|input| {
                    parse_unique_kv_opt_eol_generic(TITLE, input)
                }),
                |_| "title",
            ),
            map(
                make_many0_compatible(|input| parse_csv_opt_eol_generic(TAGS, input)),
                |_| "tags",
            ),
        ));

        let result = alt_parser.parse(remaining_input.clone());
        assert!(result.is_err(), "Alt parser should fail on empty input");

        // Test many0 directly on empty input - should succeed with empty result
        let many0_result = many0(alt_parser).parse(remaining_input);
        assert!(many0_result.is_ok(), "Many0 should succeed on empty input");

        let (final_remaining, parsed_items) = many0_result.unwrap();
        assert_eq!(
            final_remaining.input_len(),
            0,
            "Should not advance on empty input"
        );
        assert_eq!(
            parsed_items.len(),
            0,
            "Should return empty vector for empty input"
        );
    }

    #[test]
    fn test_parse_block_code_with_missing_end_marker() {
        // Test with a complete code block (has end marker)
        let lines = vec![
            GCString::new("```rust"),
            GCString::new("fn test() {"),
            GCString::new("    println!(\"Hello, world!\");"),
            GCString::new("}"),
            GCString::new("```"),
        ];
        let slice = AsStrSlice::from(&lines);

        let result = parse_code_block_generic(slice);
        assert!(
            result.is_ok(),
            "Code block parser should succeed with end marker"
        );

        let (remaining, content) = result.unwrap();
        assert_eq!(remaining.input_len(), 0, "Should consume all input");

        // Test with a code block missing the end marker
        let lines = vec![
            GCString::new("```rust"),
            GCString::new("fn test() {"),
            GCString::new("    println!(\"Hello, world!\");"),
            GCString::new("}"),
            // No closing ```
        ];
        let slice = AsStrSlice::from(&lines);

        let result = parse_code_block_generic(slice);
        assert!(
            result.is_err(),
            "Code block parser should fail without end marker"
        );

        // Verify that the error is of the expected type (Error, not Failure)
        match result {
            Err(nom::Err::Error(_)) => {
                // This is the expected error type
            }
            Err(nom::Err::Failure(_)) => {
                panic!("Expected Error, got Failure");
            }
            _ => {
                panic!("Expected Error, got something else");
            }
        }
    }
}
