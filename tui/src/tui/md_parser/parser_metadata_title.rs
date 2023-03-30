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

/// - Parse input: `@title: Something`.
/// - There may or may not be a newline at the end.
#[rustfmt::skip]
pub fn parse_title_opt_eol(input: &str) -> IResult<&str, &str> {
    // BM: eg of _opt_eol
    let (remainder, title_text) = preceded(
        /* start */ tuple((tag(TITLE), tag(COLON), tag(SPACE))),
        /* output */ alt((
            is_not(NEW_LINE),
            recognize(many1(anychar)),
        )),
    )(input)?;

    // Can't nest titles.
    if title_text.contains(format!("{TITLE}{COLON}{SPACE}").as_str()) {
        return Err(nom::Err::Error(nom::error::Error::new(
            "Can't have more than one @title: expr.",
            nom::error::ErrorKind::Fail,
        )));
    }

    // Special case: Early return when just a newline after the title prefix. Eg: `@title: \n..`.
    if title_text.starts_with(NEW_LINE) {
        if let Some(stripped) = title_text.strip_prefix(NEW_LINE) {
            return Ok((stripped, ""));
        }
    }

    // Normal case: if there is a newline, consume it since there may or may not be a newline at
    // the end.
    let (input, _) = opt(tag(NEW_LINE))(remainder)?;
    Ok((input, title_text))
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use ansi_term::Color::*;
    use pretty_assertions::assert_eq;

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
        assert_eq!(input, "");
        assert_eq!(output, "Something");
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
        assert_eq!(input, "");
        assert_eq!(output, "Something");
    }

    #[test]
    fn test_no_quoted_no_eol_nested_title() {
        let input = "@title: Something @title: Something";
        let it = parse_title_opt_eol(input);
        assert_eq!(it.is_err(), true);
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
        assert_eq!(input, "foo\nbar");
        assert_eq!(output, "");
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
        assert_eq!(input, "foo\nbar");
        assert_eq!(output, " a");
    }

    #[test]
    fn test_no_quoted_with_eol_title_with_postfix_content_3() {
        let input = "@title: \n\n# heading1\n## heading2";
        println!("❯ input: \n'{}'", Black.on(Cyan).paint(input),);
        let (remainder, title) = parse_title_opt_eol(input).unwrap();
        println!(
            "❯ remainder: \n'{}'\n❯ title: \n'{}'",
            Black.on(Yellow).paint(remainder),
            Black.on(Green).paint(title),
        );
        assert_eq!(remainder, "\n# heading1\n## heading2");
        assert_eq!(title, "");
    }
}
