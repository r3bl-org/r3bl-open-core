// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! For use with specialized parsers for: [`crate::constants::UNDERSCORE`],
//! [`crate::constants::STAR`], and [`crate::constants::BACK_TICK`]. See:
//! [`crate::parse_fragment_plain_text_no_new_line()`].
//!
//! To see this in action, set the [`DEBUG_MD_PARSER_STDOUT`] to true, and run all the
//! tests in [`crate::parse_fragments_in_a_line`].

use crate::{DEBUG_MD_PARSER_STDOUT, fg_blue, fg_green, fg_red,
            md_parser::constants::NEW_LINE, take_text_between_delims_err_on_new_line};
use nom::{IResult, Parser, bytes::complete::tag, combinator::recognize, multi::many1};

/// Returns tuple:
/// 0. number of occurrences in the input, until the first "\n" or end of input.
/// 1. does the input start with the delimiter?
/// 2. is the input the delimiter?
/// 3. the delimiter.
#[must_use]
pub fn count_delim_occurrences_until_eol<'i>(
    input: &'i str,
    delim: &'i str,
) -> (usize, bool, bool, &'i str) {
    // If the input has a "\n" then split it at the first "\n", only count the number
    // of delims at the first part of the split.
    let (first_part, _) = input.split_at(input.find(NEW_LINE).unwrap_or(input.len()));
    let num_of_delim_occurrences = first_part.matches(delim).count();
    (
        num_of_delim_occurrences,
        input.starts_with(delim),
        input == delim,
        delim,
    )
}

/// See: [`parse_fragment_plain_text_no_new_li`
/// `ne1()`].
///
/// # Errors
///
/// Returns a nom parsing error if the input doesn't start with the delimiter or contains a newline.
#[rustfmt::skip]
pub fn take_starts_with_delim_no_new_line<'i>(
    input: &'i str,
    delim: &'i str,
) -> IResult<&'i str, &'i str> {
    // Check if there is a closing delim.
    let (num_of_delim_occurrences, starts_with_delim, input_is_delim, _) =
        count_delim_occurrences_until_eol(input, delim);

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
            println!("{a} parser error out for input: {i:?}",
                a = fg_red("⬢⬢"),
                i = input
            );
        });
        return Err(nom::Err::Error(nom::error::Error {
            input,
            code: nom::error::ErrorKind::Fail,
        }));
    }

    // If there is a closing delim, then we can safely take the text between the delim.
    if num_of_delim_occurrences > 1 {
        let it = take_text_between_delims_err_on_new_line(input, delim, delim);
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!("{a} it: {b:?}",
                a = fg_blue("▲▲"),
                b = it
            );
        });
        return it;
    }

    // Otherwise, we split the input at the first delim.
    let (rem, output) = recognize(many1(tag(delim))).parse(input)?;

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!("{a}, rem: {r:?}, output: {o:?}",
            a = fg_blue("▲▲"),
            r = rem,
            o = output
        );
    });

    Ok((rem, output))
}
