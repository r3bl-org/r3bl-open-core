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

pub fn parse_markdown_alt<'a>(
    input: AsStrSlice<'a>,
) -> IResult<AsStrSlice<'a>, MdDocument<'a>> {
    let (input, output) = many0(
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
    Ok((input, it))
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
