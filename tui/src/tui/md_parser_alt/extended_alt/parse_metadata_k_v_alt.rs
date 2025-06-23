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
use nom::{bytes::complete::tag, combinator::opt, sequence::preceded, IResult, Parser};

use crate::{inline_string,
            md_parser::constants::{COLON, NEW_LINE, SPACE},
            parser_take_text_until_eol_or_eoi_alt::parser_take_line_text_alt,
            AsStrSlice};

/// Parse metadata from a line like `@title: My Article Title`, `@date: 2023-12-25` or
/// `@date: December 25, 2023`
///
/// ## Input format
/// Expects a line starting with `tag_name` + colon + space, followed by the text.
/// Leading/trailing whitespace around the text value is trimmed. The line may end with a
/// newline or be at end-of-input. There can only be one `tag_name` in the text value. If
/// you nest the `tag_name` within the text value it will return an error.
///
/// ## Line advancement
/// This is a **single-line parser that auto-advances**. It consumes
/// the optional trailing newline if present, making it consistent with heading parsers.
/// The parser now properly advances the line position when a newline is encountered.
///
/// ## Returns
/// - Either `Ok((remaining_input, Some(text)))` on success or `Ok((remaining_input,
///   None))` if no text value is provided.
/// - Or `Err` if the line doesn't start with `tag_name` + colon + space. Or if the
///   `tag_name` appears more than once in the line.
///
/// ## Example
/// - `"@date: 2023-12-25\n"` → `Some("2023-12-25")`
/// - `"@title: My Great Article\n"` → `Some("My Great Article")`
/// - `"@title: Something"` -> `Some("Something")`
/// - `"@date: Else"` -> `Some("Else")`
pub fn parse_unique_kv_opt_eol_alt<'a>(
    tag_name: &'a str,
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    let input_clone = input.clone();

    // Parse the full pattern including optional newline in one go, like heading parser
    let (remainder, (title_text, _)) = (
        preceded(
            /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
            /* output */ parser_take_line_text_alt(),
        ),
        opt(tag(NEW_LINE)),
    )
        .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    // Check if the tag pattern appears in the parsed content or remainder.
    let sub_str = inline_string!("{tag_name}{COLON}{SPACE}");
    if title_text.contains_in_current_line(sub_str.as_str())
        | remainder.contains_in_current_line(sub_str.as_str())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            input_clone, // "Can't have more than one tag_name in kv expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // Special case: Early return when something like `@title: ` or `@title: \n` is found.
    if title_text.is_empty() {
        Ok((remainder, None))
    }
    // Normal case.
    else {
        Ok((remainder, Some(title_text)))
    }
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use super::*;
    use crate::{as_str_slice_test_case,
                assert_eq2,
                fg_black,
                inline_string,
                md_parser::constants::TITLE,
                InlineStringCow,
                NomErr,
                NomErrorKind};

    #[test]
    fn test_not_quoted_no_eol() {
        as_str_slice_test_case!(input, "@title: Something");
        let (rem, output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = &output.unwrap().extract_to_slice_end();

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(output_str.as_ref(), "Something");
        assert_eq2!(rem_str.as_ref(), "");
    }

    #[test]
    fn test_not_quoted_with_eol() {
        as_str_slice_test_case!(input, "@title: Something\n");
        let (rem, output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = &output.unwrap().extract_to_slice_end();

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(output_str.as_ref(), "Something");
        assert_eq2!(rem_str.as_ref(), "");
    }

    #[test]
    fn test_no_quoted_no_eol_nested_title() {
        as_str_slice_test_case!(input, "@title: Something @title: Something");
        let input_clone = input.clone();

        let res = parse_unique_kv_opt_eol_alt(TITLE, input);

        assert_eq2!(res.is_err(), true);
        if let Err(NomErr::Error(ref e)) = res {
            assert_eq2!(e.input, input_clone);
            assert_eq2!(e.code, NomErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", res.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_no_eol_multiple_title_tags() {
        as_str_slice_test_case!(input, "@title: Something\n@title: Else\n");
        let input_clone = input.clone();

        let res = parse_unique_kv_opt_eol_alt(TITLE, input);

        assert_eq2!(res.is_err(), true);
        if let Err(NomErr::Error(ref e)) = res {
            assert_eq2!(e.input, input_clone);
            assert_eq2!(e.code, NomErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", res.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_1() {
        as_str_slice_test_case!(input, "@title: \nfoo\nbar");
        println!(
            "input: '{}'",
            fg_black(input.extract_to_slice_end()).bg_cyan()
        );

        let (rem, maybe_output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = match maybe_output {
            Some(output) => &output.extract_to_slice_end(),
            None => &InlineStringCow::new_empty_borrowed(),
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(output_str.as_ref(), "");
        assert_eq2!(rem_str.as_ref(), "foo\nbar");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_2() {
        as_str_slice_test_case!(input, "@title:  a\nfoo\nbar");
        println!(
            "input: '{}'",
            fg_black(input.extract_to_slice_end()).bg_cyan()
        );

        let (rem, maybe_output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = match maybe_output {
            Some(output) => &output.extract_to_slice_end(),
            None => &InlineStringCow::new_empty_borrowed(),
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(rem_str.as_ref(), "foo\nbar");
        assert_eq2!(output_str.as_ref(), " a");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_3() {
        as_str_slice_test_case!(input, "@title: \n\n# heading1\n## heading2");
        println!(
            "input: '{}'",
            fg_black(input.extract_to_slice_end()).bg_cyan()
        );

        let (rem, output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = match output {
            Some(o) => &o.extract_to_slice_end(),
            None => &InlineStringCow::new_empty_borrowed(),
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );
        assert_eq2!(output_str.as_ref(), "");
        assert_eq2!(rem_str.as_ref(), "\n# heading1\n## heading2");
    }
}
