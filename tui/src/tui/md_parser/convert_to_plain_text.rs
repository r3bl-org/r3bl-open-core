/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

//! This module is responsible for converting all the [`MdLineFragment`] into plain text
//! w/out any formatting.

use std::fmt::Write as _;

use crate::{convert_to_string_slice, inline_string, join, join_fmt, join_with_index,
            md_parser::constants::{BACK_TICK, CHECKED, HEADING_CHAR, LEFT_BRACKET,
                                   LEFT_IMAGE, LEFT_PARENTHESIS, LIST_SPACE_DISPLAY,
                                   LIST_SPACE_END_DISPLAY_FIRST_LINE,
                                   LIST_SPACE_END_DISPLAY_REST_LINE, NEW_LINE, PERIOD,
                                   RIGHT_BRACKET, RIGHT_IMAGE, RIGHT_PARENTHESIS,
                                   SPACE, STAR, UNCHECKED, UNDERSCORE},
            pad_fmt, usize_to_u8_array, HeadingLevel, HyperlinkData, InlineString, List,
            MdDocument, MdElement, MdLineFragment, PrettyPrintDebug};

impl PrettyPrintDebug for MdDocument<'_> {
    fn pretty_print_debug(&self) -> InlineString {
        join_with_index!(
            from: self,
            each: block,
            index: index,
            delim: NEW_LINE,
            format: "[{a}]: {b}", a = index, b = block.pretty_print_debug()
        )
    }
}

impl PrettyPrintDebug for List<MdLineFragment<'_>> {
    fn pretty_print_debug(&self) -> InlineString {
        join!(
            from: self,
            each: fragment,
            delim: "",
            format: "{a}", a = fragment.pretty_print_debug()
        )
    }
}

impl PrettyPrintDebug for MdElement<'_> {
    fn pretty_print_debug(&self) -> InlineString {
        match self {
            MdElement::Heading(heading_data) => {
                inline_string!(
                    "{}{}",
                    heading_data.level.pretty_print_debug(),
                    heading_data.text,
                )
            }
            MdElement::Text(fragments) => fragments.pretty_print_debug(),
            MdElement::CodeBlock(list_codeblock_line) => {
                let line_count = list_codeblock_line.len();
                let lang = {
                    list_codeblock_line
                        .first()
                        .and_then(|first_line| first_line.language)
                        .unwrap_or("n/a")
                };
                inline_string!("code block, line count: {line_count}, lang: {lang}")
            }
            MdElement::Title(title) => inline_string!("title: {}", title),
            MdElement::Tags(tags) => {
                join!(
                    from: tags,
                    each: tag,
                    delim: ", ",
                    format: "{a}", a = tag
                )
            }
            MdElement::Date(date) => inline_string!("title: {}", date),
            MdElement::Authors(authors) => {
                join!(
                    from: authors,
                    each: author,
                    delim: ", ",
                    format: "{a}", a = author
                )
            }
            MdElement::SmartList((list_lines, _bullet_kind, _indent)) => {
                let mut acc = InlineString::new();
                // We don't care about the result of this operation.
                write!(acc, "[  ").ok();
                join_fmt!(
                    fmt: acc,
                    from: list_lines,
                    each: fragments_in_one_line,
                    delim: " → ",
                    format: "┊{}┊", fragments_in_one_line.pretty_print_debug()
                );
                // We don't care about the result of this operation.
                write!(acc, "  ]").ok();
                acc
            }
        }
    }
}

impl PrettyPrintDebug for HeadingLevel {
    fn pretty_print_debug(&self) -> InlineString {
        let mut acc = InlineString::new();
        let num_of_hashes = self.level;
        pad_fmt!(
            fmt: acc,
            pad_str: HEADING_CHAR,
            repeat_count: num_of_hashes
        );
        // We don't care about the result of this operation.
        write!(acc, "{SPACE}").ok();
        acc
    }
}

impl PrettyPrintDebug for MdLineFragment<'_> {
    fn pretty_print_debug(&self) -> InlineString {
        match self {
            MdLineFragment::Plain(text) => (*text).into(),
            MdLineFragment::Link(HyperlinkData { text, url }) => {
                inline_string!(
                    "{LEFT_BRACKET}{text}{RIGHT_BRACKET}{LEFT_PARENTHESIS}{url}{RIGHT_PARENTHESIS}"
                )
            }
            MdLineFragment::Image(HyperlinkData {
                text: alt_text,
                url,
            }) => {
                inline_string!(
                    "{LEFT_IMAGE}{alt_text}{RIGHT_IMAGE}{LEFT_PARENTHESIS}{url}{RIGHT_PARENTHESIS}"
                )
            }
            MdLineFragment::Bold(text) => inline_string!("{STAR}{text}{STAR}"),
            MdLineFragment::Italic(text) => {
                inline_string!("{UNDERSCORE}{text}{UNDERSCORE}")
            }
            MdLineFragment::InlineCode(text) => {
                inline_string!("{BACK_TICK}{text}{BACK_TICK}")
            }
            MdLineFragment::Checkbox(is_checked) => {
                (if *is_checked { CHECKED } else { UNCHECKED }).into()
            }
            MdLineFragment::OrderedListBullet {
                indent,
                number,
                is_first_line,
            } => generate_ordered_list_item_bullet(indent, number, is_first_line),
            MdLineFragment::UnorderedListBullet {
                indent,
                is_first_line,
            } => generate_unordered_list_item_bullet(indent, is_first_line),
        }
    }
}

