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

use nom::{branch::alt, combinator::map, multi::many0, IResult, Parser};

use crate::{constants::{AUTHORS, DATE, TAGS, TITLE},
            parse_block_code_alt,
            parse_block_smart_list_alt,
            parse_csv_opt_eol_alt,
            parse_heading_until_eol_or_eoi_alt,
            parse_markdown_text_including_eol_or_eoi_alt,
            parse_unique_kv_opt_eol_alt,
            sizing_list_of::ListStorage,
            AsStrSlice,
            List,
            MdDocument,
            MdElement};

/// Parse a markdown document.
///
/// This function uses `many0(alt(...))` to parse the input, where `alt` tries each parser
/// in order until one succeeds. If none of the parsers succeed, `alt` fails. However,
/// `many0` will continue applying the parser until it fails, at which point it returns
/// the accumulated results and the remaining input.
///
/// To ensure that the parser consumes all of the input, leaving an empty remainder, we
/// add a fallback parser (`parse_any_line_as_text_alt`) as the last parser in the `alt`
/// combinator. This fallback parser can parse any line as text, ensuring that the parser
/// consumes all of the input.
///
/// We also add a check at the end of the function to ensure that the remainder is empty,
/// just in case the fallback parser doesn't handle all the input.
pub fn parse_markdown_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdDocument<'a>> {
    // Use many0 to apply the parser repeatedly.
    let (remainder, output) = many0(
        // NOTE: The ordering of the parsers below matters.
        alt((
            map(parse_title_value, |maybe_title| match maybe_title {
                None => MdElement::Title(""),
                Some(title) => MdElement::Title(title.extract_to_line_end()),
            }),
            map(parse_tags_list, |list| {
                let acc: ListStorage<&str> =
                    list.iter().map(|item| item.extract_to_line_end()).collect();
                MdElement::Tags(List::from(acc))
            }),
            map(parse_authors_list, |list| {
                let acc: ListStorage<&str> =
                    list.iter().map(|item| item.extract_to_line_end()).collect();
                MdElement::Authors(List::from(acc))
            }),
            map(parse_date_value, |maybe_date| match maybe_date {
                None => MdElement::Date(""),
                Some(date) => MdElement::Date(date.extract_to_line_end()),
            }),
            map(parse_heading_until_eol_or_eoi_alt, MdElement::Heading),
            map(parse_block_smart_list_alt, MdElement::SmartList),
            map(parse_block_code_alt, MdElement::CodeBlock),
            map(
                parse_markdown_text_including_eol_or_eoi_alt,
                MdElement::Text,
            ),
        )),
    )
    .parse(input)?;
    let it = List::from(output);
    Ok((remainder, it))
}

// key: TAGS, value: CSV parser.
fn parse_tags_list<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, List<AsStrSlice<'a>>> {
    parse_csv_opt_eol_alt(TAGS, input)
}

// key: AUTHORS, value: CSV parser.
fn parse_authors_list<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, List<AsStrSlice<'a>>> {
    parse_csv_opt_eol_alt(AUTHORS, input)
}

// key: TITLE, value: KV parser.
fn parse_title_value<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    parse_unique_kv_opt_eol_alt(TITLE, input)
}

// key: DATE, value: KV parser.
fn parse_date_value<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, Option<AsStrSlice<'a>>> {
    parse_unique_kv_opt_eol_alt(DATE, input)
}

/// Tests things that are final output (and not at the IR level).
#[cfg(test)]
mod tests_integration_block_smart_lists {
    use crate::{assert_eq2, parse_markdown_alt, AsStrSlice, GCString, PrettyPrintDebug};

    #[test]
    fn test_parse_valid_md_ol_with_indent() {
        let raw_input =
            "start\n1. ol1\n  2. ol2\n     ol2.1\n    3. ol3\n       ol3.1\n       ol3.2\nend\n";
        let binding = raw_input
            .lines()
            .map(GCString::from)
            .collect::<Vec<GCString>>();
        let input = AsStrSlice::from(binding.as_slice());

        let expected_output = [
            "start",
            "[  ┊1.│ol1┊  ]",
            "[  ┊  2.│ol2┊ → ┊    │ol2.1┊  ]",
            "[  ┊    3.│ol3┊ → ┊      │ol3.1┊ → ┊      │ol3.2┊  ]",
            "end",
        ];

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }

    #[test]
    fn test_parse_valid_md_ul_with_indent() {
        let raw_input =
            "start\n- ul1\n  - ul2\n    ul2.1\n    - ul3\n      ul3.1\n      ul3.2\nend\n";
        let binding = raw_input
            .lines()
            .map(GCString::from)
            .collect::<Vec<GCString>>();
        let input = AsStrSlice::from(binding.as_slice());

        let expected_output = [
            "start",
            "[  ┊─┤ul1┊  ]",
            "[  ┊───┤ul2┊ → ┊   │ul2.1┊  ]",
            "[  ┊─────┤ul3┊ → ┊     │ul3.1┊ → ┊     │ul3.2┊  ]",
            "end",
        ];

        let result = parse_markdown_alt(input);
        let (remainder, md_doc) = result.unwrap();

        md_doc.inner.iter().zip(expected_output.iter()).for_each(
            |(element, test_str)| {
                let lhs = element.pretty_print_debug();
                let rhs = test_str.to_string();
                assert_eq2!(lhs, rhs);
            },
        );

        dbg!(&remainder);
        assert_eq2!(remainder.is_empty(), true);
    }
}
