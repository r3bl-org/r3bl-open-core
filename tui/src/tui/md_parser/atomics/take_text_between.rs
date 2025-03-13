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
          sequence::tuple,
          IResult};
use r3bl_ansi_color::{green, red};

use crate::{constants::NEW_LINE, DEBUG_MD_PARSER_STDOUT};

/// Takes the text between the start and end delimiters. Will error out if this text
/// contains a new line.
pub fn take_text_between_delims_err_on_new_line<'input>(
    input: &'input str,
    start_delim: &'input str,
    end_delim: &'input str,
) -> IResult<&'input str, &'input str> {
    DEBUG_MD_PARSER_STDOUT.then(|| {
        println!(
            "\n{} specialized parser take text between delims err on new line: \ninput: {:?}, start_delim: {:?}, end_delim: {:?}",
            green("■■"),
            input,
            start_delim,
            end_delim
        );
    });

    let it = take_text_between(start_delim, end_delim)(input);

    if let Ok((_, output)) = &it {
        if output.contains(NEW_LINE) {
            DEBUG_MD_PARSER_STDOUT.then(|| {
                println!("{} parser error out for input: {:?}", red("⬢⬢"), input);
            });
            return Err(nom::Err::Error(nom::error::Error {
                input: output,
                code: ErrorKind::CrLf,
            }));
        };
    }

    if it.is_err() {
        DEBUG_MD_PARSER_STDOUT.then(|| {
            println!("{} parser error out for input: {:?}", red("⬢⬢"), input);
        });
    }
    it
}

/// More info: <https://github.com/dimfeld/export-logseq-notes/blob/40f4d78546bec269ad25d99e779f58de64f4a505/src/parse_string.rs#L132>
#[rustfmt::skip]
fn take_text_between<'input>(
    start_tag: &'input str,
    end_tag: &'input str,
) -> impl FnMut(&'input str) -> IResult<&'input str, &'input str> {
    map(
        tuple(
            (tag(start_tag), take_until(end_tag), tag(end_tag))
        ),
        | (_start, middle, _end)| middle
    )
}

#[cfg(test)]
mod tests_parse_take_between {
    use nom::{error::{Error, ErrorKind},
              Err as NomErr};
    use r3bl_core::assert_eq2;

    use super::*;

    #[test]
    fn test_fenced() {
        let input = "_foo bar baz_";
        let it = take_text_between("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(it, Ok(("", "foo bar baz")));

        let input = "_foo bar baz";
        let it = take_text_between("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "foo bar baz",
                code: ErrorKind::TakeUntil
            }))
        );

        let input = "foo _bar_ baz";
        let it = take_text_between("_", "_")(input);
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "foo _bar_ baz",
                code: ErrorKind::Tag
            }))
        );
    }

    #[test]
    fn test_parse_fenced_no_new_line() {
        let input = "_foo\nbar_";
        let it = take_text_between_delims_err_on_new_line(input, "_", "_");
        println!("it: {:?}", it);
        assert_eq2!(
            it,
            Err(NomErr::Error(Error {
                input: "foo\nbar",
                code: ErrorKind::CrLf
            }))
        );

        let input = "_foo bar_";
        let it = take_text_between_delims_err_on_new_line(input, "_", "_");
        println!("it: {:?}", it);
        assert_eq2!(it, Ok(("", "foo bar")));
    }
}
