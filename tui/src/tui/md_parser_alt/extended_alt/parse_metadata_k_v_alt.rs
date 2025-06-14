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
            parser_take_text_until_eol_or_eoi_alt,
            AsStrSlice};

/// - Sample parse input: `@title: Something` or `@date: Else`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
/// - Can't nest the `tag_name` within the `output`. So there can only be one `tag_name`
///   in the `output`.
pub fn parse_unique_kv_opt_eol_alt<'a>(
    tag_name: &'a str,
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    let input_clone = input.clone();

    let (remainder, title_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ parser_take_text_until_eol_or_eoi_alt(),
    )
    .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    // Check if the tag pattern appears in the parsed content or remainder.
    let sub_str = inline_string!("{tag_name}{COLON}{SPACE}");
    if title_text.contains(sub_str.as_str()) | remainder.contains(sub_str.as_str()) {
        return Err(nom::Err::Error(nom::error::Error::new(
            input_clone, // "Can't have more than one tag_name in kv expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // If there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

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
                GCString,
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

        assert_eq2!(output_str, "Something");
        assert_eq2!(rem_str, "");
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

        assert_eq2!(output_str, "Something");
        assert_eq2!(rem_str, "");
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

        let (rem, output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = match output {
            Some(o) => &o.extract_to_slice_end(),
            None => "",
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(output_str, "");
        assert_eq2!(rem_str, "foo\nbar");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_2() {
        as_str_slice_test_case!(input, "@title:  a\nfoo\nbar");
        println!(
            "input: '{}'",
            fg_black(input.extract_to_slice_end()).bg_cyan()
        );

        let (rem, output) = parse_unique_kv_opt_eol_alt(TITLE, input).unwrap();

        let rem_str = &rem.extract_to_slice_end();
        let output_str = match output {
            Some(o) => &o.extract_to_slice_end(),
            None => "",
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );

        assert_eq2!(rem_str, "foo\nbar");
        assert_eq2!(output_str, " a");
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
            None => "",
        };

        println!(
            "output: '{o}', rem: '{r}'",
            o = fg_black(output_str).bg_green(),
            r = fg_black(rem_str).bg_yellow(),
        );
        assert_eq2!(output_str, "");
        assert_eq2!(rem_str, "\n# heading1\n## heading2");
    }
}
