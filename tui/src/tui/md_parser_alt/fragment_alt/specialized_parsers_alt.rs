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

use super::{specialized_parser_delim_matchers_alt,
            take_text_between_delims_err_on_new_line_alt};
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
            NomErr,
            NomError,
            NomErrorKind,
            DEBUG_MD_PARSER_STDOUT};

pub fn parse_fragment_starts_with_underscore_err_on_new_line_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    specialized_parser_delim_matchers_alt::take_starts_with_delim_no_new_line(
        input, UNDERSCORE,
    )
}

pub fn parse_fragment_starts_with_star_err_on_new_line_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {
    specialized_parser_delim_matchers_alt::take_starts_with_delim_no_new_line(input, STAR)
}

pub fn parse_fragment_starts_with_backtick_err_on_new_line_alt<'a>(
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
        return Err(NomErr::Error(NomError {
            input: output,
            code: NomErrorKind::Tag,
        }));
    }

    // Otherwise, return the text between the backticks.
    specialized_parser_delim_matchers_alt::take_starts_with_delim_no_new_line(
        input_clone,
        BACK_TICK,
    )
}

pub fn parse_fragment_starts_with_left_image_err_on_new_line_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, HyperlinkData<'a>> {
    let input_clone_dbg = input.clone();

    // Parse the text between the image tags.
    let result_first =
        take_text_between_delims_err_on_new_line_alt(input, LEFT_IMAGE, RIGHT_IMAGE);

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
    let result_second = take_text_between_delims_err_on_new_line_alt(
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
            part_between_image_tags.extract_remaining_text_content_in_line(),
            part_between_parenthesis.extract_remaining_text_content_in_line(),
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

pub fn parse_fragment_starts_with_left_link_err_on_new_line_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, HyperlinkData<'a>> {
    let input_clone_dbg = input.clone();

    // Parse the text between the brackets.
    let result_first =
        take_text_between_delims_err_on_new_line_alt(input, LEFT_BRACKET, RIGHT_BRACKET);

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
    let result_second = take_text_between_delims_err_on_new_line_alt(
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
            part_between_brackets.extract_remaining_text_content_in_line(),
            part_between_parenthesis.extract_remaining_text_content_in_line(),
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
pub fn parse_fragment_starts_with_checkbox_into_str_alt<'a>(
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
pub fn parse_fragment_starts_with_checkbox_checkbox_into_bool_alt<'a>(
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
    use crate::{assert_eq2, GCString, NomErr, NomErrorKind};

    #[test]
    fn test_parse_fragment_starts_with_underscore_err_on_new_line_alt() {
        // "_here is italic_" -> ok
        {
            let lines = &[GCString::new("_here is italic_")];
            let input = AsStrSlice::from(lines);
            let exp_out = "here is italic";
            let exp_rem = "";
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), exp_rem);
                    assert_eq2!(output.extract_remaining_text_content_in_line(), exp_out);
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
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), exp_rem);
                    assert_eq2!(output.extract_remaining_text_content_in_line(), exp_out);
                }
                _ => panic!("Expected success result"),
            }
        }

        // "_here is italic" -> error
        {
            let err_input = "_here is italic";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is italic_" -> error
        {
            let err_input = "here is italic_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is italic" -> error
        {
            let err_input = "here is italic";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "_" -> error
        {
            let err_input = "_";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "" -> error
        {
            let err_input = "";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_underscore_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_parse_fragment_starts_with_star_err_on_new_line_alt() {
        // "*here is bold*" -> ok
        {
            let lines = &[GCString::new("*here is bold*")];
            let input = AsStrSlice::from(lines);
            let exp_out = "here is bold";
            let exp_rem = "";
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), exp_rem);
                    assert_eq2!(output.extract_remaining_text_content_in_line(), exp_out);
                }
                _ => panic!("Expected success result"),
            }
        }

        // "*here is bold" -> error
        {
            let err_input = "*here is bold";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is bold*" -> error
        {
            let err_input = "here is bold*";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "here is bold" -> error
        {
            let err_input = "here is bold";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "*" -> error
        {
            let err_input = "*";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        // "" -> error
        {
            let err_input = "";
            let lines = &[GCString::new(err_input)];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_star_err_on_new_line_alt(input);
            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        err_input
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_parse_fragment_starts_with_backtick_err_on_new_line_alt() {
        {
            let lines = &[GCString::new("")];
            let input = AsStrSlice::from(lines);
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(error.input.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(error.input.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("`here is code")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        "`here is code"
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("here is code`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        "here is code`"
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("``")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(output.extract_remaining_text_content_in_line(), "");
                }
                _ => panic!("Expected success result"),
            }
        }

        {
            let lines = vec![GCString::new("`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        "`"
                    );
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(error.input.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(error.code, NomErrorKind::Fail);
                }
                _ => panic!("Expected error result"),
            }
        }

        {
            let lines = vec![GCString::new("`abcd`")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Ok((rem, output)) => {
                    assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
                    assert_eq2!(output.extract_remaining_text_content_in_line(), "abcd");
                }
                _ => panic!("Expected success result"),
            }
        }

        {
            let lines = vec![GCString::new("```")];
            let input = AsStrSlice::from(lines.as_slice());
            let res = parse_fragment_starts_with_backtick_err_on_new_line_alt(input);

            match res {
                Err(NomErr::Error(error)) => {
                    assert_eq2!(
                        error.input.extract_remaining_text_content_in_line(),
                        "```"
                    );
                    assert_eq2!(error.code, NomErrorKind::Tag);
                }
                _ => panic!("Expected error result"),
            }
        }
    }

    #[test]
    fn test_parse_fragment_starts_with_left_link_err_on_new_line_alt() {
        {
            let lines = &[GCString::new("![alt text](image.jpg)")];
            let input = AsStrSlice::from(lines);

            let (rem, output) =
                parse_fragment_starts_with_left_image_err_on_new_line_alt(input).unwrap();
            assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
            assert_eq2!(output, HyperlinkData::new("alt text", "image.jpg"));
        }

        {
            let lines = &[GCString::new("[title](https://www.example.com)")];
            let res = parse_fragment_starts_with_left_link_err_on_new_line_alt(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(rem.extract_remaining_text_content_in_line(), "");
            assert_eq2!(
                output,
                HyperlinkData::new("title", "https://www.example.com")
            );
        }
    }

    #[test]
    fn test_parse_fragment_checkbox_into_str() {
        // Test [x] checkbox.
        {
            let lines = &[GCString::new("[x] here is a checkbox")];
            let res =
                parse_fragment_starts_with_checkbox_into_str_alt(AsStrSlice::from(lines));

            let (rem, output) = res.unwrap();
            assert_eq2!(output.extract_remaining_text_content_in_line(), "[x]");
            assert_eq2!(
                rem.extract_remaining_text_content_in_line(),
                " here is a checkbox"
            );
        }

        // Test [ ] checkbox.
        {
            let lines = &[GCString::new("[ ] here is a checkbox")];
            let res =
                parse_fragment_starts_with_checkbox_into_str_alt(AsStrSlice::from(lines));

            assert!(res.is_ok());
            let (rem, output) = res.unwrap();
            assert_eq2!(output.extract_remaining_text_content_in_line(), "[ ]");
            assert_eq2!(
                rem.extract_remaining_text_content_in_line(),
                " here is a checkbox"
            );
        }
    }

    #[test]
    fn test_parse_fragment_checkbox_into_bool() {
        // Test [x] checkbox.
        {
            let lines = &[GCString::new("[x] here is a checkbox")];
            let res = parse_fragment_starts_with_checkbox_checkbox_into_bool_alt(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(output, true);
            assert_eq2!(
                rem.extract_remaining_text_content_in_line(),
                " here is a checkbox"
            );
        }

        // Test [ ] checkbox.
        {
            let lines = &[GCString::new("[ ] here is a checkbox")];
            let res = parse_fragment_starts_with_checkbox_checkbox_into_bool_alt(
                AsStrSlice::from(lines),
            );

            let (rem, output) = res.unwrap();
            assert_eq2!(output, false);
            assert_eq2!(
                rem.extract_remaining_text_content_in_line(),
                " here is a checkbox"
            );
        }
    }
}
