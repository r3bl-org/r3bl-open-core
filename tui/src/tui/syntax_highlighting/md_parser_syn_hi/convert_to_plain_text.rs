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

//! This module is responsible for converting all the [MdLineFragment] into plain text w/out any
//! formatting.

use crate::{constants::*, *};

/// Marker trait to "remember" which types can be converted to plain text.
pub trait ConvertToPlainText {
    fn to_plain_text(&self) -> US;
}

impl ConvertToPlainText for MdLineFragments<'_> {
    fn to_plain_text(&self) -> US {
        let mut it: String = String::new();
        for fragment in self {
            it.push_str(&fragment.to_plain_text().string);
        }
        US::from(it)
    }
}

impl ConvertToPlainText for HeadingLevel {
    fn to_plain_text(&self) -> US {
        let it: String = format!(
            "{}{}",
            HEADING_CHAR.to_string().repeat(usize::from(*self)),
            SPACE
        );
        US::from(it)
    }
}

impl ConvertToPlainText for MdLineFragment<'_> {
    fn to_plain_text(&self) -> US {
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
            MdLineFragment::Bold(text) => format!("{BOLD_1}{text}{BOLD_1}"),
            MdLineFragment::Italic(text) => format!("{ITALIC_1}{text}{ITALIC_1}"),
            MdLineFragment::BoldItalic(text) => format!("{BITALIC_1}{text}{BITALIC_1}"),
            MdLineFragment::InlineCode(text) => format!("{BACK_TICK}{text}{BACK_TICK}"),
            MdLineFragment::Checkbox(is_checked) => {
                (if *is_checked { CHECKED } else { UNCHECKED }).to_string()
            }
            MdLineFragment::OrderedListItemNumber(number) => format!("{number}{PERIOD}{SPACE}"),
            MdLineFragment::UnorderedListItem => format!("{UNORDERED_LIST}{PERIOD}{SPACE}"),
        };
        US::from(it)
    }
}

#[cfg(test)]
mod to_plain_text_tests {
    use r3bl_rs_utils_core::*;

    use super::*;

    #[test]
    fn test_fragment_to_plain_text() {
        assert_eq2!(
            MdLineFragment::Plain(" Hello World ")
                .to_plain_text()
                .string,
            " Hello World "
        );
        assert_eq2!(
            MdLineFragment::Link(HyperlinkData::new("r3bl.com", "https://r3bl.com"))
                .to_plain_text()
                .string,
            "[r3bl.com](https://r3bl.com)"
        );
        assert_eq2!(
            MdLineFragment::Image(HyperlinkData::new("some image text", "https://r3bl.com"))
                .to_plain_text()
                .string,
            "![some image text](https://r3bl.com)"
        );
        assert_eq2!(
            MdLineFragment::Bold("Hello World").to_plain_text().string,
            "**Hello World**"
        );
        assert_eq2!(
            MdLineFragment::Italic("Hello World").to_plain_text().string,
            "*Hello World*"
        );
        assert_eq2!(
            MdLineFragment::BoldItalic("Hello World")
                .to_plain_text()
                .string,
            "***Hello World***"
        );
        assert_eq2!(
            MdLineFragment::InlineCode("Hello World")
                .to_plain_text()
                .string,
            "`Hello World`"
        );
        assert_eq2!(MdLineFragment::Checkbox(true).to_plain_text().string, "[x]");
        assert_eq2!(
            MdLineFragment::Checkbox(false).to_plain_text().string,
            "[ ]"
        );
    }

    #[test]
    fn test_level_to_plain_text() {
        assert_eq2!(HeadingLevel::Heading1.to_plain_text().string, "# ");
        assert_eq2!(HeadingLevel::Heading2.to_plain_text().string, "## ");
        assert_eq2!(HeadingLevel::Heading3.to_plain_text().string, "### ");
        assert_eq2!(HeadingLevel::Heading4.to_plain_text().string, "#### ");
        assert_eq2!(HeadingLevel::Heading5.to_plain_text().string, "##### ");
        assert_eq2!(HeadingLevel::Heading6.to_plain_text().string, "###### ");
    }
}
