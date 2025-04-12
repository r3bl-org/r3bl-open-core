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

use nom::{bytes::complete::tag, combinator::opt, sequence::preceded, IResult, Parser};
use r3bl_core::tiny_inline_string;

use crate::{md_parser::constants::{COLON, NEW_LINE, SPACE},
            take_text_until_new_line_or_end};

/// - Sample parse input: `@title: Something` or `@date: Else`.
/// - There may or may not be a newline at the end. If there is, it is consumed.
/// - Can't nest the `tag_name` within the `output`. So there can only be one `tag_name`
///   in the `output`.
pub fn parse_unique_kv_opt_eol<'a>(
    tag_name: &'a str,
    input: &'a str,
) -> IResult<&'a str, &'a str> {
    let (remainder, title_text) = preceded(
        /* start */ (tag(tag_name), tag(COLON), tag(SPACE)),
        /* output */ take_text_until_new_line_or_end(),
    )
    .parse(input)?;

    // Can't nest `tag_name` in `output`. Early return in this case.
    let tag_fragment = tiny_inline_string!("{tag_name}{COLON}{SPACE}");
    if title_text.contains(tag_fragment.as_str())
        | remainder.contains(tag_fragment.as_str())
    {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Can't have more than one tag_name in kv expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // If there is a newline, consume it since there may or may not be a newline at the
    // end.
    let (remainder, _) = opt(tag(NEW_LINE)).parse(remainder)?;

    // Special case: Early return when something like `@title: ` or `@title: \n` is found.
    if title_text.is_empty() {
        Ok((remainder, ""))
    }
    // Normal case.
    else {
        Ok((remainder, title_text))
    }
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use r3bl_core::{assert_eq2, fg_black, inline_string};

    use super::*;
    use crate::md_parser::constants::TITLE;

    #[test]
    fn test_not_quoted_no_eol() {
        let input = "@title: Something";
        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_not_quoted_with_eol() {
        let input = "@title: Something\n";
        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_no_quoted_no_eol_nested_title() {
        let input = "@title: Something @title: Something";
        let it = parse_unique_kv_opt_eol(TITLE, input);

        assert_eq2!(it.is_err(), true);
        if let Err(nom::Err::Error(ref e)) = it {
            assert_eq2!(e.input, "Can't have more than one tag_name in kv expr.");
            assert_eq2!(e.code, nom::error::ErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", it.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_no_eol_multiple_title_tags() {
        let input = "@title: Something\n@title: Else\n";
        let it = parse_unique_kv_opt_eol(TITLE, input);

        assert_eq2!(it.is_err(), true);
        if let Err(nom::Err::Error(ref e)) = it {
            assert_eq2!(e.input, "Can't have more than one tag_name in kv expr.");
            assert_eq2!(e.code, nom::error::ErrorKind::Fail);
        }

        println!(
            "err: '{}'",
            fg_black(&inline_string!("{:?}", it.err().unwrap())).bg_yellow(),
        );
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_1() {
        let input = "@title: \nfoo\nbar";
        println!("input: '{}'", fg_black(input).bg_cyan(),);

        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, "");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_2() {
        let input = "@title:  a\nfoo\nbar";
        println!("input: '{}'", fg_black(input).bg_cyan(),);

        let (input, output) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "input: '{i}', output: '{o}'",
            i = fg_black(input).bg_yellow(),
            o = fg_black(output).bg_green(),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, " a");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_3() {
        let input = "@title: \n\n# heading1\n## heading2";
        println!("❯ input: \n'{}'", fg_black(input).bg_cyan(),);

        let (remainder, title) = parse_unique_kv_opt_eol(TITLE, input).unwrap();
        println!(
            "❯ remainder: \n'{r}'\n❯ title: \n'{t}'",
            r = fg_black(remainder).bg_yellow(),
            t = fg_black(title).bg_green(),
        );
        assert_eq2!(remainder, "\n# heading1\n## heading2");
        assert_eq2!(title, "");
    }
}
