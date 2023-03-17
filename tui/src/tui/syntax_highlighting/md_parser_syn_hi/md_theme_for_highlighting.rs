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

//! This module is responsible for converting a [MdDocument] into a [StyleUSSpanLines].

use r3bl_rs_utils_core::*;
use r3bl_rs_utils_macro::style;

use crate::*;

impl StyleUSSpanLines {
    pub fn pretty_print(&self) -> String {
        let mut it = vec![];

        for line in &self.items {
            for fragment in &line.items {
                let StyleUSSpan(style, us) = fragment;
                let line_text = format!("fragment[ {:?} , {:?} ]", us.string, style);
                it.push(line_text);
            }
        }

        it.join("\n")
    }

    pub fn from_document(
        document: &MdDocument,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();
        for block in document.iter() {
            let block_to_lines =
                StyleUSSpanLines::from_block(block, maybe_current_box_computed_style);
            lines.items.extend(block_to_lines.items);
        }
        lines
    }

    /// Each [MdBlockElement] needs to be translated into a line. The [MdBlockElement::CodeBlock] is
    /// the only block that needs to be translated into multiple lines. This is why the return type
    /// is a [StyleUSSpanLines] (and not a single line).
    pub fn from_block(
        block: &MdBlockElement,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();

        match block {
            MdBlockElement::Heading(heading_data) => {
                lines.push(StyleUSSpanLine::from_heading_data(
                    heading_data,
                    maybe_current_box_computed_style,
                ));
            }
            MdBlockElement::Text(fragments_in_one_line) => {
                lines.push(StyleUSSpanLine::from_fragments(
                    fragments_in_one_line,
                    maybe_current_box_computed_style,
                ))
            }
            MdBlockElement::UnorderedList(_) => todo!(), // AI: 0.2. from_block(): ul -> StyleUSSpanLines
            MdBlockElement::OrderedList(_) => todo!(), // AI: 0.2. from_block(): ol -> StyleUSSpanLines
            MdBlockElement::CodeBlock(_) => todo!(), // AI: 0.2. from_block(): cb -> StyleUSSpanLines
            MdBlockElement::Title(_) => todo!(),     // AI: 0.2. from_block(): md -> StyleUSSpanLine
            MdBlockElement::Tags(_) => todo!(),      // AI: 0.2. from_block(): md -> StyleUSSpanLine
        }

        lines
    }
}

impl StyleUSSpan {
    /// Each [MdLineFragment] needs to be translated into a [StyleUSSpan]. These are then rolled up
    /// into using [StyleUSSpanLine::from_block](StyleUSSpanLine::from_block).
    pub fn from_fragment(
        fragment: &MdLineFragment,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        match fragment {
            MdLineFragment::Plain(plain_text) => StyleUSSpan(
                maybe_current_box_computed_style.unwrap_or_default(),
                US::from(*plain_text),
            ),
            MdLineFragment::Bold(bold_text) => StyleUSSpan(
                maybe_current_box_computed_style.unwrap_or_default()
                    + style! {
                        attrib: [bold]
                    },
                US::from(*bold_text),
            ),
            MdLineFragment::Italic(italic_text) => StyleUSSpan(
                maybe_current_box_computed_style.unwrap_or_default()
                    + style! {
                        attrib: [italic]
                    },
                US::from(*italic_text),
            ),
            MdLineFragment::BoldItalic(_) => todo!(), // AI: 0.1. from_fragment(): bold-italic
            MdLineFragment::InlineCode(_) => todo!(), // AI: 0.1. from_fragment(): inline-code
            MdLineFragment::Link(_) => todo!(),       // AI: 0.1. from_fragment(): Link
            MdLineFragment::Image(_) => todo!(),      // AI: 0.1. from_fragment(): Image
            MdLineFragment::Checkbox(_) => todo!(),   // AI: 0.1. from_fragment(): Checkbox
        }
    }
}

