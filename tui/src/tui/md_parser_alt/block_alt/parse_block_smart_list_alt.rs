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

use nom::{multi::many0, IResult, Parser};

use crate::{md_parser_types::CheckboxParsePolicy,
            parse_inline_fragments_until_eol_or_eoi_alt,
            AsStrSlice,
            List,
            MdLineFragments};

/// Parse markdown text with a specific checkbox policy until the end of line or input.
/// This function is used as a utility for parsing markdown text that may contain checkboxes.
/// It returns a list of markdown line fragments [MdLineFragments].
/// 
/// Does not consume the end of line character if it exists. If an EOL character
/// [crate::constants::NEW_LINE] is found:
/// - The EOL character is not included in the output.
/// - The EOL character is not consumed, and is part of the remainder.
#[rustfmt::skip]
pub fn parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt<'a>(
    input: AsStrSlice<'a>,
    checkbox_policy: CheckboxParsePolicy,
) -> IResult<AsStrSlice<'a>, MdLineFragments<'a>> {
    let (input, output) = many0(
        |it| parse_inline_fragments_until_eol_or_eoi_alt(it, checkbox_policy)
    ).parse(input)?;

    let it = List::from(output);

    Ok((input, it))
}

#[cfg(test)]
mod tests_checkbox_policy {
    use super::*;
    use crate::{as_str_slice_test_case, assert_eq2, list, MdLineFragment};

    #[test]
    fn test_ignore_checkbox_empty_string() {
        {
            as_str_slice_test_case!(input, "");
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(fragments, list![]);
        }
    }

    #[test]
    fn test_ignore_checkbox_non_empty_string() {
        {
            as_str_slice_test_case!(
                input,
                "here is some plaintext *but what if we italicize?"
            );
            let result = parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt(
                input,
                CheckboxParsePolicy::IgnoreCheckbox,
            );

            let (remaining, fragments) = result.unwrap();
            assert_eq2!(remaining.is_empty(), true);
            assert_eq2!(
                fragments,
                list![
                    MdLineFragment::Plain("here is some plaintext "),
                    MdLineFragment::Plain("*"),
                    MdLineFragment::Plain("but what if we italicize?"),
                ]
            );
        }
    }
}
