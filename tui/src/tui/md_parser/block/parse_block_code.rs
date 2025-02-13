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

use nom::{branch::alt,
          bytes::complete::{is_not, tag, take_until},
          combinator::{map, opt},
          sequence::{preceded, terminated, tuple},
          IResult};

use crate::{constants::{CODE_BLOCK_END, CODE_BLOCK_START_PARTIAL, NEW_LINE},
            CodeBlockLine,
            CodeBlockLineContent,
            List};

/// Sample inputs:
///
/// | Scenario                  | Sample input                                               |
/// |---------------------------|------------------------------------------------------------|
/// | One line                  | `"```bash\npip install foobar\n```\n"`                     |
/// | No line                   | `"```\n\n```\n"`                                           |
/// | Multi line                | `"```bash\npip install foobar\npip install foobar\n```\n"` |
/// | No language               | `"```\npip install foobar\n```\n"`                         |
/// | No language, no line      | `"```\n```\n"`                                             |
/// | No language, multi line   | `"```\npip install foobar\npip install foobar\n```\n"`     |
#[rustfmt::skip]
pub fn parse_block_code(input: &str) -> IResult<&str, List<CodeBlockLine<'_>>> {
    let (remainder, (lang, code)) = tuple((
        parse_code_block_lang_to_eol,
        parse_code_block_body_to_code_block_end,
    ))(input)?;

    // Normal case: if there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE))(remainder)?;

    let acc = split_by_new_line(code);

    Ok((remainder, convert_into_code_block_lines(lang, acc)))
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
fn parse_code_block_body_to_code_block_end(input: &str) -> IResult<&str, &str> {
    let (remainder, output) = terminated(
        take_until(CODE_BLOCK_END),
        /* end (discard) */ tag(CODE_BLOCK_END),
    )(input)?;
    Ok((remainder, output))
}

/// Split a string by newline. The idea is that a line is some text followed by a newline. An
/// empty line is just a newline character.
///
/// # Examples:
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""] `              |
/// | ""             | `[] `                |
/// | "foo\nbar\n"   | `["foo", "bar"] `    |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
pub fn split_by_new_line(input: &str) -> Vec<&str> {
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
) -> List<CodeBlockLine<'input>> {
    let mut acc = List::with_capacity(lines.len() + 2);

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::StartTag,
    };

    for line in lines {
        acc += CodeBlockLine {
            language: lang,
            content: CodeBlockLineContent::Text(line),
        };
    }

    acc += CodeBlockLine {
        language: lang,
        content: CodeBlockLineContent::EndTag,
    };

    acc
}

#[cfg(test)]
mod tests {
    use r3bl_core::assert_eq2;

    use super::*;
    use crate::list;

    #[test]
    fn test_parse_codeblock_trailing_extra() {
        let input = "```bash\npip install foobar\n````";
        let lang = "bash";
        let code_lines = vec!["pip install foobar"];
        let (remainder, code_block_lines) = parse_block_code(input).unwrap();
        assert_eq2!(remainder, "`");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(Some(lang), code_lines)
        );
    }

    #[test]
    fn test_convert_from_code_block_into_lines() {
        // no line. "```rust\n```\n".
        {
            let language = Some("rust");
            let lines = vec![];
            let expected = list![
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
            let expected = list![
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
            let expected = list![
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
            let expected = list![
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
        assert_eq2!(split_by_new_line("foobar\n"), vec!["foobar"]);
        assert_eq2!(split_by_new_line("\n"), vec![""]);
        assert_eq2!(split_by_new_line(""), Vec::<&str>::new());
        assert_eq2!(split_by_new_line("foo\nbar\n"), vec!["foo", "bar"]);
        assert_eq2!(split_by_new_line("\nfoo\nbar\n"), vec!["", "foo", "bar"]);
    }

    #[test]
    fn test_parse_codeblock() {
        // One line: "```bash\npip install foobar\n```\n"
        {
            let lang = "bash";
            let code_lines = vec!["pip install foobar"];
            let input = ["```bash", "pip install foobar", "```", ""].join("\n");
            println!("{:#?}", &input);
            let (remainder, code_block_lines) = parse_block_code(&input).unwrap();
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
            let input = ["```bash", "```", ""].join("\n");
            let (remainder, code_block_lines) = parse_block_code(&input).unwrap();
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
            let input = ["```bash", "", "```", ""].join("\n");
            let (remainder, code_block_lines) = parse_block_code(&input).unwrap();
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
            let input = [
                "```python",
                "import foobar",
                "",
                "foobar.pluralize('word') # returns 'words'",
                "foobar.pluralize('goose') # returns 'geese'",
                "foobar.singularize('phenomena') # returns 'phenomenon'",
                "```",
                "",
            ]
            .join("\n");
            let (remainder, code_block_lines) = parse_block_code(&input).unwrap();
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
        let input = ["```", "pip install foobar", "```", ""].join("\n");
        let (remainder, code_block_lines) = parse_block_code(&input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(lang, code_lines)
        );
    }
}
