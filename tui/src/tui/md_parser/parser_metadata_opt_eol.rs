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
use r3bl_rs_utils_core::*;

use crate::*;

pub fn try_parse_and_format_title_into_styled_us_span_line<'a>(
    input: &'a str,
    maybe_current_box_computed_style: &'a Option<Style>,
) -> Option<StyleUSSpanLine> {
    let (_, title_text) = parse_title_opt_eol(input).ok()?;
    let it = StyleUSSpanLine::from_title(title_text, maybe_current_box_computed_style);
    Some(it)
}

// AA: exp
/// - Parse input: `@title: "Something"` or `@title: Something`.
/// - There may or may not be a newline at the end.
#[rustfmt::skip]
pub fn parse_title_opt_eol(input: &str) -> IResult<&str, &str> {
    // There may or may not be a newline at the end.
    return alt((
        parse_title_quoted,
        parse_title_not_quoted
    ))(input);

    /// Eg: `@title: Something`.
    fn parse_title_not_quoted(input: &str) -> IResult<&str, &str> {
        preceded(
            /* start */ tuple((tag(TITLE), tag(COLON), space1)),
            /* output */ alt((
                is_not(NEW_LINE),
                recognize(many1(anychar)),
            )),
        )(input)
    }

    /// Eg: `@title: "Something"`.
    fn parse_title_quoted(input: &str) -> IResult<&str, &str> {
        preceded(
            /* start */ tuple((tag(TITLE), tag(COLON), space1)),
            /* output */ parse_quoted,
        )(input)
    }

    fn parse_quoted(input: &str) -> IResult<&str, &str> {
        // Make sure there are no new lines in the output.
        verify(
            delimited(
                /* start */ tag(QUOTE),
                /* output */ is_not(QUOTE),
                /* end */ tag(QUOTE),
            ),
            |it: &&str| !it.contains(NEW_LINE)
        )(input)
    }
}

#[cfg(test)]
mod test_parse_title_no_eol {
    use ansi_term::Color::*;
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_not_quoted_no_eol() {
        // AA: exp
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
        // AA: exp
        let input = "@title: Something\n";
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}', output: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "\n");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_quoted_no_eol() {
        // AA: exp
        let input = "@title: \"Something\"";
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
    fn test_quoted_with_eol_1() {
        // AA: exp
        let input = "@title: \"Something\"\n";
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}', output: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "\n");
        assert_eq2!(output, "Something");
    }

    #[test]
    fn test_quoted_with_eol_2() {
        // AA: exp
        let input = "@title: \"Something\n\"";
        let (input, output) = parse_title_opt_eol(input).unwrap();
        println!(
            "input: '{}', output: '{}'",
            Black.on(Yellow).paint(input),
            Black.on(Green).paint(output),
        );
        assert_eq2!(input, "\n\"");
        assert_eq2!(output, "\"Something");
    }
}
