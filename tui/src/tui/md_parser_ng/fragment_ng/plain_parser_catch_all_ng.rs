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

//! This module implements the lowest priority parser for "plain text" Markdown
//! fragments.
//!
//! This is the lowest priority parser called by
//! [`crate::fragment_ng::parse_fragments_in_a_line_ng::parse_inline_fragments_until_eol_or_eoi_ng`].
//!
//! It matches anything that is not a special character. This is used to parse plain text.
//! It works with the specialized parsers in
//! [`crate::fragment_ng::parse_fragments_in_a_line_ng::parse_inline_fragments_until_eol_or_eoi_ng`] such as:
//! - [`crate::fragment_ng::parse_fragment_starts_with_underscore_err_on_new_line_ng`],
//! - [`crate::fragment_ng::parse_fragment_starts_with_star_err_on_new_line_ng`],
//! - [`crate::fragment_ng::parse_fragment_starts_with_backtick_err_on_new_line_ng`],
//! - etc.
//!
//! It also works hand in hand with
//! [`crate::specialized_parsers_ng::delim_matchers::take_starts_with_delim_enclosed_until_eol_or_eoi()`] which is
//! used by the specialized parsers.
//!
//! To see this in action, set the [`crate::DEBUG_MD_PARSER_STDOUT`] to true, and run all
//! the tests in this module.

use nom::{branch::alt,
          bytes::complete::{tag, take_till1},
          character::complete::anychar,
          combinator::{not, recognize},
          multi::many1,
          sequence::preceded,
          IResult,
          Input,
          Parser};

use crate::{fg_blue,
            fg_magenta,
            fg_red,
            md_parser::constants::{BACK_TICK,
                                   LEFT_BRACKET,
                                   LEFT_IMAGE,
                                   NEW_LINE,
                                   NEW_LINE_CHAR,
                                   STAR,
                                   UNDERSCORE},
            specialized_parsers_ng,
            AsStrSlice,
            CharLengthExt as _,
            NErr,
            NError,
            NErrorKind,
            DEBUG_MD_PARSER_STDOUT};

/// This is the lowest priority parser called by
/// [`crate::fragment_ng::parse_fragments_in_a_line_ng::parse_inline_fragments_until_eol_or_eoi_ng`], which itself is called:
/// 1. Repeatedly in a loop by functions like
///    [`crate::standard_ng::parse_line_text_advance_ng()`].
/// 2. And by other similar functions.
///
/// It will match anything that is not a special character. This is used to parse plain
/// text.
///
/// This function handles three cases:
/// 1. Normal case: Input doesn't start with special characters
///    - Takes text until the first special character is encountered
///    - Splits the input at that point and returns the plain text and remainder
/// 2. Special edge case: Input starts with a single special character
///    ([`crate::md_parser::constants::UNDERSCORE`],
///    [`crate::md_parser::constants::STAR`], [`crate::md_parser::constants::BACK_TICK`])
///    - Handles the case where there is no closing delimiter
///    - Returns the special character as plain text
/// 3. Normal edge case: Input starts with special characters but doesn't match any
///    specialized parser
///    - Takes text until the first new line
///    - Returns it as plain text
///
/// This gives the other more specialized parsers a chance to address these special
/// characters (like italic, bold, links, etc.), when this function is called repeatedly:
/// - By functions like [`crate::standard_ng::parse_line_text_advance_ng()`],
/// - Which repeatedly calls [`crate::fragment_ng::parse_fragments_in_a_line_ng::parse_inline_fragments_until_eol_or_eoi_ng`]. This
///   function actually runs the specialized parsers.
/// - Which calls this function repeatedly (if the specialized parsers don't match & error
///   out). This serves as a "catch all" parser.
///
/// If these more specialized parsers error out, then this parser will be called
/// again to parse the remainder; this time the input will start with the special
/// character; and in this edge case it will take the input until the first new line; or
/// until the end of the input.
///
/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
/// See: [`crate::specialized_parsers_ng::delim_matchers::count_delim_occurrences_until_eol_or_eoi`].
pub fn parse_fragment_plain_text_until_eol_or_eoi_ng(
    input: AsStrSlice<'_>,
) -> IResult<AsStrSlice<'_>, AsStrSlice<'_>> {
    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!("\n{} plain parser, input: {:?}", fg_magenta("██"), input);
    });

    // Check if input doesn't start with special characters.
    if check_input_starts_with(&input, &get_sp_char_set_2()).is_none() {
        // Case 1: Normal - input doesn't start with special characters.
        return parse_plain_text_until_special_char(input);
    }

    // Case 2: Edge case - input starts with single special character.
    if let Some(result) = try_parse_single_special_char(&input) {
        return result;
    }

    // Case 3: Fallback - take everything until newline as plain text.
    parse_plain_text_until_newline(input)
}

