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
          Parser};

use crate::{md_parser::constants::{CODE_BLOCK_END, CODE_BLOCK_START_PARTIAL, NEW_LINE},
            AsStrSlice,
            CodeBlockLine,
            CodeBlockLineContent,
            List};

/// Parses a Markdown code block and returns a [List] of [CodeBlockLine] objects.
///
/// This function handles various code block formats including those with or without:
/// - Language specification.
/// - Content lines.
/// - Trailing newlines.
///
/// Sample inputs:
///
/// | Scenario                  | Sample input                                               |
/// |---------------------------|------------------------------------------------------------|
/// | One line                  | `"```bash\npip install foobar\n```\n"`                     |
/// | No line                   | `"```bash\n```\n"`                                         |
/// | Multi line                | `"```bash\npip install foobar\npip install foobar\n```\n"` |
/// | No language               | `"```\npip install foobar\n```\n"`                         |
/// | No language, no line      | `"```\n```\n"`                                             |
/// | No language, multi line   | `"```\npip install foobar\npip install foobar\n```\n"`     |
#[rustfmt::skip]
pub fn parse_block_code_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, List<CodeBlockLine<'a>>> {
    let (remainder, (lang, code)) = (
        parse_code_block_lang_including_eol_alt,
        parse_code_block_body_including_code_block_end_alt,
    )
        .parse(input)?;

    // Normal case: if there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    let acc = split_by_new_line_alt(code);

    Ok((remainder, convert_into_code_block_lines_alt(lang, acc)))
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
fn parse_code_block_body_including_code_block_end_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    let (remainder, output) = terminated(
        take_until(CODE_BLOCK_END),
        /* end (discard) */ tag(CODE_BLOCK_END),
    ).parse(input)?;
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
fn convert_into_code_block_lines_alt<'a>(
    maybe_lang: Option<AsStrSlice<'a>>,
    lines: Vec<AsStrSlice<'a>>,
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

