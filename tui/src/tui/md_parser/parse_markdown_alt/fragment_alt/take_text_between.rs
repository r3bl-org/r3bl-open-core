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
          FindSubstring,
          IResult,
          Parser};

use crate::{fg_green,
            fg_red,
            md_parser::constants::NEW_LINE,
            AsStrSlice,
            DEBUG_MD_PARSER_STDOUT};

/// Takes the text between the start and end delimiters. Will error out if this text
/// contains a new line.
pub fn take_text_between_delims_err_on_new_line<'i>(
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

    let it = take_text_between(start_delim, end_delim).parse(input);

    if let Ok((_, ref output)) = it {
        if output.find_substring(NEW_LINE).is_some() {
            // DEBUG_MD_PARSER_STDOUT.then(|| {
            //     println!("{} parser error out for input: {:?}", fg_red("⬢⬢"), input);
            // });
            return Err(nom::Err::Error(nom::error::Error {
                input: (*output).clone(),
                code: ErrorKind::CrLf,
            }));
        }
    }

    // if it.is_err() {
    //     DEBUG_MD_PARSER_STDOUT.then(|| {
    //         println!("{} parser error out for input: {:?}", fg_red("⬢⬢"), input);
    //     });
    // }

    it
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
fn take_text_between<'i>(
    start_tag: &'i str,
    end_tag: &'i str,
) -> impl Parser<AsStrSlice<'i>, Output = AsStrSlice<'i>, Error = nom::error::Error<AsStrSlice<'i>>> {
    map(
        (
            tag(start_tag),
            take_until(end_tag),
            tag(end_tag),
        ),
        |(_start, middle, _end)| middle,
    )
}

#[cfg(test)]
mod tests_parse_take_between {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};

    use super::*;
    use crate::{assert_eq2, GCString};

    #[test]
    fn test_fenced() {
        let lines = vec![GCString::new("_foo bar baz_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between("_", "_").parse(input);
        println!("it: {it:?}");

        let empty_lines = vec![GCString::new("")];
        let content_lines = vec![GCString::new("foo bar baz")];
        assert_eq2!(it, Ok((AsStrSlice::from(&empty_lines), AsStrSlice::from(&content_lines))));

        let lines = vec![GCString::new("_foo bar baz")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between("_", "_").parse(input);
        println!("it: {it:?}");

        let error_lines = vec![GCString::new("foo bar baz")];
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: AsStrSlice::from(&error_lines),
                code: ErrorKind::TakeUntil
            }))
        );

        let lines = vec![GCString::new("foo _bar_ baz")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between("_", "_").parse(input);
        println!("it: {it:?}");

        let error_lines = vec![GCString::new("foo _bar_ baz")];
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: AsStrSlice::from(&error_lines),
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_fenced_no_new_line() {
        let lines = vec![GCString::new("_foo\nbar_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_delims_err_on_new_line(input, "_", "_");
        println!("it: {it:?}");

        let error_lines = vec![GCString::new("foo\nbar")];
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: AsStrSlice::from(&error_lines),
                code: ErrorKind::CrLf
            }))
        );

        let lines = vec![GCString::new("_foo bar_")];
        let input = AsStrSlice::from(&lines);
        let it = take_text_between_delims_err_on_new_line(input, "_", "_");
        println!("it: {it:?}");

        let empty_lines = vec![GCString::new("")];
        let content_lines = vec![GCString::new("foo bar")];
        assert_eq2!(it, Ok((AsStrSlice::from(&empty_lines), AsStrSlice::from(&content_lines))));
    }
}