/// Handle the normal case: when input doesn't start with special characters.
///
/// This function processes input that doesn't start with special characters by taking
/// text until the first special character is encountered. It then splits the input at
/// that point and returns the plain text and remainder.
fn parse_plain_text_until_special_char(
    input: AsStrSlice<'_>,
) -> IResult<AsStrSlice<'_>, AsStrSlice<'_>> {
    // `tag_tuple` replaces the following:
    // `( tag(UNDERSCORE),   tag(STAR),
    //    tag(BACK_TICK),    tag(LEFT_IMAGE),
    //    tag(LEFT_BRACKET), tag(NEW_LINE) )`
    let tag_vec = get_sp_char_set_3()
        .into_iter()
        .map(tag::<&str, &str, NError<&str>>)
        .collect::<Vec<_>>();
    let tag_tuple = {
        debug_assert_eq!(tag_vec.len(), 6);
        tuple6(&tag_vec)
    };

    // Parse the input until any special character is encountered. This turns into
    // a &str since the tuples are all &str.
    let res: IResult<&str, &str> = recognize(
        // Match at least one character
        many1(preceded(
            // Don't match any of the special characters
            not(
                // Alternative of all special characters
                alt(tag_tuple),
            ),
            // Keep any character that isn't a special character
            anychar,
        )),
    )
    .parse(input.extract_to_line_end());

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} normal case :: {:?}",
            if res.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            res
        );
    });

    // Convert &str back into AsStrSlice, for both Ok and Err.
    match res {
        // input: "abcdefghijklmnopqr01234567890"
        //         ++output.len()++++--rem.len()
        //         ^new_output       ^new_rem
        //                           ↑
        //                       split here
        Ok((rem, output)) => {
            // Convert the output and remainder to character counts, not byte counts.
            // &str.len() returns byte count, but AsStrSlice methods expect character
            // counts.
            let output_char_count = output.len_chars();
            let rem_char_count = rem.len_chars();

            // Given the character count information, extract the following from `input`:
            let new_output = input.take(output_char_count.as_usize());
            let new_rem = input.skip_take_in_current_line(
                /* skip this many */ output_char_count,
                /* take this many */ rem_char_count,
            );

            Ok((new_rem, new_output))
        }
        Err(err) => {
            // Extract the ErrorKind from the original error
            let error_kind = match &err {
                NErr::Error(e) => e.code,
                NErr::Failure(e) => e.code,
                NErr::Incomplete(_) => NErrorKind::Complete,
            };

            // Convert to proper NomError type for AsStrSlice
            let nom_error = NError::new(input, error_kind);
            Err(NErr::Error(nom_error))
        }
    }
}

/// Handle the special edge case: when input starts with a single special character.
///
/// This function handles the case where the input starts with a single special character
/// (UNDERSCORE, STAR, `BACK_TICK`) and there is no closing delimiter.
fn try_parse_single_special_char<'a>(
    input: &AsStrSlice<'a>,
) -> Option<IResult<AsStrSlice<'a>, AsStrSlice<'a>>> {
    // Check for single UNDERSCORE, STAR, BACK_TICK.
    if let Some(special_str) = check_input_starts_with(input, &get_sp_char_set_1()) {
        let input_clone_counting = input.clone();

        let (count, _, _, _) =
            specialized_parsers_ng::delim_matchers::count_delim_occurrences_until_eol_or_eoi(
                input_clone_counting,
                special_str,
            );

        if count == 1 {
            let input_clone_parsing = input.clone();

            // Split the input, by returning the first part as plain text.
            let res: IResult<&str, &str> =
                recognize(many1(tag::<&str, &str, NError<&str>>(special_str)))
                    .parse(input_clone_parsing.extract_to_line_end());

            // Convert &str back into AsStrSlice for Ok.
            // input: "abcdefghijklmnopqr01234567890"
            //         ++output.len()++++--rem.len()
            //         ^new_output       ^new_rem
            //                           ↑
            //                       split here
            if let Ok((rem, output)) = res {
                DEBUG_MD_PARSER_STDOUT.then(|| {
                    println!(
                        "{} edge case -> special case :: rem: {:?}, output: {:?}",
                        fg_blue("▲▲"),
                        rem,
                        output
                    );
                });

                // Convert the output length to determine how many characters to take from
                // input.
                let output_len = output.len(); // Ok to use &str.len(), since not dealing with bytes.
                let rem_len = rem.len(); // Ok to use &str.len(), since not dealing with bytes.

                // Given the &str information, extract the following from `input`:
                let new_output = input.take(output_len);
                let new_rem = input.skip_take_in_current_line(
                    /* skip this many */ output_len,
                    /* take this many */ rem_len,
                );

                return Some(Ok((new_rem, new_output)));
            }
        }
    }

    None
}

