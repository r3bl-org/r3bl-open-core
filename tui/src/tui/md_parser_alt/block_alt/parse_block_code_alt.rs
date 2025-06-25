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
          bytes::complete::{is_not, tag, take_until},
          combinator::{map, opt},
          sequence::{preceded, terminated},
          IResult,
          Input,
          Parser};

use crate::{constants::NEW_LINE_CHAR,
            md_parser::constants::{CODE_BLOCK_END, CODE_BLOCK_START_PARTIAL, NEW_LINE},
            AsStrSlice,
            CodeBlockLine,
            CodeBlockLineContent,
            InlineVec,
            List};

/// Parses a complete Markdown code block - a **multi-line block parser** with automatic advancement.
///
/// ## Purpose
/// This is a **block-level parser** that processes entire fenced code blocks (```...```)
/// and returns structured output ready for rendering. It handles various code block formats
/// including those with or without language specifications, content lines, and trailing newlines.
///
/// ## Parsing Strategy Overview
///
/// This function implements **block-level parsing** that accumulates multiple lines into
/// structured output. Unlike single-line parsers, this function:
///
/// 1. **Multi-line Accumulation**: Consumes entire code blocks spanning multiple lines
/// 2. **Block Boundary Detection**: Identifies start (```) and end (```) markers
/// 3. **Language Extraction**: Parses optional language specifiers (e.g., `bash`, `rust`)
/// 4. **Line-by-Line Processing**: Splits content into individual code lines
/// 5. **Structured Output**: Returns `CodeBlockLine` objects for rendering
///
/// ### Processing Flow
/// ```text
/// Input: "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n"
///        ↓
/// 1. Parse opening: "```rust" → language = Some("rust")
/// 2. Extract body: "fn main() {\n    println!(\"Hello\");\n}"
/// 3. Split by lines: ["fn main() {", "    println!(\"Hello\");", "}"]
/// 4. Build IR: [CodeBlockLine{...}, CodeBlockLine{...}, CodeBlockLine{...}]
/// 5. Return: (remainder, code_block_lines)
/// ```
///
/// ## Comparison with Single-Line Parsers
///
/// | Single-Line Parser | Block Parser (this function) |
/// |-------------------|------------------------------|
/// | Processes one line | Processes entire code blocks |
/// | Simple string output | Complex structured output |
/// | Fast, minimal state | Stateful multi-line accumulation |
/// | Used for inline elements | Used for block elements |
///
/// ## Performance: O(1) Line Processing Through Zero-Copy Design
///
/// While the overall parsing involves processing the code block content, the **line splitting
/// operation achieves O(1) efficiency per line** by leveraging `AsStrSlice`'s pre-computed
/// structure rather than materializing input strings:
///
/// **Avoided expensive operations:**
/// - ❌ Full input materialization via `memcpy()` - would require copying entire blocks
/// - ❌ Repeated newline scanning - would require re-parsing already-parsed content
/// - ❌ String allocation for each line - would create unnecessary memory overhead
///
/// **Efficient operations used:**
/// - ✅ `AsStrSlice` zero-copy slicing - creates views into original input
/// - ✅ Iterator-based character traversal - single pass through content
/// - ✅ Character-position-based line extraction - direct indexing without string copying
///
/// The `split_by_new_line_alt` function performs a single character-iterator pass to identify
/// line boundaries, then creates zero-copy `AsStrSlice` views for each line.
///
/// ## Sample Inputs and Outputs
///
/// | Scenario                  | Sample input                                               |
/// |---------------------------|------------------------------------------------------------|
/// | One line                  | `"```bash\npip install foobar\n```\n"`                     |
/// | No line                   | `"```bash\n```\n"`                                         |
/// | Multi line                | `"```bash\npip install foobar\npip install foobar\n```\n"` |
/// | No language               | `"```\npip install foobar\n```\n"`                         |
/// | No language, no line      | `"```\n```\n"`                                             |
/// | No language, multi line   | `"```\npip install foobar\npip install foobar\n```\n"`     |
///
/// ## Line Advancement
/// This is a **multi-line block parser that auto-advances**. It consumes all lines
/// from the opening ``` to the closing ``` and positions the remainder at the first
/// line after the code block ends.
///
/// ## Returns
/// - A tuple containing the remainder of the input and a `List<CodeBlockLine>` ready for rendering
/// - Each `CodeBlockLine` contains the parsed content and metadata for proper display
#[rustfmt::skip]
pub fn parse_block_code_advance_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, List<CodeBlockLine<'a>>> {
    let (remainder, (lang, code)) = (
        parse_code_block_lang_including_eol_alt,
        parse_code_block_body_including_code_block_end_alt,
    )
        .parse(input)?;

    // Normal case: if there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    let acc = split_by_new_line_alt(code);
    let lines = acc.as_slice();

    Ok((remainder, convert_into_code_block_lines_alt(lang, lines)))
}

