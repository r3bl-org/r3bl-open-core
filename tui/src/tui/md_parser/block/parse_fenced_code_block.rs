// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use nom::{IResult, Parser,
          branch::alt,
          bytes::complete::{is_not, tag, take_until, take_while},
          combinator::{eof, map},
          multi::many0,
          sequence::{preceded, terminated}};

use crate::{CodeBlockLine, CodeBlockLineContent, CodeBlockLines, List,
            md_parser::constants::{CODE_BLOCK_END, CODE_BLOCK_START_PARTIAL, NEW_LINE,
                                   NEWLINE_OR_NULL, NULL_CHAR},
            parse_null_padded_line::{is, trim_optional_leading_newline_and_nulls}};

/// Parse fenced code blocks with language tags and content.
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0` characters,
/// as provided by `ZeroCopyGapBuffer::as_str()`. The parser uses `NEWLINE_OR_NULL` constant to
/// handle both `\n` and `\0` as line terminators.
///
/// # Sample inputs:
///
/// | Scenario                  | Sample input                                               |
/// |---------------------------|------------------------------------------------------------|
/// | One line                  | `"```bash\npip install foobar\n```\n"`                     |
/// | No line                   | `"```\n\n```\n"`                                           |
/// | Multi line                | `"```bash\npip install foobar\npip install foobar\n```\n"` |
/// | No language               | `"```\npip install foobar\n```\n"`                         |
/// | No language, no line      | `"```\n```\n"`                                             |
/// | No language, multi line   | `"```\npip install foobar\npip install foobar\n```\n"`     |
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain a valid fenced code block.
#[rustfmt::skip]
pub fn parse_fenced_code_block(input: &str) -> IResult<&str, List<CodeBlockLine<'_>>> {
    let (remainder, (lang, code)) = (
        parse_code_block_lang_including_eol,
        parse_code_block_body_including_code_block_end,
    )
        .parse(input)?;

    // Normal case: if there is a newline, consume it along with any null padding
    // to handle the ZeroCopyGapBuffer null padding invariant.
    let remainder = trim_optional_leading_newline_and_nulls(remainder);

    let acc = split_by_new_line(code);

    Ok((remainder, convert_into_code_block_lines(lang, &acc.into())))
}

/// Take text until an optional EOL character is found, or end of input is reached.
/// Consumes the [`NEW_LINE`] if it exists.
#[rustfmt::skip]
fn parse_code_block_lang_including_eol(input: &str) -> IResult<&str, Option<&str>> {
    alt((
        // Either - Successfully parse both code block language & text.
        map(
            preceded(
                /* prefix - discarded */ tag(CODE_BLOCK_START_PARTIAL),
                /* output */
                terminated(
                    /* match */ is_not(NEWLINE_OR_NULL),
                    /* ends with (discarded) */
                    (tag(NEW_LINE), /* zero or more */ take_while(is(NULL_CHAR))),
                ),
            ),
            Some,
        ),
        // Or - Fail to parse language, use unknown language instead.
        map(
            (
                tag(CODE_BLOCK_START_PARTIAL),
                tag(NEW_LINE),
                /* zero or more */ take_while(is(NULL_CHAR))
            ),
            |_| None,
        ),
    )).parse(input)
}

/// Parse the body of a code block until the end of the code block is reached.
/// The end of the code block is indicated by the [`CODE_BLOCK_END`] constant.
/// Consumes the [`CODE_BLOCK_END`] if it exists.
#[rustfmt::skip]
fn parse_code_block_body_including_code_block_end(input: &str) -> IResult<&str, &str> {
    let (remainder, output) = terminated(
        take_until(CODE_BLOCK_END),
        /* end (discard) */ tag(CODE_BLOCK_END),
    ).parse(input)?;
    Ok((remainder, output))
}

/// Split a string by newline using nom parsers. The idea is that a line is some text
/// followed by a newline. An empty line is just a newline character.
///
/// # Examples:
/// | input          | output               |
/// | -------------- | -------------------- |
/// | "foobar\n"     | `["foobar"]`         |
/// | "\n"           | `[""] `              |
/// | ""             | `[] `                |
/// | "foo\nbar\n"   | `["foo", "bar"] `    |
/// | "\nfoo\nbar\n" | `["", "foo", "bar"]` |
#[must_use]
pub fn split_by_new_line(input: &str) -> Vec<&str> {
    // Define a parser that can handle three different line patterns.
    // This parser will be called repeatedly by many0() to consume the entire input.
    let parser = alt((
        // CASE 1: Regular line with content followed by newline
        // Example: "hello world\n" -> "hello world"
        // This handles the most common case where a line has text content.
        terminated(
            // First part: capture all characters that are NOT newline or null
            // is_not(NEWLINE_OR_NULL) matches any sequence of chars except '\n' and '\0'
            map(is_not(NEWLINE_OR_NULL), |s: &str| s),
            // Second part: consume (but don't capture) the line ending
            // This handles both the newline character and any null padding that follows.
            (
                tag(NEW_LINE),
                /* zero or more */ take_while(is(NULL_CHAR)),
            ),
        ),
        // CASE 2: Empty line (just a newline character, possibly with null padding)
        // Example: "\n" -> ""
        // This is needed because is_not() in CASE 1 would fail on empty content
        map(
            // Match a newline followed by any number of null chars.
            (
                tag(NEW_LINE),
                /* zero or more */ take_while(is(NULL_CHAR)),
            ),
            // Transform the matched pattern into an empty string.
            |_| "",
        ),
        // CASE 3: Last line without trailing newline (at end of file)
        // Example: "last line" (at EOF) -> "last line"
        // This handles the edge case where the file doesn't end with a newline.
        terminated(
            // Match any characters except newline/null
            is_not(NEWLINE_OR_NULL),
            // But only if we're at the end of the input.
            eof,
        ),
    ));

    // Apply the parser repeatedly using many0() to collect all lines
    // many0() will keep applying the parser until it fails, collecting results in a Vec
    let result: IResult<&str, Vec<&str>> = many0(parser).parse(input);

    match result {
        Ok((_, lines)) => lines,
        Err(_) => Vec::new(), // Return empty vec if parsing fails
    }
}

/// Convert language and lines into `CodeBlockLines` structure.
/// This function was previously in `md_parser_ng` but is now implemented locally.
fn convert_into_code_block_lines<'a>(
    language: Option<&'a str>,
    lines: &List<&'a str>,
) -> CodeBlockLines<'a> {
    let mut result = List::new();

    // Add start tag
    result.push(CodeBlockLine {
        language,
        content: CodeBlockLineContent::StartTag,
    });

    // Add text lines
    for line in lines.iter() {
        result.push(CodeBlockLine {
            language,
            content: CodeBlockLineContent::Text(line),
        });
    }

    // Add end tag
    result.push(CodeBlockLine {
        language,
        content: CodeBlockLineContent::EndTag,
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, list};

    #[test]
    fn test_parse_codeblock_split_by_eol() {
        assert_eq2!(split_by_new_line("foobar\n"), vec!["foobar"]);
        assert_eq2!(split_by_new_line("\n"), vec![""]);
        assert_eq2!(split_by_new_line(""), Vec::<&str>::new());
        assert_eq2!(split_by_new_line("foo\nbar\n"), vec!["foo", "bar"]);
        assert_eq2!(split_by_new_line("\nfoo\nbar\n"), vec!["", "foo", "bar"]);
    }

    #[test]
    fn test_parse_codeblock_trailing_extra() {
        let input = "```bash\npip install foobar\n````";
        let lang = "bash";
        let code_lines = vec!["pip install foobar"];
        let (remainder, code_block_lines) = parse_fenced_code_block(input).unwrap();
        assert_eq2!(remainder, "`");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(Some(lang), &code_lines.into())
        );
    }

    #[test]
    fn test_convert_from_code_block_into_lines() {
        // no line. "```rs\n```\n".
        {
            let language = Some("rs");
            let lines = vec![];
            let expected = list![
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, &lines.into());
            assert_eq2!(output, expected);
        }

        // 1 empty line. "```rs\n\n```\n".
        {
            let language = Some("rs");
            let lines = vec![""];
            let expected = list![
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::Text(""),
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, &lines.into());
            assert_eq2!(output, expected);
        }

        // 1 line. "```rs\nlet x = 1;\n```\n".
        {
            let language = Some("rs");
            let lines = vec!["let x = 1;"];
            let expected = list![
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::Text("let x = 1;"),
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, &lines.into());
            assert_eq2!(output, expected);
        }

        // 2 lines. "```rs\nlet x = 1;\nlet y = 2;\n```\n".
        {
            let language = Some("rs");
            let lines = vec!["let x = 1;", "let y = 2;"];
            let expected = list![
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::StartTag,
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::Text("let x = 1;"),
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::Text("let y = 2;"),
                },
                CodeBlockLine {
                    language: Some("rs"),
                    content: CodeBlockLineContent::EndTag,
                },
            ];
            let output = convert_into_code_block_lines(language, &lines.into());
            assert_eq2!(output, expected);
        }
    }

    #[test]
    fn test_parse_codeblock() {
        // One line: "```bash\npip install foobar\n```\n"
        {
            let lang = "bash";
            let code_lines = vec!["pip install foobar"];
            let input = ["```bash", "pip install foobar", "```", ""].join("\n");
            println!("{:#?}", &input);
            let (remainder, code_block_lines) = parse_fenced_code_block(&input).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }

        // No line: "```bash\n```\n"
        {
            let lang = "bash";
            let code_lines = vec![];
            let input = ["```bash", "```", ""].join("\n");
            let (remainder, code_block_lines) = parse_fenced_code_block(&input).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }

        // 1 empty line: "```bash\n\n```\n"
        {
            let lang = "bash";
            let code_lines = vec![""];
            let input = ["```bash", "", "```", ""].join("\n");
            let (remainder, code_block_lines) = parse_fenced_code_block(&input).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }

        // Multiple lines.
        // "import foobar\n\nfoobar.pluralize('word') # returns
        // 'words'\nfoobar.pluralize('goose') # returns
        // 'geese'\nfoobar.singularize('phenomena') # returns 'phenomenon'\n```"
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
            let (remainder, code_block_lines) = parse_fenced_code_block(&input).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }
    }

    #[test]
    fn test_parse_codeblock_no_language() {
        let lang = None;
        let code_lines = vec!["pip install foobar"];
        let input = ["```", "pip install foobar", "```", ""].join("\n");
        let (remainder, code_block_lines) = parse_fenced_code_block(&input).unwrap();
        assert_eq2!(remainder, "");
        assert_eq2!(
            code_block_lines,
            convert_into_code_block_lines(lang, &code_lines.into())
        );
    }

    #[test]
    fn test_parse_codeblock_with_null_padding() {
        // Code block followed by null padding.
        {
            let lang = "python";
            let code_lines = vec!["import foo", "bar()"];
            let input = "```python\nimport foo\nbar()\n```\n\0\0\0";
            let (remainder, code_block_lines) = parse_fenced_code_block(input).unwrap();
            assert_eq2!(remainder, "");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }

        // Code block with null padding and more content after.
        {
            let lang = "bash";
            let code_lines = vec!["pip install foobar"];
            let input = "```bash\npip install foobar\n```\0\0\0\nNext line";
            let (remainder, code_block_lines) = parse_fenced_code_block(input).unwrap();
            assert_eq2!(remainder, "\0\0\0\nNext line");
            assert_eq2!(
                code_block_lines,
                convert_into_code_block_lines(Some(lang), &code_lines.into())
            );
        }
    }
}
