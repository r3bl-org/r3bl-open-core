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

use nom::{branch::alt,
          bytes::complete::tag,
          character::complete::anychar,
          combinator::{not, recognize},
          multi::many0,
          sequence::preceded,
          Parser};

use crate::{md_parser::constants::NEW_LINE, AsStrSlice, NomError};

/// This returns a function, which implements [Parser]. So call `input()` on it
/// or pass it to other `nom` combination functions.
///
/// Take text until an optional EOL character is found, or end of input is reached. If an
/// EOL character is found:
/// - The EOL character is not included in the output.
/// - The EOL character is not consumed, and is part of the remainder.
/// - The EOL character is defined by [NEW_LINE] string constant.
///
/// Here are some examples:
///
/// | input               | rem            |  output           |
/// | ------------------- | -------------- | ----------------- |
/// | `"\nfoo\nbar"`      | `"\nfoo\nbar"` | `""`              |
/// | `"Hello, world!\n"` | `"\n"`         | `"Hello, world!"` |
/// | `"Hello, world!"`   | `""`           | `"Hello, world!"` |
#[rustfmt::skip]
pub fn parser_take_text_until_eol_or_eoi_alt<'a>() ->
    impl Parser<AsStrSlice<'a>, Output = AsStrSlice<'a>, Error = NomError<AsStrSlice<'a>>>
{
    recognize( /* match anychar up until a denied string below is encountered */
        many0( /* may match nothing */
            preceded( /* match anything that isn't in the denied strings list below */
                /* prefix is discarded, it doesn't match anything, only errors out for denied strings */
                not( /* error out if starts w/ denied strings below */
                    alt((
                        tag(NEW_LINE),
                    ))
                ),
                /* output - keep char if it didn't error out above */
                anychar,
            )
        )
    )
}

#[cfg(test)]
mod test_text_until_opt_eol {
    use super::*;
    use crate::{assert_eq2, AsStrSlice, GCString, as_str_slice_test_case};

    #[test]
    fn test_input_starts_with_new_line() {
        // Starts with new line.
        as_str_slice_test_case!(input, "\nfoo\nbar");
        let (remainder, result) = parser_take_text_until_eol_or_eoi_alt()
            .parse(input)
            .unwrap();
        // Should return empty content when input immediately starts with newline
        assert_eq2!(result.extract_to_slice_end(), "");
        // Remainder should start from the newline
        assert_eq2!(remainder.extract_to_slice_end(), "\nfoo\nbar");
    }

    #[test]
    fn test_input_with_eol() {
        // With EOL.
        as_str_slice_test_case!(input, "Hello, world!\n");
        let (rem, output) = parser_take_text_until_eol_or_eoi_alt()
            .parse(input)
            .unwrap();
        println!("{:8}: {:?}", "input", "_input_array");
        println!("{:8}: {:?}", "rem", rem);
        println!("{:8}: {:?}", "output", output);
        assert_eq2!(output.extract_to_slice_end(), "Hello, world!");
        assert_eq2!(rem.extract_to_slice_end(), "\n");
    }

    #[test]
    fn test_input_without_eol() {
        // Without EOL.
        as_str_slice_test_case!(input, "Hello, world!");
        let (rem, output) = parser_take_text_until_eol_or_eoi_alt()
            .parse(input)
            .unwrap();
        println!("\n{:8}: {:?}", "input", "_input_array");
        println!("{:8}: {:?}", "rem", rem);
        println!("{:8}: {:?}", "output", output);
        assert_eq2!(output.extract_to_slice_end(), "Hello, world!");
        assert_eq2!(rem.extract_to_slice_end(), "");
    }

    #[test]
    fn test_another_input_starts_with_eol() {
        // Begins with EOL, then has some text, and ends with EOL.
        {
            as_str_slice_test_case!(input, "\nfoo\nbar");
            let (rem, output) = parser_take_text_until_eol_or_eoi_alt()
                .parse(input)
                .unwrap();
            println!("\n{:8}: {:?}", "input", "_input_array");
            println!("{:8}: {:?}", "rem", rem);
            println!("{:8}: {:?}", "output", output);
            assert_eq2!(output.extract_to_slice_end(), "");
            assert_eq2!(rem.extract_to_slice_end(), "\nfoo\nbar");
        }

        // Multi-element input with empty line (new line only), "foo", and "bar".
        // Note the extra new line at the end of the rem. This is expected behavior
        // for `AsStrSlice` which generates a new line at the end of the slice
        // when there is more than 1 line.
        {
            as_str_slice_test_case!(input, "", "foo", "bar");
            let (rem, output) = parser_take_text_until_eol_or_eoi_alt()
                .parse(input)
                .unwrap();
            println!("\n{:8}: {:?}", "input", "_input_array");
            println!("{:8}: {:?}", "rem", rem);
            println!("{:8}: {:?}", "output", output);
            assert_eq2!(output.extract_to_slice_end(), "");
            assert_eq2!(rem.extract_to_slice_end(), "\nfoo\nbar\n");
        }
    }
}
