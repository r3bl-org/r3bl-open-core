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

/// Take text until an optional EOL character is found, or end of input is reached.
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

/// Parse the body of a code block until the end of the code block is reached.
/// The end of the code block is indicated by the [CODE_BLOCK_END] constant.
/// Consumes the [CODE_BLOCK_END] if it exists.
#[rustfmt::skip]
fn parse_code_block_body_including_code_block_end_alt<'a>(input: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    let (remainder, output) = terminated(
        take_until(CODE_BLOCK_END),
        /* end (discard) */ tag(CODE_BLOCK_END),
    ).parse(input)?;
    Ok((remainder, output))
}

/// At a minimum, a [CodeBlockLine] will be 2 lines of text.
/// 1. The first line will be the language of the code block, eg: "```rs\n" or "```\n".
/// 2. The second line will be the end of the code block, eg: "```\n" Then there may be
///    some number of lines of text in the middle. These lines are stored in the
///    [content](CodeBlockLine.content) field.
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
/// # Examples:
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""] `              |
/// | ""             | `[] `                |
/// | "foo\nbar\n"   | `["foo", "bar"] `    |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
fn split_by_new_line_alt<'a>(input: AsStrSlice<'a>) -> Vec<AsStrSlice<'a>> {
    if input.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();
    let full_content = input.extract_to_slice_end();
    let content_str = full_content.as_ref();

    // Split the string content and apply the same logic as the original
    let mut string_splits: Vec<&str> = content_str.split('\n').collect();
    if let Some(last_item) = string_splits.last() {
        if last_item.is_empty() {
            string_splits.pop();
        }
    }

    // Convert each string slice back to AsStrSlice
    let mut current_offset = 0;
    for split_str in string_splits {
        if !split_str.is_empty() {
            // Create an AsStrSlice for this segment
            let segment_slice = input.skip_take(current_offset, split_str.len());
            result.push(segment_slice);
        } else {
            // Handle empty segments (from "\n" cases)
            let segment_slice = input.skip_take(current_offset, 0);
            result.push(segment_slice);
        }
        // Move past this segment and the newline character
        current_offset += split_str.len() + 1; // +1 for the newline
    }

    result
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
        // Test with unicode characters in code content
        {
            as_str_slice_test_case!(
                input,
                "let emoji = \"😀🚀\";\nlet unicode = \"αβγδε\";\n```"
            );
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
    use crate::{as_str_slice_test_case,
                assert_eq2,
                list,
                CodeBlockLine,
                CodeBlockLineContent};

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