/// Parse the language identifier from a code block's opening line.
/// Returns `Some(language)` if a language is specified, or `None` if no language is specified.
/// Consumes the [NEW_LINE] if it exists.
#[rustfmt::skip]
fn parse_code_block_lang_including_eol_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    alt((
        // Either - Successfully parse both code block language & text.
        map(
            preceded(
                /* prefix - discarded */ tag(CODE_BLOCK_START_PARTIAL),
                /* output */
                terminated(
                    /* match */ is_not(NEW_LINE),
                    /* ends with (discarded) */ tag(NEW_LINE),
                ),
            ),
            Some,
        ),
        // Or - Fail to parse language, use unknown language instead.
        map(
            (tag(CODE_BLOCK_START_PARTIAL), tag(NEW_LINE)),
            |_| None,
        ),
    )).parse(input)
}

/// Parse the body of a code block until the end marker is reached.
///
/// This function extracts all content between the current position and the code block end marker
/// (indicated by the [CODE_BLOCK_END] constant, which is "```").
///
/// The function:
/// 1. Captures all text until the end marker.
/// 2. Consumes the end marker itself (removing it from the returned content).
/// 3. Returns the remainder of the input and the captured content.
#[rustfmt::skip]
pub fn parse_code_block_body_including_code_block_end_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    // The extra trailing newline is added by AsStrSlice find_substring()
    // which is used in the `take_until` parser. This has to be removed from the end
    // using opt(tag(NEW_LINE)).
    let (remainder, (output, _)) = (
        terminated(
            take_until(CODE_BLOCK_END),
            /* end (discard) */ tag(CODE_BLOCK_END),
        ),
        opt(tag(NEW_LINE)),
    )
        .parse(input)?;
    Ok((remainder, output))
}

/// Converts a language identifier and a vector of code lines into a List of
/// [CodeBlockLine] objects. The resulting List will always contain at least 2
/// [CodeBlockLine] objects:
/// 1. A [CodeBlockLine] with content type StartTag representing the opening of the code
///    block.
/// 2. A [CodeBlockLine] with content type EndTag representing the closing of the code
///    block.
///
/// Any lines of code between the opening and closing tags will be converted to
/// [CodeBlockLine] objects with content type Text. The language identifier is attached to
/// all [CodeBlockLine] objects.
pub fn convert_into_code_block_lines_alt<'a>(
    maybe_lang: Option<AsStrSlice<'a>>,
    lines: &[AsStrSlice<'a>],
) -> List<CodeBlockLine<'a>> {
    let mut acc = List::with_capacity(lines.len() + 2);

    let lang = maybe_lang.map(|lang| lang.extract_to_line_end());

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::StartTag,
    };

    for line in lines {
        let text = line.extract_to_line_end();
        acc += CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::Text(text),
        };
    }

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::EndTag,
    };

    acc
}

