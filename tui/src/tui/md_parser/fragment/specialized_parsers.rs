// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use nom::{IResult, Parser,
          branch::alt,
          bytes::complete::tag,
          combinator::{map, recognize},
          multi::many0};

use super::specialized_parser_delim_matchers;
use crate::{DEBUG_MD_PARSER_STDOUT, HyperlinkData, fg_blue, fg_red,
            md_parser::constants::{BACK_TICK, CHECKED, LEFT_BRACKET, LEFT_IMAGE,
                                   LEFT_PARENTHESIS, RIGHT_BRACKET, RIGHT_IMAGE,
                                   RIGHT_PARENTHESIS, STAR, UNCHECKED, UNDERSCORE},
            take_text_between_delims_err_on_new_line};

/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The underlying delim matcher
/// handles null padding.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with an underscore or contains
/// a newline.
pub fn parse_fragment_starts_with_underscore_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(
        input, UNDERSCORE,
    )
}

/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The underlying delim matcher
/// handles null padding.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with a star or contains a
/// newline.
pub fn parse_fragment_starts_with_star_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(input, STAR)
}

/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The underlying delim matcher
/// handles null padding.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with a backtick or contains a
/// newline.
pub fn parse_fragment_starts_with_backtick_err_on_new_line(
    input: &str,
) -> IResult<&str, &str> {
    // Count the number of consecutive backticks. If there are more than 2 backticks,
    // return an error, since this could be a code block.
    let it = recognize(many0(tag(BACK_TICK))).parse(input);
    if it.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with backtick: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                input,
                BACK_TICK
            );
        });
    }
    let (_, output) = it?;
    if output.len() > 2 {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "{} more than 2 backticks in input:{:?}",
                fg_red("⬢⬢"),
                input
            );
        });
        return Err(nom::Err::Error(nom::error::Error {
            input: output,
            code: nom::error::ErrorKind::Tag,
        }));
    }

    // Otherwise, return the text between the backticks.
    specialized_parser_delim_matchers::take_starts_with_delim_no_new_line(
        input, BACK_TICK,
    )
}

/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The underlying delim matcher
/// handles null padding.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain a valid image markdown syntax
/// or contains a newline.
pub fn parse_fragment_starts_with_left_image_err_on_new_line(
    input: &str,
) -> IResult<&str, HyperlinkData<'_>> {
    // Parse the text between the image tags.
    let result_first =
        take_text_between_delims_err_on_new_line(input, LEFT_IMAGE, RIGHT_IMAGE);
    if result_first.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                    "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                    fg_red("⬢⬢"),
                    input,
                    LEFT_IMAGE
                );
        });
    }
    let (rem, part_between_image_tags) = result_first?;

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_err_on_new_line(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );
    if result_second.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                    "\n{} specialized parser error out with image: \ninput: {:?}, delim: {:?}",
                    fg_red("⬢⬢"),
                    rem,
                    LEFT_PARENTHESIS
                );
        });
    }
    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((part_between_image_tags, part_between_parenthesis)),
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

/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`. The underlying delim matcher
/// handles null padding.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't contain a valid link markdown syntax
/// or contains a newline.
pub fn parse_fragment_starts_with_left_link_err_on_new_line(
    input: &str,
) -> IResult<&str, HyperlinkData<'_>> {
    // Parse the text between the brackets.
    let result_first =
        take_text_between_delims_err_on_new_line(input, LEFT_BRACKET, RIGHT_BRACKET);
    if result_first.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                input,
                LEFT_BRACKET
            );
        });
    }
    let (rem, part_between_brackets) = result_first?;

    // Parse the text between the parenthesis.
    let result_second = take_text_between_delims_err_on_new_line(
        rem,
        LEFT_PARENTHESIS,
        RIGHT_PARENTHESIS,
    );
    if result_second.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!(
                "\n{} specialized parser error out with link: \ninput: {:?}, delim: {:?}",
                fg_red("⬢⬢"),
                rem,
                LEFT_PARENTHESIS
            );
        });
    }
    let (rem, part_between_parenthesis) = result_second?;

    let it = Ok((
        rem,
        HyperlinkData::from((part_between_brackets, part_between_parenthesis)),
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
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with a valid checkbox syntax.
pub fn parse_fragment_starts_with_checkbox_into_str(input: &str) -> IResult<&str, &str> {
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
///
/// # Null Padding Invariant
///
/// This parser expects input where lines end with `\n` followed by zero or more `\0`
/// characters, as provided by `ZeroCopyGapBuffer::as_str()`.
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with a valid checkbox syntax.
pub fn parse_fragment_starts_with_checkbox_checkbox_into_bool(
    input: &str,
) -> IResult<&str, bool> {
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
