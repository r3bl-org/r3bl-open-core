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

use nom::{bytes::complete::{tag, take_until},
          combinator::map,
          error::ErrorKind,
          IResult,
          Parser};

use crate::{fg_green,
            fg_red,
            md_parser::constants::NEW_LINE,
            AsStrSlice,
            NomErr,
            NomError,
            DEBUG_MD_PARSER_STDOUT};

/// Takes the text between the start and end delimiters. Will error out if this text
/// contains a new line.
pub fn take_text_between_delims_enclosed_err_on_new_line_alt<'i>(
    input: AsStrSlice<'i>,
    start_delim: &'i str,
    end_delim: &'i str,
) -> IResult<AsStrSlice<'i>, AsStrSlice<'i>> {
    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "\n{} specialized parser take text between delims err on new line: \ninput: {:?}, start_delim: {:?}, end_delim: {:?}",
            fg_green("■■"),
            input,
            start_delim,
            end_delim
        );
    });

    let input_clone_dbg = input.clone();

    match take_text_between_alt(start_delim, end_delim, input) {
        Ok((remaining, output)) => {
            // If the output contains a new line, return an error.
            if output.contains_in_current_line(NEW_LINE) {
                DEBUG_MD_PARSER_STDOUT.then(|| {
                    println!(
                        "{} parser error out for input: {:?}",
                        fg_red("⬢⬢"),
                        input_clone_dbg
                    );
                });
                Err(NomErr::Error(NomError {
                    input: output,
                    code: ErrorKind::CrLf,
                }))
            } else {
                Ok((remaining, output))
            }
        }
        Err(err) => {
            DEBUG_MD_PARSER_STDOUT.then(|| {
                println!(
                    "{} parser error out for input: {:?}",
                    fg_red("⬢⬢"),
                    input_clone_dbg
                );
            });
            Err(err)
        }
    }
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
pub fn take_text_between_alt<'i>(
    start_tag: &'i str,
    end_tag: &'i str,
    input: AsStrSlice<'i>,
) -> IResult<AsStrSlice<'i>, AsStrSlice<'i>> {
    // Declarative way (idiomatic).
    map(
        (
            tag(start_tag),
            take_until(end_tag),
            tag(end_tag),
        ),
        |(_start, middle, _end)| middle,
    ).parse(input)

    // Imperative way.
    // let (rem, _) = tag(start_tag)(input)?;
    // let (rem_2, middle) = take_until(end_tag)(rem)?;
    // let (rem_3, _) = tag(end_tag)(rem_2)?;
    // Ok((rem_3, middle))
}

#[cfg(test)]
mod tests_parse_take_between {
    use nom::{error::ErrorKind, Err as NomErr};

    use super::*;
    use crate::{idx, GCString};

    #[test]
    fn test_fenced_success() {
        let lines = [GCString::new("_foo bar baz_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_alt("_", "_", input);
        println!("it: {it:#?}");

        // Extract the result for comparison
        let (rem, output) = it.unwrap();

        // Check that remaining is empty by converting to string and checking length
        let remaining_str = format!("{rem}");
        assert_eq!(remaining_str.len(), 0);

        // Check that extracted contains "foo bar baz"
        let extracted_str = format!("{output}");
        assert_eq!(extracted_str, "foo bar baz");
    }

    #[test]
    fn test_fenced_missing_end_tag() {
        let lines = [GCString::new("_foo bar baz")];
        let input = AsStrSlice::from(&lines);
        assert_eq!(input.char_index, idx(0));

        let res = take_text_between_alt("_", "_", input);
        println!("it: {res:?}");

        match res {
            Ok(_) => panic!("Expected an error, but got Ok"),
            Err(nom::Err::Error(error)) => {
                assert_eq!(error.code, nom::error::ErrorKind::TakeUntil);
                // `tag("_")` moved this forward by 1. it is no longer equal to `input`.
                assert_eq!(error.input.char_index, idx(1));
            }
            Err(other_err) => panic!("Expected Error variant, but got: {other_err:?}"),
        }
    }

    #[test]
    fn test_fenced_missing_start_tag() {
        let lines = [GCString::new("foo _bar_ baz")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_alt("_", "_", input);
        println!("it: {it:?}");

        // Check that the result is an error with Tag error kind
        match it {
            Err(NomErr::Error(err)) => {
                assert_eq!(err.code, ErrorKind::Tag);
            }
            _ => panic!("Expected Err(NomErr::Error) with Tag error kind"),
        }
    }

    #[test]
    fn test_parse_fenced_with_new_line_error() {
        let lines = [GCString::new("_foo\nbar_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_delims_enclosed_err_on_new_line_alt(input, "_", "_");
        println!("it: {it:?}");

        // Check that the result is an error with CrLf error kind
        match it {
            Err(NomErr::Error(err)) => {
                assert_eq!(err.code, ErrorKind::CrLf);
            }
            _ => panic!("Expected Err(NomErr::Error) with CrLf error kind"),
        }
    }

    #[test]
    fn test_parse_fenced_without_new_line_success() {
        let lines = [GCString::new("_foo bar_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_delims_enclosed_err_on_new_line_alt(input, "_", "_");
        println!("it: {it:?}");

        // Extract the result for comparison
        let (remaining, extracted) = it.unwrap();

        // Check that remaining is empty by converting to string and checking length
        let remaining_str = format!("{remaining}");
        assert_eq!(remaining_str.len(), 0);

        // Check that extracted contains "foo bar"
        let extracted_str = format!("{extracted}");
        assert_eq!(extracted_str, "foo bar");
    }
}
