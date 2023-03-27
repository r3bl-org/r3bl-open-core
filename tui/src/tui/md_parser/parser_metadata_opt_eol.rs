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
use nom::{branch::*,
          bytes::complete::*,
          character::complete::*,
          combinator::*,
          multi::*,
          sequence::*,
          IResult};

use crate::*;

// 00: rename file to `parser_metadata_title_opt_eol.rs`
/// - Parse input: `@title: Something`.
/// - There may or may not be a newline at the end.
#[rustfmt::skip]
pub fn parse_title_opt_eol(input: &str) -> IResult<&str, &str> {
    let (mut input, mut output) = preceded(
        /* start */ tuple((tag(TITLE), tag(COLON), tag(SPACE))),
        /* output */ alt((
            is_not(NEW_LINE),
            recognize(many1(anychar)),
        )),
    )(input)?;

    // Special case: just a newline after the title prefix. Eg: `@title: \n..`.
    if output.starts_with(NEW_LINE) {
        input = &output[1..];
        output = "";
    }

    // Can't nest titles.
    if output.contains(format!("{TITLE}{COLON}{SPACE}").as_str()) {
        return Err(nom::Err::Error(nom::error::Error::new(
            output,
            nom::error::ErrorKind::Many1Count,
        )));
    }

    // If there is a newline, consume it since there may or may not be a newline at the end.
    let (input, _) = opt(tag(NEW_LINE))(input)?;

    Ok((input, output))
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use ansi_term::Color::*;
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_not_quoted_no_eol() {
        let input = "@title: Something";
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}', output: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_not_quoted_with_eol() {
        let input = "@title: Something\n";
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}', output: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_no_quoted_no_eol_nested_title() {
        let input = "@title: Something @title: Something";
        let it = parse_title_opt_eol(input);
        assert_eq2!(it.is_err(), true);
        println!(
            "err: '{}'",
            Black.on(Yellow).paint(format!("{:?}", it.err().unwrap()))
        );
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_1() {
        let input = "@title: \nfoo\nbar";
        println!("input: '{}'", Black.on(Cyan).paint(input),);
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}'\noutput: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, "");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_2() {
        let input = "@title:  a\nfoo\nbar";
        println!("input: '{}'", Black.on(Cyan).paint(input),);
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}'\noutput: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "foo\nbar");
        assert_eq2!(output, " a");
    }
}
