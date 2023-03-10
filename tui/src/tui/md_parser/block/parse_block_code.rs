/*
 *   Copyright (c) 2023 R3BL LLC
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

use constants::*;
use nom::{branch::*, bytes::complete::*, combinator::*, sequence::*, IResult};

use crate::*;

/// Sample inputs:
/// One line:                "```bash\npip install foobar\n```\n"
/// No line:                 "```\n\n```\n"
/// Multi line:              "```bash\npip install foobar\npip install foobar\n```\n"
/// No language:             "```\npip install foobar\n```\n"
/// No language, no line:    "```\n```\n"
/// No language, multi line: "```\npip install foobar\npip install foobar\n```\n"
pub fn parse_block_code(input: &str) -> IResult<&str, Vec<CodeBlockLine>> { parse(input) }

#[rustfmt::skip]
fn parse(input: &str) -> IResult<&str, Vec<CodeBlockLine>> {
    let (input, (lang, code)) = tuple(
        (parse_code_block_lang_to_eol, parse_code_block_body_to_code_block_end_to_eol)
    )(input)?;
    let acc = split_by_newline(code);
    return Ok((input, convert_into_code_block_lines(lang, acc)));
}

#[rustfmt::skip]
fn parse_code_block_lang_to_eol(input: &str) -> IResult<&str, Option<&str>> {
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
            tuple((tag(CODE_BLOCK_START_PARTIAL), tag(NEW_LINE))),
            |_| None,
        ),
    ))(input)
}

#[rustfmt::skip]
fn parse_code_block_body_to_code_block_end_to_eol(input: &str) -> IResult<&str, &str> {
    terminated(
        take_until(CODE_BLOCK_END),
        /* end (discard) */ tag(CODE_BLOCK_END),
    )(input)
}

/// Split a string by newline. The idea is that a line is some text followed by a newline. An
/// empty line is just a newline character.
///
/// ## Examples:
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""] `              |
/// | ""             | `[] `                |
/// | "foo\nbar\n"   | `["foo", "bar"] `    |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
pub fn split_by_newline(input: &str) -> Vec<&str> {
    let mut acc: Vec<&str> = input.split('\n').collect();
    if let Some(last_item) = acc.last() {
        if last_item.is_empty() {
            acc.pop();
        }
    }
    acc
}

/// At a minimum, a [CodeBlockLine] will be 2 lines of text.
/// 1. The first line will be the language of the code block, eg: "```rust\n" or "```\n".
/// 2. The second line will be the end of the code block, eg: "```\n" Then there may be some
///    number of lines of text in the middle. These lines are stored in the
///    [content](CodeBlockLine.content) field.
pub fn convert_into_code_block_lines<'input>(
    lang: Option<&'input str>,
    lines: Vec<&'input str>,
) -> Vec<CodeBlockLine<'input>> {
    let mut acc = Vec::with_capacity(lines.len() + 2);

    acc.push(CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::StartTag,
    });
    for line in lines {
        acc.push(CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::Text(line),
        });
    }
    acc.push(CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::EndTag,
    });

    acc
}

#[cfg(test)]
mod tests {
    use r3bl_rs_utils_core::*;

    use super::*;
    use crate::test_data::raw_strings;

    #[test]
    fn test_convert_from_code_block_into_lines() {
        // no line. "```rust\n```\n".
        {
            let language = Some("rust");
            let lines = vec![];
            let expected = vec![
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, lines);
            assert_eq2!(output, expected);
        }

        // 1 empty line. "```rust\n\n```\n".
        {
            let language = Some("rust");
            let lines = vec![""];
            let expected = vec![
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::Text(""),
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, lines);
            assert_eq2!(output, expected);
        }

        // 1 line. "```rust\nlet x = 1;\n```\n".
        {
            let language = Some("rust");
            let lines = vec!["let x = 1;"];
            let expected = vec![
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::Text("let x = 1;"),
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, lines);
            assert_eq2!(output, expected);
        }

        // 2 lines. "```rust\nlet x = 1;\nlet y = 2;\n```\n".
        {
            let language = Some("rust");
            let lines = vec!["let x = 1;", "let y = 2;"];
            let expected = vec![
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::Text("let x = 1;"),
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::Text("let y = 2;"),
                },
                CodeBlockLine {
                    language: Some("rust"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, lines);
            assert_eq2!(output, expected);
        }
    }

    #[test]
    fn test_parse_codeblock_split_by_eol() {
        assert_eq2!(split_by_newline("foobar\n"), vec!["foobar"]);
        assert_eq2!(split_by_newline("\n"), vec![""]);
        assert_eq2!(split_by_newline(""), Vec::<&str>::new());
        assert_eq2!(split_by_newline("foo\nbar\n"), vec!["foo", "bar"]);
        assert_eq2!(split_by_newline("\nfoo\nbar\n"), vec!["", "foo", "bar"]);
    }

    #[test]
    fn test_parse_codeblock() {
        // One line: "```bash\npip install foobar\n```\n"
        {
            let lang = "bash";
            let code_lines = vec!["pip install foobar"];
            println!("{:#?}", (raw_strings::CODE_BLOCK_3_INPUT));
            let (remainder, code_block_lines) =
                parse_block_code(raw_strings::CODE_BLOCK_3_INPUT).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), code_lines)
            );
        }

        // No line: "```bash\n```\n"
        {
            let lang = "bash";
            let code_lines = vec![];
            let (remainder, code_block_lines) =
                parse_block_code(raw_strings::CODE_BLOCK_0_INPUT).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), code_lines)
            );
        }

        // 1 empty line: "```bash\n\n```\n"
        {
            let lang = "bash";
            let code_lines = vec![""];
            let (remainder, code_block_lines) =
                parse_block_code(raw_strings::CODE_BLOCK_1_EMPTY_INPUT).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), code_lines)
            );
        }

        // Multiple lines.
        // "import foobar\n\nfoobar.pluralize('word') # returns 'words'\nfoobar.pluralize('goose') # returns 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```"
        {
            let lang = "python";
            let code_lines = vec![
                "import foobar",
                "",
                "foobar.pluralize('word') # returns 'words'",
                "foobar.pluralize('goose') # returns 'geese'",
                "foobar.singularize('phenomena') # returns 'phenomenon'",
            ];
            let (remainder, code_block_lines) =
                parse_block_code(raw_strings::CODE_BLOCK_2_INPUT).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), code_lines)
            );
        }
    }

    #[test]
    fn test_parse_codeblock_no_language() {
        let lang = None;
        let code_lines = vec!["pip install foobar"];
        let (remainder, code_block_lines) =
            parse_block_code(raw_strings::CODE_BLOCK_1_INPUT).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(lang, code_lines)
        );
    }
}