/// Handle the normal edge case: fallback for other inputs.
///
/// This function handles the case where the input starts with special characters
/// but doesn't match any specialized parser. It takes text until the first new line.
fn parse_plain_text_until_newline(
    input: AsStrSlice<'_>,
) -> IResult<AsStrSlice<'_>, AsStrSlice<'_>> {
    // Take till the first new line or until the end of input. This does not consume
    // the new line.
    // For this not to return an error, at least 1 char must exist in
    // the input that is followed by a new line.
    let res = take_till1(|it: char| it == NEW_LINE_CHAR)(input);

    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "{} edge case -> normal case :: {:?}",
            if res.is_err() {
                fg_red("⬢⬢")
            } else {
                fg_blue("▲▲")
            },
            res
        );
    });

    res
}

/// Returns a set of special characters that require special handling.
///
/// This set contains characters (UNDERSCORE, STAR, `BACK_TICK`) that are used for
/// formatting in Markdown (like italic, bold, code). These characters must have at least
/// 2 occurrences to be parsed by the specialized parsers. If only 1 occurrence is found,
/// then the `handle_special_edge_case` function will handle it by returning the character
/// as plain text.
fn get_sp_char_set_1<'a>() -> [&'a str; 3] { [UNDERSCORE, STAR, BACK_TICK] }

/// Returns an extended set of special characters for detecting the normal edge case.
///
/// This set extends `get_sp_char_set_1()` with additional characters (`LEFT_IMAGE`,
/// `LEFT_BRACKET`) that are used to detect when the input starts with special characters.
/// In such cases, the `handle_normal_edge_case` function will take text until the first
/// new line, unless the special edge case applies:
/// 1. The character is in `get_sp_char_set_1()` and,
/// 2. There is exactly one occurrence of it.
fn get_sp_char_set_2<'a>() -> [&'a str; 5] {
    // We know this will always have exactly 5 elements:
    // 3 from get_sp_char_set_1() + 2 from [LEFT_IMAGE, LEFT_BRACKET]
    let mut result = [""; 5];
    let vec = get_sp_char_set_1()
        .iter()
        .chain([LEFT_IMAGE, LEFT_BRACKET].iter())
        .copied()
        .collect::<Vec<_>>();

    // Copy elements from vec to result
    for (i, &item) in vec.iter().enumerate() {
        result[i] = item;
    }

    result
}

/// Returns a complete set of special characters for the normal case.
///
/// This set extends `get_sp_char_set_2()` with `NEW_LINE` character. It's used in the
/// `handle_normal_case` function to detect when to stop parsing plain text. When any of
/// these characters is encountered, the input is split at that point. The text before
/// the special character is returned as plain text, and the remainder gets a chance to
/// be parsed by specialized parsers.
fn get_sp_char_set_3<'a>() -> [&'a str; 6] {
    // We know this will always have exactly 6 elements:
    // 5 from get_sp_char_set_2() + 1 from [NEW_LINE]
    let mut result = [""; 6];
    let vec = get_sp_char_set_2()
        .iter()
        .chain([NEW_LINE].iter())
        .copied()
        .collect::<Vec<_>>();

    // Copy elements from vec to result
    for (i, &item) in vec.iter().enumerate() {
        result[i] = item;
    }

    result
}

/// Checks if the input string starts with any of the strings in the provided character
/// set.
///
/// # Arguments
/// * `input` - The input string to check
/// * `char_set` - A slice of strings to check against
///
/// # Returns
/// * `Some(&str)` - The matching string from the character set if found
/// * `None` - If the input doesn't start with any of the strings in the character set
fn check_input_starts_with<'a>(
    input: &AsStrSlice<'a>,
    char_set: &[&'a str],
) -> Option<&'a str> {
    char_set
        .iter()
        .find(|&special_str| input.starts_with(special_str))
        .copied()
}

