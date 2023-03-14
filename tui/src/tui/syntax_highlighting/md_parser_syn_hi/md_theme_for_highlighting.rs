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

//! This module is responsible for converting a [Document] into a [StyleUSFragmentLines].

use r3bl_rs_utils_core::*;

use crate::*;

impl StyleUSFragmentLines {
    pub fn from_document(
        document: &Document,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut lines = StyleUSFragmentLines::default();
        for block in document.iter() {
            let block_to_lines =
                StyleUSFragmentLines::from_block(block, maybe_current_box_computed_style);
            lines.items.extend(block_to_lines.items);
        }
        lines
    }
}

impl StyleUSFragmentLines {
    /// Each [Block] needs to be translated into a line. The [Block::CodeBlock] is the only
    /// block that needs to be translated into multiple lines. This is why the return type is a
    /// [StyleUSFragmentLines] (and not a single line).
    fn from_block(block: &Block, maybe_current_box_computed_style: &Option<Style>) -> Self {
        let mut lines = StyleUSFragmentLines::default();

        // AI: 1. convert each block into a line (or lines) using the md_theme below

        match block {
            Block::Heading(heading_data) => {
                lines.push(StyleUSFragmentLine::from_heading_data(
                    heading_data,
                    maybe_current_box_computed_style,
                ));
            }
            Block::OrderedList(_) => todo!(),   // AI:
            Block::UnorderedList(_) => todo!(), // AI:
            Block::Text(_) => todo!(),          // AI:
            Block::CodeBlock(_) => todo!(),     // AI:
            Block::Title(_) => todo!(),         // AI:
            Block::Tags(_) => todo!(),          // AI:
        }

        lines
    }
}

impl StyleUSFragmentLine {
    /// This is a sample [HeadingData] that needs to be converted into a [StyleUSFragmentLine].
    ///
    /// ```text
    /// #░heading░*foo*░**bar**
    /// ░░▓▓▓▓▓▓▓▓░░░░░▓░░░░░░░
    /// |    |      |  |   |
    /// |    |      |  |   + Fragment::Bold("bar")
    /// |    |      |  + Fragment::Plain("░")
    /// |    |      + Fragment::Italic("foo")
    /// |    + Fragment::Plain("heading░")
    /// + Level::Heading1
    /// ```
    fn from_heading_data(
        heading_data: &HeadingData,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut color_wheel = ColorWheel::from_heading_data(heading_data);
        let mut line = StyleUSFragmentLine::default();

        let heading_level_style_us_fragment: StyleUSFragment = {
            let heading_level = heading_data.level.to_plain_text();
            let my_style = {
                let mut it = match maybe_current_box_computed_style {
                    Some(style) => *style,
                    None => Style::default(),
                };
                it.dim = true;
                it
            };
            (my_style, heading_level)
        };

        let heading_text_style_us_fragment: StyleUSFragmentLine = {
            let heading_text = heading_data.content.to_plain_text();
            let styled_texts = color_wheel.colorize_into_styled_texts(
                &heading_text,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(*maybe_current_box_computed_style),
            );
            StyleUSFragmentLine::from(styled_texts)
        };

        line.items.push(heading_level_style_us_fragment);
        line.items.extend(heading_text_style_us_fragment.items);

        line
    }
}

impl From<StyledTexts> for StyleUSFragmentLine {
    fn from(styled_texts: StyledTexts) -> Self {
        let mut it = StyleUSFragmentLine::default();
        // More info on `into_iter`: <https://users.rust-lang.org/t/move-value-from-an-iterator/46172>
        for styled_text in styled_texts.items.into_iter() {
            let style = styled_text.style;
            let us = styled_text.plain_text;
            it.items.push((style, us));
        }
        it
    }
}

#[cfg(test)]
mod test_generate_style_us_fragment_lines_from_document {
    use super::*;

    fn generate_doc<'a>() -> Document<'a> {
        vec![
            Block::Title("Something"),
            Block::Tags(vec!["tag1", "tag2", "tag3"]),
            Block::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                content: vec![Fragment::Plain("Foobar")],
            }),
            Block::Text(vec![]), // Empty line.
            Block::Text(vec![Fragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            Block::Text(vec![]), // Empty line.
            Block::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            Block::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            Block::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            Block::Heading(HeadingData {
                level: HeadingLevel::Heading2,
                content: vec![Fragment::Plain("Installation")],
            }),
            Block::Text(vec![]), // Empty line.
            Block::Text(vec![
                Fragment::Plain("Use the package manager "),
                Fragment::Link(("pip", "https://pip.pypa.io/en/stable/")),
                Fragment::Plain(" to install foobar."),
            ]),
            Block::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
            Block::UnorderedList(vec![
                vec![Fragment::Plain("ul1")],
                vec![Fragment::Plain("ul2")],
            ]),
            Block::OrderedList(vec![
                vec![Fragment::Plain("ol1")],
                vec![Fragment::Plain("ol2")],
            ]),
            Block::UnorderedList(vec![
                vec![Fragment::Checkbox(false), Fragment::Plain(" todo")],
                vec![Fragment::Checkbox(true), Fragment::Plain(" done")],
            ]),
            Block::Text(vec![Fragment::Plain("end")]),
        ]
    }

    #[test]
    fn test_generate_style_us_fragment_lines_from_document() {
        // AI: write tests
        let document: Document = generate_doc();
    }
}