impl StyleUSSpanLine {
    fn from_fragments(
        fragments_in_one_line: &FragmentsInOneLine,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        fragments_in_one_line
            .iter()
            .map(|fragment| StyleUSSpan::from_fragment(fragment, maybe_current_box_computed_style))
            .collect::<Vec<_>>()
            .into()
    }

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
        let mut line = StyleUSSpanLine::default();

        let heading_level_style_us_fragment: StyleUSSpan = {
            let heading_level = heading_data.level.to_plain_text();
            let my_style = {
                maybe_current_box_computed_style.unwrap_or_default()
                    + style! {
                        attrib: [dim]
                    }
            };
            StyleUSSpan(my_style, heading_level)
        };

        let heading_text_style_us_fragment: StyleUSSpanLine = {
            let heading_text = heading_data.content.to_plain_text();
            let styled_texts = color_wheel.colorize_into_styled_texts(
                &heading_text,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(*maybe_current_box_computed_style),
            );
            StyleUSSpanLine::from(styled_texts)
        };

        line.items.push(heading_level_style_us_fragment);
        line.items.extend(heading_text_style_us_fragment.items);

        line
    }
}

impl From<StyledTexts> for StyleUSSpanLine {
    fn from(styled_texts: StyledTexts) -> Self {
        let mut it = StyleUSSpanLine::default();
        // More info on `into_iter`: <https://users.rust-lang.org/t/move-value-from-an-iterator/46172>
        for styled_text in styled_texts.items.into_iter() {
            let style = styled_text.get_style();
            let us = styled_text.get_text();
            it.items.push(StyleUSSpan(*style, us.clone()));
        }
        it
    }
}

#[cfg(test)]
mod test_generate_style_us_span_lines {
    use r3bl_rs_utils_macro::style;

    use super::*;

    /// Test each [MdLineFragment] variant is converted by
    /// [StyleUSSpan::from_fragment](StyleUSSpan::from_fragment).
    mod from_fragment {
        use super::*;

        // AI: 0.1. TEST from_fragment [ ] BoldItalic(&'a str),
        // AI: 0.1. TEST from_fragment [ ] InlineCode(&'a str),
        // AI: 0.1. TEST from_fragment [ ] Link((&'a str, &'a str)),
        // AI: 0.1. TEST from_fragment [ ] Image((&'a str, &'a str)),
        // AI: 0.1. TEST from_fragment [ ] Checkbox(bool),

        #[test]
        fn plain() {
            let fragment = MdLineFragment::Plain("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            assert_eq2!(
                StyleUSSpan::from_fragment(&fragment, &maybe_style),
                StyleUSSpan(maybe_style.unwrap_or_default(), US::from("Foobar"))
            );
        }

        #[test]
        fn bold() {
            let fragment = MdLineFragment::Bold("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            assert_eq2!(
                StyleUSSpan::from_fragment(&fragment, &maybe_style),
                StyleUSSpan(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [bold]
                        },
                    US::from("Foobar")
                )
            );
        }