/// Split an [AsStrSlice] by newline using an efficient iterator-based approach.
///
/// The idea is that a line is some text followed by a newline. An empty line is just a
/// newline character.
///
/// This function is the [AsStrSlice] equivalent of the original
/// [`crate::split_by_new_line`] function that works with string slices (`&str`).
///
/// ## Performance Optimization
///
/// This implementation uses a **single-pass, zero-allocation** iterator approach that
/// provides significant performance improvements over naive implementations:
///
/// ### Memory Efficiency
/// - **No memory allocation**: Works directly with character positions without
///   materializing the entire content into a contiguous string
/// - **Zero-copy**: Constructs `AsStrSlice` segments directly from character positions
///   using `take_from()` and `take()` methods
/// - **Constant memory usage**: Memory usage remains constant regardless of input size
///
/// ### Computational Efficiency
/// - **Single pass**: Iterates through characters exactly once, tracking newline
///   positions
/// - **O(n) time complexity**: Linear time complexity with respect to input size
/// - **No string materialization**: Avoids the overhead of calling
///   `extract_to_slice_end()` and `split('\n')` which would require multiple passes and
///   string allocation
///
/// ### Comparison with Alternative Approaches
///
/// **Naive approach** (avoided):
/// ```rust,ignore
/// // DON'T DO THIS - inefficient!
/// let full_content = input.extract_to_slice_end();  // 1st pass + allocation
/// let str_lines: Vec<&str> = full_content.split('\n').collect();  // 2nd pass
/// let result: Vec<AsStrSlice> = str_lines.iter()
///     .map(|line| /* convert back to AsStrSlice */)  // 3rd pass
///     .collect();
/// ```
///
/// **Optimized approach** (implemented):
/// ```rust,ignore
/// // Single pass through characters, direct AsStrSlice construction
/// let mut iter = input.iter_elements();  // Uses existing StringChars iterator
/// while let Some(ch) = iter.next() {     // Single pass
///     if ch == '\n' {
///         let line_slice = input.take_from(start).take(len);  // Zero-copy
///         acc.push(line_slice);
///     }
/// }
/// ```
///
/// ### Infrastructure Reuse
/// - **Leverages existing iterator**: Uses the battle-tested `StringChars` iterator from
///   `iterators.rs` via `input.iter_elements()`
/// - **Maintains Unicode safety**: Character-based iteration ensures proper handling of
///   multi-byte UTF-8 sequences like emojis
/// - **Consistent with codebase**: Follows the same patterns used throughout the
///   `AsStrSlice` implementation
///
/// ## Context: Why AsStrSlice behaves like a continuous string here
///
/// In this specific context, the [AsStrSlice] input represents a continuous text span
/// that may cross multiple underlying lines. This occurs because the input comes from
/// [`parse_code_block_body_including_code_block_end_alt`], which extracts the content
/// between code block markers "```" and "```" as a single continuous string.
///
/// For example, when parsing a markdown code block like:
///
/// - Line 1: "```python"
/// - Line 2: "import foobar"
/// - Line 3: ""
/// - Line 4: "foobar.pluralize('word')"
/// - Line 5: "```"
///
/// The [`parse_code_block_body_including_code_block_end_alt`] function returns an
/// [AsStrSlice] containing `"import foobar\n\nfoobar.pluralize('word')\n"` as a
/// continuous span. This function then splits this continuous content back into
/// individual lines for proper code block processing.
///
/// ## Implementation Notes
///
/// - **Method choice**: Uses `take_from()` and `take()` instead of
///   `skip_take_in_current_line()` for proper line boundary handling when crossing
///   multiple underlying lines
/// - **Unicode safety**: Character-based iteration ensures proper handling of multi-byte
///   UTF-8 sequences like emojis
/// - **Compatibility**: Maintains complete behavioral parity with the original
///   [`crate::split_by_new_line`] function for all edge cases
///
/// # Examples
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""]`               |
/// | ""             | `[]`                 |
/// | "foo\nbar\n"   | `["foo", "bar"]`     |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
pub fn split_by_new_line_alt<'a>(input: AsStrSlice<'a>) -> InlineVec<AsStrSlice<'a>> {
    if input.is_empty() {
        return InlineVec::new();
    }

    let mut acc = InlineVec::new();
    let mut line_start = 0;
    let mut current_pos = 0;

    // Iterator-based approach: single pass through characters to find newline positions
    // Using the existing StringChars iterator from iterators.rs
    let mut iter = input.iter_elements();

    while let Some(ch) = iter.next() {
        if ch == NEW_LINE_CHAR {
            // Found a newline - create a slice from line_start to current_pos
            let line_length = current_pos - line_start;
            let line_slice = input.take_from(line_start).take(line_length);
            acc.push(line_slice);

            // Move to start of next line (after the newline)
            current_pos += 1;
            line_start = current_pos;
        } else {
            current_pos += 1;
        }
    }

    // Handle any remaining content after the last newline (or if there were no newlines)
    if line_start < current_pos {
        let line_length = current_pos - line_start;
        let line_slice = input.take_from(line_start).take(line_length);
        acc.push(line_slice);
    }

    acc
}

/// Look at the similar tests in the `tests_parse_block_code_alt_lines` module.
#[cfg(test)]
mod tests_parse_block_code_alt_single_line {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, convert_into_code_block_lines};

    #[test]
    fn test_parse_codeblock_trailing_extra() {
        as_str_slice_test_case!(input, "```bash", "pip install foobar", "```");
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_slice_end().as_ref(), "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_single_line() {
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```bash", "pip install foobar", "```", "");

        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_no_content() {
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(input, "```bash", "```", "");

        let code_lines = vec![];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_single_empty_line() {
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(empty_line, "");
        as_str_slice_test_case!(input, "```bash", "", "```", "");

        let code_lines = vec![empty_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_multiple_lines() {
        as_str_slice_test_case!(
            input,
            "```python",
            "import foobar",
            "",
            "foobar.pluralize('word') # returns 'words'",
            "foobar.pluralize('goose') # returns 'geese'",
            "foobar.singularize('phenomena') # returns 'phenomenon'",
            "```",
            ""
        );

        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();

        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ]
            )
        );
    }

    #[test]
    fn test_parse_codeblock_no_language() {
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```", "pip install foobar", "```", "");

        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(None, &code_lines)
        );
    }
}