#[must_use]
pub fn generate_ordered_list_item_bullet(
    indent: &usize,
    number: &usize,
    is_first_line: &bool,
) -> InlineString {
    let mut acc = InlineString::new();

    if *is_first_line {
        pad_fmt!(
            fmt: acc,
            pad_str: SPACE,
            repeat_count: *indent
        );
        // We don't care about the result of this operation.
        write!(acc, "{number}{PERIOD}{LIST_SPACE_END_DISPLAY_REST_LINE}").ok();
    } else {
        // Padding for indent.
        pad_fmt!(
            fmt: acc,
            pad_str: SPACE,
            repeat_count: *indent
        );

        // Padding for number.
        let number_ray = usize_to_u8_array(*number);
        let number_str = convert_to_string_slice(&number_ray);
        let number_str_len = number_str.len();
        pad_fmt!(
            fmt: acc,
            pad_str: SPACE,
            repeat_count: number_str_len
        );

        // Write the reset rest of the line.
        // We don't care about the result of this operation.
        write!(acc, "{SPACE}{LIST_SPACE_END_DISPLAY_REST_LINE}").ok();
    }

    acc
}

#[must_use]
pub fn generate_unordered_list_item_bullet(
    indent: &usize,
    is_first_line: &bool,
) -> InlineString {
    let mut acc = InlineString::new();

    if *is_first_line {
        pad_fmt!(
            fmt: acc,
            pad_str: LIST_SPACE_DISPLAY,
            repeat_count: *indent
        );

        // We don't care about the result of this operation.
        write!(
            acc,
            "{LIST_SPACE_DISPLAY}{LIST_SPACE_END_DISPLAY_FIRST_LINE}",
        ).ok();
    } else {
        pad_fmt!(
            fmt: acc,
            pad_str: SPACE,
            repeat_count: *indent
        );
        // We don't care about the result of this operation.
        write!(acc, "{SPACE}{LIST_SPACE_END_DISPLAY_REST_LINE}").ok();
    }

    acc
}

#[cfg(test)]
mod to_plain_text_tests {
    use super::*;
    use crate::assert_eq2;

    #[test]
    fn test_fragment_to_plain_text() {
        assert_eq2!(
            MdLineFragment::Plain(" Hello World ").pretty_print_debug(),
            " Hello World "
        );
        assert_eq2!(
            MdLineFragment::Link(HyperlinkData::new("r3bl.com", "https://r3bl.com"))
                .pretty_print_debug(),
            "[r3bl.com](https://r3bl.com)"
        );
        assert_eq2!(
            MdLineFragment::Image(HyperlinkData::new(
                "some image text",
                "https://r3bl.com"
            ))
            .pretty_print_debug(),
            "![some image text](https://r3bl.com)"
        );
        assert_eq2!(
            MdLineFragment::Bold("Hello World").pretty_print_debug(),
            "*Hello World*"
        );
        assert_eq2!(
            MdLineFragment::Italic("Hello World").pretty_print_debug(),
            "_Hello World_"
        );
        assert_eq2!(
            MdLineFragment::InlineCode("Hello World").pretty_print_debug(),
            "`Hello World`"
        );
        assert_eq2!(MdLineFragment::Checkbox(true).pretty_print_debug(), "[x]");
        assert_eq2!(MdLineFragment::Checkbox(false).pretty_print_debug(), "[ ]");
    }

    #[test]
    fn test_level_to_plain_text() {
        assert_eq2!(HeadingLevel { level: 1 }.pretty_print_debug(), "# ");
        assert_eq2!(HeadingLevel { level: 2 }.pretty_print_debug(), "## ");
        assert_eq2!(HeadingLevel { level: 3 }.pretty_print_debug(), "### ");
        assert_eq2!(HeadingLevel { level: 4 }.pretty_print_debug(), "#### ");
        assert_eq2!(HeadingLevel { level: 5 }.pretty_print_debug(), "##### ");
        assert_eq2!(HeadingLevel { level: 6 }.pretty_print_debug(), "###### ");
        assert_eq2!(HeadingLevel { level: 7 }.pretty_print_debug(), "####### ");
    }
}
