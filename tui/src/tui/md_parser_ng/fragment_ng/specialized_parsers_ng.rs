/*
 *   Copyright (c) 2024-2025 R3BL LLC
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
          bytes::complete::tag,
          combinator::{map, recognize},
          multi::many0,
          IResult,
          Input,
          Parser};

use super::take_text_between_delims_enclosed_err_on_new_line_ng;
use crate::{fg_blue,
            fg_red,
            md_parser::constants::{BACK_TICK,
                                   CHECKED,
                                   LEFT_BRACKET,
                                   LEFT_IMAGE,
                                   LEFT_PARENTHESIS,
                                   RIGHT_BRACKET,
                                   RIGHT_IMAGE,
                                   RIGHT_PARENTHESIS,
                                   STAR,
                                   UNCHECKED,
                                   UNDERSCORE},
            AsStrSlice,
            HyperlinkData,
            NErr,
            NError,
            NErrorKind,
            DEBUG_MD_PARSER_STDOUT};

pub fn parse_fragment_starts_with_underscore_err_on_new_line_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(input, UNDERSCORE)
}

pub fn parse_fragment_starts_with_star_err_on_new_line_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(input, STAR)
}

/// For use with specialized parsers for: [`crate::constants::UNDERSCORE`],
/// [`crate::constants::STAR`], and [`crate::constants::BACK_TICK`]. See:
/// [`crate::parse_fragment_plain_text_no_new_line()`].
///
/// To see this in action, set the [`DEBUG_MD_PARSER_STDOUT`] to true, and run all the
/// tests in [`crate::parse_fragments_in_a_line`].
pub mod delim_matchers {
    use nom::multi::many1;

    use super::*;
    use crate::{constants::NEW_LINE, fg_green};

    /// Returns tuple:
    /// 0. number of occurrences in the input, until the first "\n" or end of input.
    /// 1. does the input start with the delimiter?
    /// 2. is the input the delimiter?
    /// 3. the delimiter.
    #[must_use]
    pub fn count_delim_occurrences_until_eol_or_eoi<'a>(
        input: AsStrSlice<'a>,
        delim: &'a str,
    ) -> (usize, bool, bool, &'a str) {
        // If the input has a "\n" then split it at the first "\n", only count the number
        // of delims at the first part of the split.
        let input_str = input.extract_to_line_end();
        let (first_part, _) =
            input_str.split_at(input_str.find(NEW_LINE).unwrap_or(input_str.len()));
        let num_of_delim_occurrences = first_part.matches(delim).count();
        (
            num_of_delim_occurrences,
            input_str.starts_with(delim),
            input_str == delim,
            delim,
        )
    }

    pub fn take_starts_with_delim_enclosed_until_eol_or_eoi<'a>(
        input: AsStrSlice<'a>,
        delim: &'a str,
    ) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
        // Check if there is a closing delim.
        let (num_of_delim_occurrences, starts_with_delim, input_is_delim, _) =
            count_delim_occurrences_until_eol_or_eoi(input.clone(), delim);

        DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "\n{} specialized parser {}: \ninput: {:?}, delim: {:?}",
            fg_green("■■"),
            delim,
            input,
            delim
        );
        println!(
            "count: {num_of_delim_occurrences}, starts_w: {starts_with_delim}, input=delim: {input_is_delim}"
        );
    });

        if
        // The input must start with the delim for this parser to run.
        !starts_with_delim
        ||
        // If the input just contains a single delim, error out.
        input_is_delim
        ||
        // If there is no closing delim, only a single opening delim, then error out. This
        // forces the [parse_fragment_plain_text_no_new_line1()] to take care of this
        // case.
        num_of_delim_occurrences == 1
        {
            DEBUG_MD_PARSER_STDOUT.then(|| {
                println!(
                    "{a} parser error out for input: {i:?}",
                    a = fg_red("⬢⬢"),
                    i = input
                );
            });
            return Err(nom::Err::Error(nom::error::Error {
                input,
                code: nom::error::ErrorKind::Fail,
            }));
        }

        // If there is a closing delim, then we can safely take the text between the
        // delim.
        if num_of_delim_occurrences > 1 {
            let it =
                take_text_between_delims_enclosed_err_on_new_line_ng(input, delim, delim);
            DEBUG_MD_PARSER_STDOUT.then(|| {
                println!("{a} it: {b:?}", a = fg_blue("▲▲"), b = it);
            });
            return it;
        }

        // Otherwise, we split the input at the first delim.
        let (rem, output) = recognize(many1(tag(delim))).parse(input)?;

        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "{a}, rem: {r:?}, output: {o:?}",
                a = fg_blue("▲▲"),
                r = rem,
                o = output
            );
        });

        Ok((rem, output))
    }
}

pub fn parse_fragment_starts_with_backtick_err_on_new_line_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    // Backup in case of error.
    let input_clone = input.clone();

    // Count the number of consecutive backticks. If there are more than 2 backticks,
    // return an error, since this could be a code block.
    let it = recognize(many0(tag(BACK_TICK))).parse(input);
    if it.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with backtick: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                input_clone,
                BACK_TICK
            );
        });
    }
    let (_, output) = it?;
    if output.input_len() > 2 {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "{} more than 2 backticks in input:{:?}",
                fg_red("⬢⬢"),
                input_clone
            );
        });
        return Err(NErr::Error(NError {
            input: output,
            code: NErrorKind::Tag,
        }));
    }

    // Otherwise, return the text between the backticks.
    delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
        input_clone,
        BACK_TICK,
    )
}

pub fn parse_fragment_starts_with_left_image_err_on_new_line_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, HyperlinkData<'a>> {
    let input_clone_dbg = input.clone();

    // Parse the text between the image tags.
    let result_first = take_text_between_delims_enclosed_err_on_new_line_ng(
        input,
        LEFT_IMAGE,
        RIGHT_IMAGE,
    );

    if result_first.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                    "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                    fg_red("⬢⬢"),
                    input_clone_dbg,
                    LEFT_IMAGE
                );
        });
    }

    let (rem, part_between_image_tags) = result_first?;

    let rem_clone_dbg = rem.clone();

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_enclosed_err_on_new_line_ng(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );

    if result_second.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                rem_clone_dbg,
                LEFT_PARENTHESIS
            );
        });
    }

    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((
            part_between_image_tags.extract_to_line_end(),
            part_between_parenthesis.extract_to_line_end(),
        )),
    ));

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} specialized parser for image: {:?}",
            if it.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            it
        );
    });

    it
}

pub fn parse_fragment_starts_with_left_link_err_on_new_line_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, HyperlinkData<'a>> {
    let input_clone_dbg = input.clone();

    // Parse the text between the brackets.
    let result_first = take_text_between_delims_enclosed_err_on_new_line_ng(
        input,
        LEFT_BRACKET,
        RIGHT_BRACKET,
    );

    if result_first.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                input_clone_dbg,
                LEFT_BRACKET
            );
        });
    }

    let (rem, part_between_brackets) = result_first?;

    let rem_clone_dbg = rem.clone();

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_enclosed_err_on_new_line_ng(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );

    if result_second.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                rem_clone_dbg,
                LEFT_PARENTHESIS
            );
        });
    }

    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((
            part_between_brackets.extract_to_line_end(),
            part_between_parenthesis.extract_to_line_end(),
        )),
    ));

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} specialized parser for link: {:?}",
            if it.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            it
        );
    });

    it
}

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and
/// images.
///
/// So some extra hint is need from the code calling this parser to let it know whether to
/// parse a checkbox into plain text, or into a boolean.
pub fn parse_fragment_starts_with_checkbox_into_str_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    let it = alt((recognize(tag(CHECKED)), recognize(tag(UNCHECKED)))).parse(input);

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} specialized parser for checkbox: {:?}",
            if it.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            it
        );
    });

    it
}

/// Checkboxes are tricky since they begin with "[" which is also used for hyperlinks and
/// images.
///
/// So some extra hint is need from the code calling this parser to let it know whether to
/// parse a checkbox into plain text, or into a boolean.
pub fn parse_fragment_starts_with_checkbox_checkbox_into_bool_ng<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, bool> {
    let it =
        alt((map(tag(CHECKED), |_| true), map(tag(UNCHECKED), |_| false))).parse(input);

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} specialized parser for checkbox: {:?}",
            if it.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            it
        );
    });

    it
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, GCString, NErr, NErrorKind};

    #[test]
    fn test_underscore_fragment_parsing_fails_on_newline() {
        // "_here is italic_" -> ok
        {
            let lines = &[GCString::new("_here is italic_")];
            let input = AsStrSlice::from(lines);
            let exp_out = "here is italic";
            let exp_rem = "";
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), exp_rem);
                    assert_eq2!(output.extract_to_line_end(), exp_out);
                }
                _ => panic!("Expected success result"),
            }
        }

        // "__" -> ok
        {
            let lines = &[GCString::new("__")];
            let input = AsStrSlice::from(lines);
            let exp_out = "";
            let exp_rem = "";
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), exp_rem);
                    assert_eq2!(output.extract_to_line_end(), exp_out);
                }
                _ => panic!("Expected success result"),
            }
        }

        // "_here is italic" -> error
        {
            let err_input = "_here is italic";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is italic_" -> error
        {
            let err_input = "here is italic_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is italic" -> error
        {
            let err_input = "here is italic";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "_" -> error
        {
            let err_input = "_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "" -> error
        {
            let err_input = "";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_asterisk_fragment_parsing_fails_on_newline() {
        // "*here is bold*" -> ok
        {
            let lines = &[GCString::new("*here is bold*")];
            let input = AsStrSlice::from(lines);
            let exp_out = "here is bold";
            let exp_rem = "";
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), exp_rem);
                    assert_eq2!(output.extract_to_line_end(), exp_out);
                }
                _ => panic!("Expected success result"),
            }
        }

        // "*here is bold" -> error
        {
            let err_input = "*here is bold";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is bold*" -> error
        {
            let err_input = "here is bold*";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is bold" -> error
        {
            let err_input = "here is bold";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "*" -> error
        {
            let err_input = "*";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "" -> error
        {
            let err_input = "";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_ng(input);
            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_backtick_fragment_parsing_fails_on_newline() {
        {
            let lines = &[GCString::new("")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("`here is code")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "`here is code");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("here is code`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "here is code`");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("``")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "");
                }
                _ => panic!("Expected success result"),
            }
        }

        {
            let lines = vec![GCString::new("`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "`");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "");
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("`abcd`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "abcd");
                }
                _ => panic!("Expected success result"),
            }
        }

        {
            let lines = vec![GCString::new("```")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_ng(input);

            match res {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), "```");
                    assert_eq2!(error.code, NErrorKind::Tag);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_link_fragment_parsing_fails_on_newline() {
        {
            let lines = &[GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(lines);

            let (rem, output) =
                parse_fragment_starts_with_left_image_err_on_new_line_ng(input).unwrap();
            assert_eq2!(rem.extract_to_line_end(), "");
            assert_eq2!(output, HyperlinkData::new("alt text", "image.jpg"));
        }

        {
            let lines = &[GCString::new("[title](https://www.example.com)")];
            let res = parse_fragment_starts_with_left_link_err_on_new_line_ng(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(rem.extract_to_line_end(), "");
            assert_eq2!(
                output,
                HyperlinkData::new("title", "https://www.example.com")
            );
        }
    }

    #[test]
    fn test_checkbox_fragment_parsing_returns_string() {
        // Test [x] checkbox.
        {
            let lines = &[GCString::new("[x] here is a checkbox")];
            let res =
                parse_fragment_starts_with_checkbox_into_str_ng(AsStrSlice::from(lines));

            let (rem, output) = res.unwrap();
            assert_eq2!(output.extract_to_line_end(), "[x]");
            assert_eq2!(rem.extract_to_line_end(), " here is a checkbox");
        }

        // Test [ ] checkbox.
        {
            let lines = &[GCString::new("[ ] here is a checkbox")];
            let res =
                parse_fragment_starts_with_checkbox_into_str_ng(AsStrSlice::from(lines));

            assert!(res.is_ok());
            let (rem, output) = res.unwrap();
            assert_eq2!(output.extract_to_line_end(), "[ ]");
            assert_eq2!(rem.extract_to_line_end(), " here is a checkbox");
        }
    }

    #[test]
    fn test_checkbox_fragment_parsing_returns_boolean() {
        // Test [x] checkbox.
        {
            let lines = &[GCString::new("[x] here is a checkbox")];
            let res = parse_fragment_starts_with_checkbox_checkbox_into_bool_ng(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(output, true);
            assert_eq2!(rem.extract_to_line_end(), " here is a checkbox");
        }

        // Test [ ] checkbox.
        {
            let lines = &[GCString::new("[ ] here is a checkbox")];
            let res = parse_fragment_starts_with_checkbox_checkbox_into_bool_ng(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(output, false);
            assert_eq2!(rem.extract_to_line_end(), " here is a checkbox");
        }
    }
}

#[cfg(test)]
mod tests_delim_matchers {
    use super::*;
    use crate::{assert_eq2, GCString, NErr, NErrorKind};

    #[test]
    fn test_delimiter_occurrence_counting_until_line_end() {
        // Test basic underscore counting
        {
            let lines = &[GCString::new("_hello_world_")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 3);
            assert_eq2!(starts_with, true);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "_");
        }

        // Test with newline - should only count before newline
        {
            let input_str = "_hello_\nworld_more_";
            let lines = &[GCString::new(input_str)];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 2); // Only counts "_hello_", not after newline
            assert_eq2!(starts_with, true);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "_");
        }

        // Test input that is just the delimiter
        {
            let lines = &[GCString::new("_")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 1);
            assert_eq2!(starts_with, true);
            assert_eq2!(is_delim, true);
            assert_eq2!(delim, "_");
        }

        // Test input that doesn't start with delimiter
        {
            let lines = &[GCString::new("hello_world")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 1);
            assert_eq2!(starts_with, false);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "_");
        }

        // Test empty input
        {
            let lines = &[GCString::new("")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 0);
            assert_eq2!(starts_with, false);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "_");
        }

        // Test with star delimiter
        {
            let lines = &[GCString::new("*bold*text*")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "*");
            assert_eq2!(count, 3);
            assert_eq2!(starts_with, true);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "*");
        }

        // Test with backtick delimiter
        {
            let lines = &[GCString::new("`code`")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "`");
            assert_eq2!(count, 2);
            assert_eq2!(starts_with, true);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "`");
        }

        // Test no occurrences
        {
            let lines = &[GCString::new("hello world")];
            let input = AsStrSlice::from(lines);
            let (count, starts_with, is_delim, delim) =
                delim_matchers::count_delim_occurrences_until_eol_or_eoi(input, "_");
            assert_eq2!(count, 0);
            assert_eq2!(starts_with, false);
            assert_eq2!(is_delim, false);
            assert_eq2!(delim, "_");
        }
    }

    #[test]
    fn test_delimiter_extraction_without_newline_termination() {
        // Test successful case with underscore - paired delimiters
        {
            let lines = &[GCString::new("_hello_")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "hello");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test successful case with star - paired delimiters
        {
            let lines = &[GCString::new("*bold*")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "*",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "bold");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test successful case with backtick - paired delimiters
        {
            let lines = &[GCString::new("`code`")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "`",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "code");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test successful case with empty content between delimiters
        {
            let lines = &[GCString::new("__")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test error case - doesn't start with delimiter
        {
            let err_input = "hello_world_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // Test error case - input is just the delimiter
        {
            let err_input = "_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // Test error case - only one delimiter (no closing delimiter)
        {
            let err_input = "_hello";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // Test error case - empty input
        {
            let err_input = "";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Err(NErr::Error(error)) => {
                    assert_eq2!(error.input.extract_to_line_end(), err_input);
                    assert_eq2!(error.code, NErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // Test successful case with multiple delimiters (more than 2)
        {
            let lines = &[GCString::new("_hello_world_more_")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "world_more_");
                    assert_eq2!(output.extract_to_line_end(), "hello");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test with remaining text after paired delimiters
        {
            let lines = &[GCString::new("_italic_ and more text")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), " and more text");
                    assert_eq2!(output.extract_to_line_end(), "italic");
                }
                _ => panic!("Expected success result"),
            }
        }

        // Test with content containing spaces
        {
            let lines = &[GCString::new("_hello world_")];
            let input = AsStrSlice::from(lines);
            let result = delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi(
                input, "_",
            );
            match result {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_to_line_end(), "");
                    assert_eq2!(output.extract_to_line_end(), "hello world");
                }
                _ => panic!("Expected success result"),
            }
        }
    }
}