/// These tests are very similar to the tests in `tests_parse_block_code_alt` module.
/// There is a key difference. These tests simulate what real input from the
/// [crate::EditorContent] looks like. The editor reads a file and calls `.lines()` on it,
/// which strips any trailing [NEW_LINE] lines. Here's an example to demonstrate this:
/// ```
/// let input = "```bash\npip install foobar\n```\n";
/// let count = input.lines().count(); // Last "\n" gets eaten by lines()
/// assert_eq!(count, 3);
/// let lines = input.lines().collect::<Vec<_>>();
/// assert_eq!(lines[0], "```bash");
/// assert_eq!(lines[0], "pip install foobar");
/// assert_eq!(lines[0], "```");
/// ```
#[cfg(test)]
mod tests_parse_block_code_alt_lines {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_parse_codeblock_trailing_extra_lines() {
        as_str_slice_test_case!(input, "```bash", "pip install foobar", "```");
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_slice_end().as_ref(), "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_single_line() {
        // One line: "```bash\npip install foobar\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```bash", "pip install foobar", "```");

        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_no_content() {
        // No line: "```bash\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(input, "```bash", "```");

        let code_lines = vec![];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_single_empty_line() {
        // 1 empty line: "```bash\n\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(empty_line, "");
        as_str_slice_test_case!(input, "```bash", "", "```");

        let code_lines = vec![empty_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_multiple_lines() {
        // Multiple lines.
        as_str_slice_test_case!(lang_slice, "python");
        as_str_slice_test_case!(line1, "import foobar");
        as_str_slice_test_case!(line2, "");
        as_str_slice_test_case!(line3, "foobar.pluralize('word') # returns 'words'");
        as_str_slice_test_case!(line4, "foobar.pluralize('goose') # returns 'geese'");
        as_str_slice_test_case!(
            line5,
            "foobar.singularize('phenomena') # returns 'phenomenon'"
        );
        as_str_slice_test_case!(
            input,
            "```python",
            "import foobar",
            "",
            "foobar.pluralize('word') # returns 'words'",
            "foobar.pluralize('goose') # returns 'geese'",
            "foobar.singularize('phenomena') # returns 'phenomenon'",
            "```",
            ""
        );

        let code_lines = vec![line1, line2, line3, line4, line5];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), &code_lines)
        );
        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_parse_codeblock_no_language_lines() {
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```", "pip install foobar", "```");

        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(None, &code_lines)
        );
    }
}

#[cfg(test)]
mod tests_parse_code_block_lang_including_eol_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_parse_code_block_lang_with_language() {
        // Test with language specified
        {
            as_str_slice_test_case!(input, "```rust", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "rust");
        }
    }

    #[test]
    fn test_parse_code_block_lang_no_language() {
        // Test with no language specified (just ``` followed by newline)
        {
            as_str_slice_test_case!(input, "```", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_none(), true);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_remainder() {
        // Test with language and content after newline
        {
            as_str_slice_test_case!(input, "```python", "print('hello')", "```");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            dbg!(&remainder.extract_to_slice_end());
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "print('hello')\n```\n"
            );
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "python");
        }
    }