        #[test]
        fn italic() {
            let fragment = MdLineFragment::Italic("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            assert_eq2!(
                StyleUSSpan::from_fragment(&fragment, &maybe_style),
                StyleUSSpan(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [italic]
                        },
                    US::from("Foobar")
                )
            );
        }
    }

    /// Test each variant of [MdBlockElement] is converted by
    /// [StyleUSSpanLines::from_block](StyleUSSpanLines::from_block).
    mod from_block {
        use super::*;

        // AI: 0.2. TEST from_block [ ] OrderedList(Lines<'a>),
        // AI: 0.2. TEST from_block [ ] UnorderedList(Lines<'a>),
        // AI: 0.2. TEST from_block [ ] CodeBlock(Vec<CodeBlockLine<'a>>),
        // AI: 0.2. TEST from_block [ ] Title(&'a str),
        // AI: 0.2. TEST from_block [ ] Tags(Vec<&'a str>),

        #[test]
        fn text() {
            let text_block = MdBlockElement::Text(vec![MdLineFragment::Plain("Foobar")]);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines = StyleUSSpanLines::from_block(&text_block, &maybe_style);
            // println!("{}", lines.pretty_print());

            let line_0 = &lines.items[0];
            let fragment_0_in_line_0 = &line_0.items[0];
            let StyleUSSpan(style, text) = fragment_0_in_line_0;
            assert_eq2!(text.string, "Foobar");
            assert_eq2!(style, &maybe_style.unwrap());
        }

        #[test]
        fn heading() {
            let heading_block = MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                content: vec![MdLineFragment::Plain("Foobar")],
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines = StyleUSSpanLines::from_block(&heading_block, &maybe_style);
            // println!("{}", lines.pretty_print());

            // There should just be 1 line.
            assert_eq2!(lines.items.len(), 1);
            let first_line = &lines.items[0];
            let fragments_in_line = &first_line.items;

            // There should be 7 fragments in the line.
            assert_eq2!(fragments_in_line.len(), 7);

            // First fragment is the heading level `# ` in dim w/ Red bg color, and no fg color.
            assert_eq2!(fragments_in_line[0].0.dim, true);
            assert_eq2!(
                fragments_in_line[0].0.color_bg.unwrap(),
                TuiColor::Basic(ANSIBasicColor::Red)
            );
            assert_eq2!(fragments_in_line[0].0.color_fg.is_some(), false);
            assert_eq2!(fragments_in_line[0].1.string, "# ");

            // The remainder of the fragments are the heading text which are colorized with a color
            // wheel.
            for fragment in &fragments_in_line[1..=6] {
                assert_eq2!(fragment.0.dim, false);
                assert_eq2!(
                    fragment.0.color_bg.unwrap(),
                    TuiColor::Basic(ANSIBasicColor::Red)
                );
                assert_eq2!(fragment.0.color_fg.is_some(), true);
            }
        }
    }

    fn generate_doc<'a>() -> MdDocument<'a> {
        vec![
            MdBlockElement::Title("Something"),
            MdBlockElement::Tags(vec!["tag1", "tag2", "tag3"]),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                content: vec![MdLineFragment::Plain("Foobar")],
            }),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::Text(vec![MdLineFragment::Plain(
                "Foobar is a Python library for dealing with word pluralization.",
            )]),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("bash"),
                vec!["pip install foobar"],
            )),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("fish"), vec![])),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(Some("python"), vec![""])),
            MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading2,
                content: vec![MdLineFragment::Plain("Installation")],
            }),
            MdBlockElement::Text(vec![]), // Empty line.
            MdBlockElement::Text(vec![
                MdLineFragment::Plain("Use the package manager "),
                MdLineFragment::Link(("pip", "https://pip.pypa.io/en/stable/")),
                MdLineFragment::Plain(" to install foobar."),
            ]),
            MdBlockElement::CodeBlock(convert_into_code_block_lines(
                Some("python"),
                vec![
                    "import foobar",
                    "",
                    "foobar.pluralize('word') # returns 'words'",
                    "foobar.pluralize('goose') # returns 'geese'",
                    "foobar.singularize('phenomena') # returns 'phenomenon'",
                ],
            )),
            MdBlockElement::UnorderedList(vec![
                vec![MdLineFragment::Plain("ul1")],
                vec![MdLineFragment::Plain("ul2")],
            ]),
            MdBlockElement::OrderedList(vec![
                vec![MdLineFragment::Plain("ol1")],
                vec![MdLineFragment::Plain("ol2")],
            ]),
            MdBlockElement::UnorderedList(vec![
                vec![
                    MdLineFragment::Checkbox(false),
                    MdLineFragment::Plain(" todo"),
                ],
                vec![
                    MdLineFragment::Checkbox(true),
                    MdLineFragment::Plain(" done"),
                ],
            ]),
            MdBlockElement::Text(vec![MdLineFragment::Plain("end")]),
        ]
    }
}