/// Converts a slice of 5 elements into a tuple of 5 references.
///
/// This utility function is used to convert a slice into a tuple format that can be used
/// with nom's `alt` function.
#[allow(dead_code)]
fn tuple5<T>(a: &[T]) -> (&T, &T, &T, &T, &T) { (&a[0], &a[1], &a[2], &a[3], &a[4]) }

/// Converts a slice of 6 elements into a tuple of 6 references.
///
/// This utility function is used to convert a slice into a tuple format that can be used
/// with nom's `alt` function.
fn tuple6<T>(a: &[T]) -> (&T, &T, &T, &T, &T, &T) {
    (&a[0], &a[1], &a[2], &a[3], &a[4], &a[5])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, GCString, NErr};

    #[test]
    fn test_check_input_starts_with() {
        assert_eq!(
            check_input_starts_with(
                &AsStrSlice::from(&[GCString::new("abc")]),
                &["a", "b", "c"]
            ),
            Some("a")
        );
        assert_eq!(
            check_input_starts_with(
                &AsStrSlice::from(&[GCString::new("abc")]),
                &["d", "e", "f"]
            ),
            None
        );
        assert_eq!(
            check_input_starts_with(
                &AsStrSlice::from(&[GCString::new("")]),
                &["a", "b", "c"]
            ),
            None
        );
        assert_eq!(
            check_input_starts_with(&AsStrSlice::from(&[GCString::new("abc")]), &[]),
            None
        );
    }

    #[test]
    fn test_get_sp_char_sets() {
        // Test get_sp_char_set_1
        let set1 = get_sp_char_set_1();
        assert_eq!(set1.len(), 3);
        assert!(set1.contains(&UNDERSCORE));
        assert!(set1.contains(&STAR));
        assert!(set1.contains(&BACK_TICK));

        // Test get_sp_char_set_2
        let set2 = get_sp_char_set_2();
        assert_eq!(set2.len(), 5);
        assert!(set2.contains(&UNDERSCORE));
        assert!(set2.contains(&STAR));
        assert!(set2.contains(&BACK_TICK));
        assert!(set2.contains(&LEFT_IMAGE));
        assert!(set2.contains(&LEFT_BRACKET));

        // Test get_sp_char_set_3
        let set3 = get_sp_char_set_3();
        assert_eq!(set3.len(), 6);
        assert!(set3.contains(&UNDERSCORE));
        assert!(set3.contains(&STAR));
        assert!(set3.contains(&BACK_TICK));
        assert!(set3.contains(&LEFT_IMAGE));
        assert!(set3.contains(&LEFT_BRACKET));
        assert!(set3.contains(&NEW_LINE));
    }

    #[test]
    fn test_tuple_functions() {
        let vec5 = vec![1, 2, 3, 4, 5];
        let tuple5_result = tuple5(&vec5);
        assert_eq!(tuple5_result, (&1, &2, &3, &4, &5));

        let vec6 = vec![1, 2, 3, 4, 5, 6];
        let tuple6_result = tuple6(&vec6);
        assert_eq!(tuple6_result, (&1, &2, &3, &4, &5, &6));
    }

    #[test]
    fn test_parse_fragment_plain_text_normal_case() {
        // Test normal case: plain text without special characters
        let lines = &[GCString::new("Hello world")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "Hello world");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_parse_fragment_plain_text_with_special_chars() {
        // Test normal case: text with special characters
        let lines = &[GCString::new("Hello *world")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "*world");
                assert_eq2!(out.extract_to_line_end(), "Hello ");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test with multiple special characters
        let lines = &[GCString::new("Hello _*`[!world")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "_*`[!world");
                assert_eq2!(out.extract_to_line_end(), "Hello ");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_parse_fragment_plain_text_special_edge_case() {
        // Test special edge case: single underscore
        let lines = &[GCString::new("_single")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "single");
                assert_eq2!(out.extract_to_line_end(), "_");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test special edge case: single star
        let lines = &[GCString::new("*single")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "single");
                assert_eq2!(out.extract_to_line_end(), "*");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test special edge case: single backtick
        let lines = &[GCString::new("`single")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "single");
                assert_eq2!(out.extract_to_line_end(), "`");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_parse_fragment_plain_text_normal_edge_case() {
        // Test normal edge case: starts with special character but not a single one
        let lines = &[GCString::new("**bold**")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "**bold**");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test normal edge case: starts with left bracket
        let lines = &[GCString::new("[link](url)")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "[link](url)");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test normal edge case: starts with left image
        let lines = &[GCString::new("![image](url)")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "![image](url)");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_parse_fragment_plain_text_with_newlines() {
        // Test with newline in the middle
        let lines = &[GCString::new("Hello\nworld")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "\nworld");
                assert_eq2!(out.extract_to_line_end(), "Hello");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test with multiple lines
        let lines = &[GCString::new("Hello"), GCString::new("world")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "Hello");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_parse_fragment_plain_text_empty_input() {
        // Test with empty input
        let lines = &[GCString::new("")];
        let input = AsStrSlice::from(lines);
        let res = parse_fragment_plain_text_until_eol_or_eoi_ng(input);

        match res {
            Ok(_) => panic!("Expected Err, got Ok"),
            Err(err) => match err {
                NErr::Error(error) => {
                    assert_eq2!(error.input.extract_to_line_end(), "");
                }
                _ => panic!("Expected Error, got different error type"),
            },
        }
    }

    #[test]
    fn test_handle_normal_case() {
        // Test normal case: plain text without special characters
        let lines = &[GCString::new("Hello world")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_special_char(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "");
                assert_eq2!(out.extract_to_line_end(), "Hello world");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test with special character in the middle
        let lines = &[GCString::new("Hello *world")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_special_char(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(rem.extract_to_line_end(), "*world");
                assert_eq2!(out.extract_to_line_end(), "Hello ");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }
    }

    #[test]
    fn test_handle_special_edge_case() {
        // Test special edge case: single underscore
        let lines = &[GCString::new("_single")];
        let input = AsStrSlice::from(lines);
        let res = try_parse_single_special_char(&input);

        match res {
            Some(Ok((rem, out))) => {
                assert_eq2!(rem.extract_to_line_end(), "single");
                assert_eq2!(out.extract_to_line_end(), "_");
            }
            Some(Err(err)) => panic!("Expected Ok, got Err: {err:?}"),
            None => panic!("Expected Some, got None"),
        }

        // Test not a special edge case: double underscore
        let lines = &[GCString::new("__double")];
        let input = AsStrSlice::from(lines);
        let res = try_parse_single_special_char(&input);

        assert!(res.is_none(), "Expected None for double underscore");

        // Test not a special edge case: doesn't start with special character
        let lines = &[GCString::new("normal")];
        let input = AsStrSlice::from(lines);
        let res = try_parse_single_special_char(&input);

        assert!(res.is_none(), "Expected None for normal text");
    }

    #[test]
    fn test_handle_normal_edge_case() {
        // Test normal edge case: starts with special character.
        let lines = &[GCString::new("**bold**")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_newline(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(out.extract_to_line_end(), "**bold**");
                assert_eq2!(rem.extract_to_line_end(), "");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Test with newline in the middle.
        let lines = &[GCString::new("**bold\ntext**")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_newline(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(out.extract_to_line_end(), "**bold");
                assert_eq2!(rem.extract_to_line_end(), "\ntext**");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Input: "hello world" (no newline)
        // Result: Ok(("", "hello world"))
        // takes everything, empty remainder
        let lines = &[GCString::new("hello world")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_newline(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(out.extract_to_line_end(), "hello world");
                assert_eq2!(rem.extract_to_line_end(), "");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Input: "single line text" (no newline)
        // Result: Ok(("", "single line text"))
        // consumes entire input
        let lines = &[GCString::new("single line text")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_newline(input);

        match res {
            Ok((rem, out)) => {
                assert_eq2!(out.extract_to_line_end(), "single line text");
                assert_eq2!(rem.extract_to_line_end(), "");
            }
            Err(err) => panic!("Expected Ok, got Err: {err:?}"),
        }

        // Input: "" (empty string)
        // Result: Err(...)
        // because take_till1 requires at least one character
        let lines = &[GCString::new("")];
        let input = AsStrSlice::from(lines);
        let res = parse_plain_text_until_newline(input);

        match res {
            Ok((rem, out)) => panic!(
                "Expected Err for empty input, got Ok: rem={:?}, out={:?}",
                rem.extract_to_line_end(),
                out.extract_to_line_end()
            ),
            Err(_err) => {
                // Expected error case - take_till1 requires at least one character
            }
        }
    }
}
