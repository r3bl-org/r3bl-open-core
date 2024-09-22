/*
 *   Copyright (c) 2023 R3BL LLC
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

//! This module is responsible for converting all the [MdLineFragment] into plain text
//! w/out any formatting.

use r3bl_rs_utils_core::PrettyPrintDebug;

use super::*;
use crate::{constants::*, *};

impl<'a> PrettyPrintDebug for MdDocument<'a> {
    fn pretty_print_debug(&self) -> String {
        let mut it = vec![];
        for (index, block) in self.iter().enumerate() {
            it.push(format!("[{}]: {}", index, block.pretty_print_debug()));
        }
        it.join("\n")
    }
}

impl<'a> PrettyPrintDebug for List<MdLineFragment<'a>> {
    fn pretty_print_debug(&self) -> String {
        self.inner
            .iter()
            .map(|fragment| fragment.pretty_print_debug())
            .collect::<Vec<String>>()
            .join("")
    }
}

impl<'a> PrettyPrintDebug for MdBlock<'a> {
    fn pretty_print_debug(&self) -> String {
        match self {
            MdBlock::Heading(heading_data) => {
                format!(
                    "{}{}",
                    heading_data.heading_level.pretty_print_debug(),
                    heading_data.text,
                )
            }
            MdBlock::Text(fragments) => fragments.pretty_print_debug(),
            MdBlock::CodeBlock(list_codeblock_line) => {
                let line_count = list_codeblock_line.len();
                let lang = {
                    list_codeblock_line
                        .first()
                        .and_then(|first_line| first_line.language)
                        .unwrap_or("n/a")
                };
                format!("code block, line count: {line_count}, lang: {lang}")
            }
            MdBlock::Title(title) => format!("title: {}", title),
            MdBlock::Tags(tags) => format!("tags: {}", tags.join(", ")),
            MdBlock::Date(date) => format!("title: {}", date),
            MdBlock::Authors(authors) => format!("tags: {}", authors.join(", ")),
            MdBlock::SmartList((list_lines, _bullet_kind, _indent)) => format!(
                "[  {}  ]",
                list_lines
                    .iter()
                    .map(|fragments_in_one_line| format!(
                        "┊{}┊",
                        fragments_in_one_line.pretty_print_debug()
                    ))
                    .collect::<Vec<String>>()
                    .join(" → ")
            ),
        }
    }
}

impl PrettyPrintDebug for HeadingLevel {
    fn pretty_print_debug(&self) -> String {
        let num_of_hashes = usize::from(*self);
        let it: String = format!(
            "{}{}",
            HEADING_CHAR.to_string().repeat(num_of_hashes),
            SPACE
        );
        it
    }
}

impl PrettyPrintDebug for MdLineFragment<'_> {
    fn pretty_print_debug(&self) -> String {
        let it: String = match self {
            MdLineFragment::Plain(text) => text.to_string(),
            MdLineFragment::Link(HyperlinkData { text, url }) => {
                format!(
                    "{LEFT_BRACKET}{text}{RIGHT_BRACKET}{LEFT_PARENTHESIS}{url}{RIGHT_PARENTHESIS}"
                )
            }
            MdLineFragment::Image(HyperlinkData {
                text: alt_text,
                url,
            }) => {
                format!(
                    "{LEFT_IMAGE}{alt_text}{RIGHT_IMAGE}{LEFT_PARENTHESIS}{url}{RIGHT_PARENTHESIS}"
                )
            }
            MdLineFragment::Bold(text) => format!("{STAR}{text}{STAR}"),
            MdLineFragment::Italic(text) => format!("{UNDERSCORE}{text}{UNDERSCORE}"),
            MdLineFragment::InlineCode(text) => format!("{BACK_TICK}{text}{BACK_TICK}"),
            MdLineFragment::Checkbox(is_checked) => {
                (if *is_checked { CHECKED } else { UNCHECKED }).to_string()
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
        };
        it
    }
}

pub fn generate_ordered_list_item_bullet(
    indent: &usize,
    number: &usize,
    is_first_line: &bool,
) -> String {
    if *is_first_line {
        let padding_for_indent = SPACE.repeat(*indent);
        let first_line_bullet =
            format!("{number}{PERIOD}{LIST_SPACE_END_DISPLAY_REST_LINE}");
        format!("{padding_for_indent}{first_line_bullet}")
    } else {
        let padding_for_indent = SPACE.repeat(*indent);
        let number_str = format!("{}", number);
        let number_str_len = number_str.len();
        let number_str_blanks = SPACE.repeat(number_str_len);
        let rest_line_bullet =
            format!("{number_str_blanks}{SPACE}{LIST_SPACE_END_DISPLAY_REST_LINE}");
        format!("{padding_for_indent}{rest_line_bullet}")
    }
}

pub fn generate_unordered_list_item_bullet(
    indent: &usize,
    is_first_line: &bool,
) -> String {
    if *is_first_line {
        let padding_for_indent = LIST_SPACE_DISPLAY.repeat(*indent);
        let first_line_bullet = format!(
            "{}{}",
            LIST_SPACE_DISPLAY, LIST_SPACE_END_DISPLAY_FIRST_LINE
        );
        format!("{padding_for_indent}{first_line_bullet}")
    } else {
        let padding_for_indent = SPACE.repeat(*indent);
        let rest_line_bullet = format!("{}{}", SPACE, LIST_SPACE_END_DISPLAY_REST_LINE);
        format!("{padding_for_indent}{rest_line_bullet}")
    }
}

#[cfg(test)]
mod to_plain_text_tests {
    use r3bl_rs_utils_core::*;

    use super::*;

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