    #[test]
    fn test_parse_code_block_lang_empty_language() {
        // Test with empty language (``` followed immediately by newline)
        {
            as_str_slice_test_case!(input, "```", "some code here");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "some code here\n"
            );
            assert_eq2!(lang.is_none(), true);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_spaces() {
        // Test with language that has spaces/attributes
        {
            as_str_slice_test_case!(input, "```javascript {.line-numbers}", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(
                lang.unwrap().extract_to_slice_end().as_ref(),
                "javascript {.line-numbers}"
            );
        }
    }

    #[test]
    fn test_parse_code_block_lang_common_languages() {
        // Test various common programming languages
        let test_cases = [
            "rust",
            "python",
            "javascript",
            "typescript",
            "java",
            "cpp",
            "c",
            "html",
            "css",
            "json",
            "yaml",
            "toml",
            "xml",
            "bash",
            "sh",
            "sql",
        ];

        for lang in test_cases {
            as_str_slice_test_case!(input, format!("```{lang}"), "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, parsed_lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(parsed_lang.is_some(), true);
            assert_eq2!(parsed_lang.unwrap().extract_to_slice_end().as_ref(), lang);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_numbers() {
        // Test language identifier with numbers
        {
            as_str_slice_test_case!(input, "```c++11", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "c++11");
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_dashes() {
        // Test language identifier with dashes/hyphens
        {
            as_str_slice_test_case!(input, "```objective-c", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "objective-c");
        }
    }

    #[test]
    fn test_parse_code_block_lang_missing_newline_error() {
        // Test error case when newline is missing
        {
            as_str_slice_test_case!(input, "```rust some code without newline");
            let result = parse_code_block_lang_including_eol_alt(input);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_missing_backticks_error() {
        // Test error case when CODE_BLOCK_START_PARTIAL is missing
        {
            as_str_slice_test_case!(input, "rust", "");
            let result = parse_code_block_lang_including_eol_alt(input);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_partial_backticks_error() {
        // Test error case with incomplete backticks
        {
            as_str_slice_test_case!(input, "``rust", "");
            let result = parse_code_block_lang_including_eol_alt(input);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_case_sensitive() {
        // Test that language parsing is case sensitive
        {
            as_str_slice_test_case!(input, "```RUST", "");
            let result = parse_code_block_lang_including_eol_alt(input);
            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "RUST");
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_attributes() {
        // Test with GitHub-style language attributes
        {
            as_str_slice_test_case!(input, "```rust,ignore", "");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "rust,ignore");
        }
    }

    #[test]
    fn test_parse_code_block_lang_only_backticks() {
        // Test edge case with only the starting backticks
        {
            as_str_slice_test_case!(input, "```");
            let result = parse_code_block_lang_including_eol_alt(input);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_unicode_language() {
        // Test with unicode characters in language identifier (though uncommon)
        {
            as_str_slice_test_case!(input, "```语言", "");
            let result = parse_code_block_lang_including_eol_alt(input);
            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), false);
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "\n");
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "语言");
        }
    }
}

#[cfg(test)]
mod tests_parse_code_block_body_including_code_block_end_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_parse_code_block_body_simple_case() {
        // Test basic case with code content and closing tag
        {
            as_str_slice_test_case!(
                input,
                "fn main() {",
                "    println!(\"Hello\");",
                "}",
                "```"
            );
            let result = parse_code_block_body_including_code_block_end_alt(input);
            let (remainder, body) = result.unwrap();
            dbg!(&body.extract_to_slice_end().as_ref());
            assert_eq2!(
                body.extract_to_slice_end().as_ref(),
                "fn main() {\n    println!(\"Hello\");\n}\n"
            );
            assert_eq2!(remainder.is_empty(), true);
        }
    }

    #[test]
    fn test_parse_code_block_body_empty_content() {
        // Test with empty code block (only closing tag)
        {
            as_str_slice_test_case!(input, "```");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(body.extract_to_slice_end().as_ref(), "");
        }
    }

    #[test]
    fn test_parse_code_block_body_single_line() {
        // Test with single line of code
        {
            as_str_slice_test_case!(input, "let x = 42;```");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(body.extract_to_slice_end().as_ref(), "let x = 42;");
        }
    }

    #[test]
    fn test_parse_code_block_body_with_remainder() {
        // Test with content after the closing tag
        {
            as_str_slice_test_case!(input, "console.log('test');```", "Some text after");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(body.extract_to_slice_end().as_ref(), "console.log('test');");
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "Some text after\n"
            );
        }
    }

    #[test]
    fn test_parse_code_block_body_multiline_with_newlines() {
        // Test with multiple lines including empty lines
        {
            as_str_slice_test_case!(input, "line1", "", "", "line3", "", "```");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            dbg!(&body.extract_to_slice_end().as_ref());
            assert_eq2!(body.extract_to_slice_end().as_ref(), "line1\n\n\nline3\n\n");
        }
    }

    #[test]
    fn test_parse_code_block_body_with_backticks_in_content() {
        // Test with backticks that are not the closing tag
        {
            as_str_slice_test_case!(
                input,
                "let code = `template string`;",
                "let other = `another`;",
                "```"
            );
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                body.extract_to_slice_end().as_ref(),
                "let code = `template string`;\nlet other = `another`;\n"
            );
        }
    }

    #[test]
    fn test_parse_code_block_body_missing_end_tag() {
        // Test error case when closing tag is missing
        {
            as_str_slice_test_case!(input, "some code without closing tag");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_body_immediate_closing() {
        // Test with closing tag immediately at the start
        {
            as_str_slice_test_case!(input, "```more content");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "more content");
            assert_eq2!(body.extract_to_slice_end().as_ref(), "");
        }
    }

    #[test]
    fn test_parse_code_block_body_unicode_content() {
        // Test that the parser now correctly handles unicode characters in code content
        // This demonstrates that our improved find_substring implementation supports
        // Unicode
        {
            // cspell:disable
            as_str_slice_test_case!(
                input,
                "let emoji = \"😀🚀\";",
                "let unicode = \"αβγδε\";",
                "```"
            );
            // cspell:enable
            let result = parse_code_block_body_including_code_block_end_alt(input);
            assert!(
                result.is_ok(),
                "Parser should now handle Unicode content correctly"
            );

            let (remainder, body) = result.unwrap();
            assert!(
                remainder.is_empty(),
                "Should consume all input including closing backticks"
            );

            let body_content = body.extract_to_slice_end();
            let expected_content = "let emoji = \"😀🚀\";\nlet unicode = \"αβγδε\";\n";
            assert_eq2!(body_content.as_ref(), expected_content);
        }
    }

    #[test]
    fn test_parse_code_block_body_special_characters() {
        // Test with special characters and symbols
        {
            as_str_slice_test_case!(
                input,
                "#!/bin/bash",
                "echo \"$USER @ $(hostname)\"",
                "```"
            );
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                body.extract_to_slice_end().as_ref(),
                "#!/bin/bash\necho \"$USER @ $(hostname)\"\n"
            );
        }
    }

    #[test]
    fn test_parse_code_block_body_only_whitespace() {
        // Test with only whitespace content
        {
            as_str_slice_test_case!(input, "   ", "\t", "  ```");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(body.extract_to_slice_end().as_ref(), "   \n\t\n  ");
        }
    }
}

#[cfg(test)]
mod tests_convert_into_code_block_lines_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, CodeBlockLineContent};

    #[test]
    fn test_convert_into_code_block_lines_alt_with_language() {
        // Test with language and content lines
        {
            as_str_slice_test_case!(lang, "rust");
            as_str_slice_test_case!(line1, "fn main() {");
            as_str_slice_test_case!(line2, "    println!(\"Hello, world!\");");
            as_str_slice_test_case!(line3, "}");

            let lines = vec![line1, line2, line3];
            let result = convert_into_code_block_lines_alt(Some(lang), &lines);

            assert_eq2!(result.len(), 5); // start + 3 content + end

            // Check start tag
            assert_eq2!(result[0].language, Some("rust"));
            assert_eq2!(result[0].content, CodeBlockLineContent::StartTag);

            // Check content lines
            assert_eq2!(result[1].language, Some("rust"));
            assert_eq2!(result[1].content, CodeBlockLineContent::Text("fn main() {"));

            assert_eq2!(result[2].language, Some("rust"));
            assert_eq2!(
                result[2].content,
                CodeBlockLineContent::Text("    println!(\"Hello, world!\");")
            );

            assert_eq2!(result[3].language, Some("rust"));
            assert_eq2!(result[3].content, CodeBlockLineContent::Text("}"));

            // Check end tag
            assert_eq2!(result[4].language, Some("rust"));
            assert_eq2!(result[4].content, CodeBlockLineContent::EndTag);
        }
    }

    #[test]
    fn test_convert_into_code_block_lines_alt_without_language() {
        // Test without language
        {
            as_str_slice_test_case!(line1, "some code");
            as_str_slice_test_case!(line2, "more code");

            let lines = vec![line1, line2];
            let result = convert_into_code_block_lines_alt(None, &lines);

            assert_eq2!(result.len(), 4); // start + 2 content + end

            // Check start tag
            assert_eq2!(result[0].language, None);
            assert_eq2!(result[0].content, CodeBlockLineContent::StartTag);

            // Check content lines
            assert_eq2!(result[1].language, None);
            assert_eq2!(result[1].content, CodeBlockLineContent::Text("some code"));

            assert_eq2!(result[2].language, None);
            assert_eq2!(result[2].content, CodeBlockLineContent::Text("more code"));

            // Check end tag
            assert_eq2!(result[3].language, None);
            assert_eq2!(result[3].content, CodeBlockLineContent::EndTag);
        }
    }

    #[test]
    fn test_convert_into_code_block_lines_alt_empty_content() {
        // Test with no content lines
        {
            as_str_slice_test_case!(lang, "python");

            let lines = vec![];
            let result = convert_into_code_block_lines_alt(Some(lang), &lines);

            assert_eq2!(result.len(), 2); // start + end only

            // Check start tag
            assert_eq2!(result[0].language, Some("python"));
            assert_eq2!(result[0].content, CodeBlockLineContent::StartTag);

            // Check end tag
            assert_eq2!(result[1].language, Some("python"));
            assert_eq2!(result[1].content, CodeBlockLineContent::EndTag);
        }
    }

    #[test]
    fn test_convert_into_code_block_lines_alt_empty_lines() {
        // Test with empty content lines
        {
            as_str_slice_test_case!(lang, "javascript");
            as_str_slice_test_case!(empty_line, "");
            as_str_slice_test_case!(another_empty, "");

            let lines = vec![empty_line, another_empty];
            let result = convert_into_code_block_lines_alt(Some(lang), &lines);

            assert_eq2!(result.len(), 4); // start + 2 empty content + end

            // Check start tag
            assert_eq2!(result[0].language, Some("javascript"));
            assert_eq2!(result[0].content, CodeBlockLineContent::StartTag);

            // Check empty content lines
            assert_eq2!(result[1].language, Some("javascript"));
            assert_eq2!(result[1].content, CodeBlockLineContent::Text(""));

            assert_eq2!(result[2].language, Some("javascript"));
            assert_eq2!(result[2].content, CodeBlockLineContent::Text(""));

            // Check end tag
            assert_eq2!(result[3].language, Some("javascript"));
            assert_eq2!(result[3].content, CodeBlockLineContent::EndTag);
        }
    }

    #[test]
    fn test_convert_into_code_block_lines_alt_single_line() {
        // Test with single content line
        {
            as_str_slice_test_case!(lang, "bash");
            as_str_slice_test_case!(single_line, "echo 'Hello World'");

            let lines = vec![single_line];
            let result = convert_into_code_block_lines_alt(Some(lang), &lines);

            assert_eq2!(result.len(), 3); // start + 1 content + end

            // Check start tag
            assert_eq2!(result[0].language, Some("bash"));
            assert_eq2!(result[0].content, CodeBlockLineContent::StartTag);

            // Check content line
            assert_eq2!(result[1].language, Some("bash"));
            assert_eq2!(
                result[1].content,
                CodeBlockLineContent::Text("echo 'Hello World'")
            );

            // Check end tag
            assert_eq2!(result[2].language, Some("bash"));
            assert_eq2!(result[2].content, CodeBlockLineContent::EndTag);
        }
    }
}

#[cfg(test)]
mod tests_split_by_new_line_alt {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    // Helper function to convert AsStrSlice results to strings for comparison
    fn slice_results_to_strings(slices: &[AsStrSlice<'_>]) -> Vec<String> {
        slices
            .into_iter()
            .map(|slice| slice.extract_to_slice_end().to_string())
            .collect()
    }

    #[test]
    fn test_parse_codeblock_split_by_eol_alt() {
        // Test "foobar\n" -> ["foobar"]
        {
            as_str_slice_test_case!(input, "foobar\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(&result);
            assert_eq2!(result_strings, vec!["foobar"]);
        }

        // Test "\n" -> [""]
        {
            as_str_slice_test_case!(input, "\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(&result);
            assert_eq2!(result_strings, vec![""]);
        }

        // Test "" -> []
        {
            as_str_slice_test_case!(input, "");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(&result);
            assert_eq2!(result_strings, Vec::<String>::new());
        }

        // Test "foo\nbar\n" -> ["foo", "bar"]
        {
            as_str_slice_test_case!(input, "foo\nbar\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(&result);
            assert_eq2!(result_strings, vec!["foo", "bar"]);
        }

        // Test "\nfoo\nbar\n" -> ["", "foo", "bar"]
        {
            as_str_slice_test_case!(input, "\nfoo\nbar\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(&result);
            assert_eq2!(result_strings, vec!["", "foo", "bar"]);
        }
    }
}

#[cfg(test)]
mod tests_compat_with_original_split_by_new_line {
    use super::*;
    use crate::{as_str_slice_test_case,
                assert_eq2,
                split_by_new_line,
                AsStrSlice,
                GCString};

    // Helper function to convert AsStrSlice results to strings for easy comparison
    fn slice_results_to_strings(slices: &[AsStrSlice<'_>]) -> Vec<String> {
        slices
            .into_iter()
            .map(|slice| slice.extract_to_slice_end().to_string())
            .collect()
    }

    #[test]
    fn test_parity_empty_string() {
        // Test with original function
        let str_result = split_by_new_line("");

        // Test with alt function
        as_str_slice_test_case!(input, "");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result.len(), 0);
    }

    #[test]
    fn test_parity_single_newline() {
        // Test with original function
        let str_result = split_by_new_line("\n");

        // Test with alt function
        as_str_slice_test_case!(input, "\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec![""]);
    }

    #[test]
    fn test_parity_content_with_trailing_newline() {
        // Test with original function
        let str_result = split_by_new_line("foobar\n");

        // Test with alt function
        as_str_slice_test_case!(input, "foobar\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["foobar"]);
    }

    #[test]
    fn test_parity_multiple_lines_with_trailing_newline() {
        // Test with original function
        let str_result = split_by_new_line("foo\nbar\n");

        // Test with alt function
        as_str_slice_test_case!(input, "foo\nbar\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["foo", "bar"]);
    }

    #[test]
    fn test_parity_leading_newline() {
        // Test with original function
        let str_result = split_by_new_line("\nfoo\nbar\n");

        // Test with alt function
        as_str_slice_test_case!(input, "\nfoo\nbar\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["", "foo", "bar"]);
    }

    #[test]
    fn test_parity_no_trailing_newline() {
        // Test with original function
        let str_result = split_by_new_line("foo\nbar");

        // Test with alt function
        as_str_slice_test_case!(input, "foo\nbar");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["foo", "bar"]);
    }

    #[test]
    fn test_parity_multiple_empty_lines() {
        // Test with original function
        let str_result = split_by_new_line("\n\n\n");

        // Test with alt function
        as_str_slice_test_case!(input, "\n\n\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["", "", ""]);
    }

    #[test]
    fn test_parity_mixed_empty_and_content_lines() {
        // Test with original function
        let str_result = split_by_new_line("foo\n\nbar\n\n");

        // Test with alt function
        as_str_slice_test_case!(input, "foo\n\nbar\n\n");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["foo", "", "bar", ""]);
    }

    #[test]
    fn test_parity_single_character() {
        // Test with original function
        let str_result = split_by_new_line("a");

        // Test with alt function
        as_str_slice_test_case!(input, "a");
        let alt_result = split_by_new_line_alt(input);
        let alt_result_strings = slice_results_to_strings(&alt_result);

        // Verify parity
        assert_eq2!(str_result, alt_result_strings);
        assert_eq2!(str_result, vec!["a"]);
    }

    #[test]
    fn test_alt_function_preserves_as_str_slice_properties() {
        // Test that the returned AsStrSlice instances maintain correct properties
        as_str_slice_test_case!(input, "foo\nbar\nbaz\n");
        let input_clone = input.clone();
        let results = split_by_new_line_alt(input);

        assert_eq2!(results.len(), 3);

        // Verify each result slice has the correct content
        assert_eq2!(results[0].extract_to_slice_end().as_ref(), "foo");
        assert_eq2!(results[1].extract_to_slice_end().as_ref(), "bar");
        assert_eq2!(results[2].extract_to_slice_end().as_ref(), "baz");

        // Verify they reference the same underlying lines
        assert_eq2!(results[0].lines, input_clone.lines);
        assert_eq2!(results[1].lines, input_clone.lines);
        assert_eq2!(results[2].lines, input_clone.lines);
    }

    #[test]
    fn test_comprehensive_parity_test() {
        // Test a variety of inputs to ensure complete parity
        let test_cases = vec![
            "",
            "\n",
            "a",
            "a\n",
            "\na",
            "\na\n",
            "foo",
            "foo\n",
            "\nfoo",
            "\nfoo\n",
            "foo\nbar",
            "foo\nbar\n",
            "\nfoo\nbar",
            "\nfoo\nbar\n",
            "foo\n\nbar",
            "foo\n\nbar\n\n",
            "\n\n\n",
            "a\nb\nc\nd\ne\n",
        ];

        for test_input in test_cases {
            // Test with original function
            let str_result = split_by_new_line(test_input);

            // Test with alt function
            let input_lines = vec![GCString::new(test_input)];
            let input_slice = AsStrSlice::from(&input_lines);
            let alt_result = split_by_new_line_alt(input_slice);
            let alt_result_strings = slice_results_to_strings(&alt_result);

            // Verify parity
            assert_eq2!(
                str_result,
                alt_result_strings,
                "Parity failed for input: {:?}",
                test_input
            );
        }
    }
}

#[cfg(test)]
mod debug_tests {
    use super::*;
    use crate::as_str_slice_test_case;

    #[test]
    fn debug_test_parse_codeblock_multiple_lines() {
        as_str_slice_test_case!(
            input,
            "```python",
            "import foobar",
            "",
            "foobar.pluralize('word') # returns 'words'",
            "foobar.pluralize('goose') # returns 'geese'",
            "foobar.singularize('phenomena') # returns 'phenomenon'",
            "```",
            ""
        );

        println!("Input: {:?}", input.extract_to_slice_end().as_ref());

        // Test the language parsing first
        let input_clone = input.clone();
        let (remainder_after_lang, lang) =
            parse_code_block_lang_including_eol_alt(input_clone).unwrap();
        let lang_str = lang.map(|l| l.extract_to_slice_end().as_ref().to_string());
        println!("Language: {:?}", lang_str);
        println!(
            "Remainder after lang: {:?}",
            remainder_after_lang.extract_to_slice_end().as_ref()
        );

        // Test the body parsing
        let (remainder_after_body, body) =
            parse_code_block_body_including_code_block_end_alt(remainder_after_lang)
                .unwrap();
        println!("Body: {:?}", body.extract_to_slice_end().as_ref());
        println!(
            "Remainder after body: {:?}",
            remainder_after_body.extract_to_slice_end().as_ref()
        );

        // Test the splitting
        let split_result = split_by_new_line_alt(body);
        println!("Split result count: {}", split_result.len());
        for (i, split) in split_result.iter().enumerate() {
            println!("Split[{}]: {:?}", i, split.extract_to_slice_end().as_ref());
        }

        // Test the full parsing
        let (_remainder, code_block_lines) = parse_block_code_advance_alt(input).unwrap();
        println!("Final code block lines count: {}", code_block_lines.len());
        for (i, line) in code_block_lines.iter().enumerate() {
            println!("Line[{}]: {:?}", i, line);
        }
    }
}