/// Split an [AsStrSlice] by newline. The idea is that a line is some text followed by a
/// newline. An empty line is just a newline character.
///
/// This function is the [AsStrSlice] equivalent of the original `split_by_new_line`
/// function that works with string slices (&str).
///
/// # Examples:
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""]`               |
/// | ""             | `[]`                 |
/// | "foo\nbar\n"   | `["foo", "bar"]`     |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
fn split_by_new_line_alt<'a>(input: AsStrSlice<'a>) -> Vec<AsStrSlice<'a>> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let full_content = input.extract_to_slice_end();
    let content_str = full_content.as_ref();

    // Split the string content and apply the same logic as the original.
    let mut string_splits: Vec<&str> = content_str.split('\n').collect();
    if let Some(last_item) = string_splits.last() {
        if last_item.is_empty() {
            string_splits.pop();
        }
    }

    // Convert each string slice back to AsStrSlice.
    let mut current_offset = 0;
    for split_str in string_splits {
        if !split_str.is_empty() {
            // ⚠️ CRITICAL: Convert byte length to character count
            // split_str.len() returns BYTE count, but skip_take() expects CHARACTER count
            let char_count = split_str.chars().count();
            let segment_slice = input.skip_take(current_offset, char_count);
            result.push(segment_slice);
        } else {
            // Handle empty segments (from "\n" cases).
            let segment_slice = input.skip_take(current_offset, 0);
            result.push(segment_slice);
        }
        // Move past this segment and the newline character.
        // ⚠️ CRITICAL: Use character count, not byte count
        let char_count = split_str.chars().count();
        current_offset += char_count + 1; // +1 for the newline character
    }

    result
}

/// Look at the similar tests in the `tests_parse_block_code_alt_lines` module.
#[cfg(test)]
mod tests_parse_block_code_alt_single_line {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2};

    #[test]
    fn test_parse_codeblock_trailing_extra() {
        as_str_slice_test_case!(input, "```bash\npip install foobar\n```");
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_slice_end().as_ref(), "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock() {
        // One line: "```bash\npip install foobar\n```\n"
        {
            as_str_slice_test_case!(lang_slice, "bash");
            as_str_slice_test_case!(code_line, "pip install foobar");
            as_str_slice_test_case!(input, "```bash\npip install foobar\n```\n");

            let code_lines = vec![code_line];
            let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
            );
        }

        // No line: "```bash\n```\n"
        {
            as_str_slice_test_case!(lang_slice, "bash");
            as_str_slice_test_case!(input, "```bash\n```\n");

            let code_lines = vec![];
            let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
            );
        }

        // 1 empty line: "```bash\n\n```\n"
        {
            as_str_slice_test_case!(lang_slice, "bash");
            as_str_slice_test_case!(empty_line, "");
            as_str_slice_test_case!(input, "```bash\n\n```\n");

            let code_lines = vec![empty_line];
            let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
            );
        }

        // Multiple lines.
        {
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
                "```python\nimport foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```\n"
            );

            let code_lines = vec![line1, line2, line3, line4, line5];
            let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
            );
        }
    }

    #[test]
    fn test_parse_codeblock_no_language() {
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```\npip install foobar\n```\n");

        let code_lines = vec![code_line];
        let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(remainder.is_empty(), true);
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(None, code_lines)
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
        let (remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(remainder.extract_to_slice_end().as_ref(), "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_single_line() {
        // One line: "```bash\npip install foobar\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```bash", "pip install foobar", "```");

        let code_lines = vec![code_line];
        let (_remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_no_content() {
        // No line: "```bash\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(input, "```bash", "```");

        let code_lines = vec![];
        let (_remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_single_empty_line() {
        // 1 empty line: "```bash\n\n```\n"
        as_str_slice_test_case!(lang_slice, "bash");
        as_str_slice_test_case!(empty_line, "");
        as_str_slice_test_case!(input, "```bash", "", "```");

        let code_lines = vec![empty_line];
        let (_remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_lines_multiple_lines() {
        // Multiple lines.
        as_str_slice_test_case!(lang_slice, "python");
        as_str_slice_test_case!(line1, "import foobar");
        as_str_slice_test_case!(line2, "");
        as_str_slice_test_case!(empty_line3, "");
        as_str_slice_test_case!(empty_line4, "");
        as_str_slice_test_case!(empty_line5, "");
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

        let code_lines = vec![line1, line2, empty_line3, empty_line4, empty_line5];
        let (_remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(Some(lang_slice), code_lines)
        );
    }

    #[test]
    fn test_parse_codeblock_no_language_lines() {
        as_str_slice_test_case!(code_line, "pip install foobar");
        as_str_slice_test_case!(input, "```", "pip install foobar", "```");

        let code_lines = vec![code_line];
        let (_remainder, code_block_lines) = parse_block_code_alt(input).unwrap();
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines_alt(None, code_lines)
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
            as_str_slice_test_case!(input, "```rust\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "rust");
        }
    }

    #[test]
    fn test_parse_code_block_lang_no_language() {
        // Test with no language specified (just ``` followed by newline)
        {
            as_str_slice_test_case!(input, "```\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(lang.is_none(), true);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_remainder() {
        // Test with language and content after newline
        {
            as_str_slice_test_case!(input, "```python\nprint('hello')\n```");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "print('hello')\n```"
            );
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "python");
        }
    }

    #[test]
    fn test_parse_code_block_lang_empty_language() {
        // Test with empty language (``` followed immediately by newline)
        {
            as_str_slice_test_case!(input, "```\nsome code here");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.extract_to_slice_end().as_ref(), "some code here");
            assert_eq2!(lang.is_none(), true);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_spaces() {
        // Test with language that has spaces/attributes
        {
            as_str_slice_test_case!(input, "```javascript {.line-numbers}\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, format!("```{}\n", lang));
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, parsed_lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(parsed_lang.is_some(), true);
            assert_eq2!(parsed_lang.unwrap().extract_to_slice_end().as_ref(), lang);
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_numbers() {
        // Test language identifier with numbers
        {
            as_str_slice_test_case!(input, "```c++11\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "c++11");
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_dashes() {
        // Test language identifier with dashes/hyphens
        {
            as_str_slice_test_case!(input, "```objective-c\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, "rust\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_partial_backticks_error() {
        // Test error case with incomplete backticks
        {
            as_str_slice_test_case!(input, "``rust\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_lang_case_sensitive() {
        // Test that language parsing is case sensitive
        {
            as_str_slice_test_case!(input, "```RUST\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(lang.is_some(), true);
            assert_eq2!(lang.unwrap().extract_to_slice_end().as_ref(), "RUST");
        }
    }

    #[test]
    fn test_parse_code_block_lang_with_attributes() {
        // Test with GitHub-style language attributes
        {
            as_str_slice_test_case!(input, "```rust,ignore\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
            as_str_slice_test_case!(input, "```语言\n");
            let result = parse_code_block_lang_including_eol_alt(input);

            let (remainder, lang) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
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
                "fn main() {\n    println!(\"Hello\");\n}\n```"
            );
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(
                body.extract_to_slice_end().as_ref(),
                "fn main() {\n    println!(\"Hello\");\n}\n"
            );
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
            as_str_slice_test_case!(input, "console.log('test');```\nSome text after");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(
                remainder.extract_to_slice_end().as_ref(),
                "\nSome text after"
            );
            assert_eq2!(body.extract_to_slice_end().as_ref(), "console.log('test');");
        }
    }

    #[test]
    fn test_parse_code_block_body_multiline_with_newlines() {
        // Test with multiple lines including empty lines
        {
            as_str_slice_test_case!(input, "line1\n\nline3\n```");
            let result = parse_code_block_body_including_code_block_end_alt(input);

            let (remainder, body) = result.unwrap();
            assert_eq2!(remainder.is_empty(), true);
            assert_eq2!(body.extract_to_slice_end().as_ref(), "line1\n\nline3\n");
        }
    }

    #[test]
    fn test_parse_code_block_body_with_backticks_in_content() {
        // Test with backticks that are not the closing tag
        {
            as_str_slice_test_case!(
                input,
                "let code = `template string`;\nlet other = `another`;\n```"
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
        // Test that the parser fails when given unicode characters in code content
        // This is expected behavior as the current implementation doesn't support unicode
        {
            // cspell:disable
            as_str_slice_test_case!(
                input,
                "let emoji = \"😀🚀\";\nlet unicode = \"αβγδε\";\n```"
            );
            // cspell:enable
            let result = parse_code_block_body_including_code_block_end_alt(input);
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_parse_code_block_body_special_characters() {
        // Test with special characters and symbols
        {
            as_str_slice_test_case!(
                input,
                "#!/bin/bash\necho \"$USER @ $(hostname)\"\n```"
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
            as_str_slice_test_case!(input, "   \n\t\n  ```");
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
            let result = convert_into_code_block_lines_alt(Some(lang), lines);

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
            let result = convert_into_code_block_lines_alt(None, lines);

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
            let result = convert_into_code_block_lines_alt(Some(lang), lines);

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
            let result = convert_into_code_block_lines_alt(Some(lang), lines);

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
            let result = convert_into_code_block_lines_alt(Some(lang), lines);

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
    fn slice_results_to_strings(slices: Vec<AsStrSlice<'_>>) -> Vec<String> {
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
            let result_strings = slice_results_to_strings(result);
            assert_eq2!(result_strings, vec!["foobar"]);
        }

        // Test "\n" -> [""]
        {
            as_str_slice_test_case!(input, "\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(result);
            assert_eq2!(result_strings, vec![""]);
        }

        // Test "" -> []
        {
            as_str_slice_test_case!(input, "");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(result);
            assert_eq2!(result_strings, Vec::<String>::new());
        }

        // Test "foo\nbar\n" -> ["foo", "bar"]
        {
            as_str_slice_test_case!(input, "foo\nbar\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(result);
            assert_eq2!(result_strings, vec!["foo", "bar"]);
        }

        // Test "\nfoo\nbar\n" -> ["", "foo", "bar"]
        {
            as_str_slice_test_case!(input, "\nfoo\nbar\n");
            let result = split_by_new_line_alt(input);
            let result_strings = slice_results_to_strings(result);
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
    fn slice_results_to_strings(slices: Vec<AsStrSlice<'_>>) -> Vec<String> {
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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
        let alt_result_strings = slice_results_to_strings(alt_result);

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
            let alt_result_strings = slice_results_to_strings(alt_result);

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
