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

use crate::{constants::*, *};

impl StyleUSSpanLines {
    pub fn pretty_print(&self) -> String {
        let mut it = vec![];

        for line in &self.items {
            it.push(line.pretty_print())
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

    pub fn from_ul(ul_lines: &Lines, maybe_current_box_computed_style: &Option<Style>) -> Self {
        let mut it = StyleUSSpanLines::default();

        // prefix: Eg: "- "
        let prefix_text = format!("{UNORDERED_LIST}{SPACE}");
        let prefix_style =
            maybe_current_box_computed_style.unwrap_or_default() + get_list_bullet_style();
        it.push(StyleUSSpanLine::from_fragments(
            &vec![MdLineFragment::Plain(&prefix_text)],
            &Some(prefix_style),
        ));

        // postfix: Eg: "foo *bar* [baz](url)"
        for fragments_in_one_line in ul_lines {
            it.push(StyleUSSpanLine::from_fragments(
                fragments_in_one_line,
                &Some(
                    maybe_current_box_computed_style.unwrap_or_default() + get_list_content_style(),
                ),
            ))
        }

        it
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
            // AI: 0.2. from_block(): ul -> StyleUSSpanLines
            MdBlockElement::UnorderedList(ul_lines) => {
                let it = StyleUSSpanLines::from_ul(ul_lines, maybe_current_box_computed_style);
                lines.items.extend(it.items)
            }
            // AI: 0.2. from_block(): ol -> StyleUSSpanLines
            MdBlockElement::OrderedList(_) => todo!(),
            // AI: 0.2. from_block(): cb -> StyleUSSpanLines
            MdBlockElement::CodeBlock(_) => todo!(),
            // AI: 0.2. from_block(): title -> StyleUSSpanLine
            MdBlockElement::Title(_) => todo!(),
            // AI: 0.2. from_block(): tags -> StyleUSSpanLine
            MdBlockElement::Tags(_) => todo!(),
        }

        lines
    }
}

enum HyperlinkType {
    Image,
    Link,
}

impl StyleUSSpan {
    fn format_hyperlink_data(
        link_data: &HyperlinkData,
        maybe_current_box_computed_style: &Option<Style>,
        hyperlink_type: HyperlinkType,
    ) -> Vec<Self> {
        let link_text = link_data.text.to_string();
        let link_url = link_data.url.to_string();

        let base_style =
            maybe_current_box_computed_style.unwrap_or_default() + get_link_base_style();

        let link_text_style =
            maybe_current_box_computed_style.unwrap_or_default() + get_link_text_style();

        let link_url_style =
            maybe_current_box_computed_style.unwrap_or_default() + get_link_url_style();

        vec![
            // [link_text] or ![link_text]
            StyleUSSpan::new(
                base_style,
                US::from(match hyperlink_type {
                    HyperlinkType::Link => LEFT_BRACKET,
                    HyperlinkType::Image => LEFT_IMAGE,
                }),
            ),
            StyleUSSpan::new(link_text_style, US::from(link_text)),
            StyleUSSpan::new(
                base_style,
                US::from(match hyperlink_type {
                    HyperlinkType::Link => RIGHT_BRACKET,
                    HyperlinkType::Image => RIGHT_IMAGE,
                }),
            ),
            // (link_url)
            StyleUSSpan::new(base_style, US::from(LEFT_PARENTHESIS)),
            StyleUSSpan::new(link_url_style, US::from(link_url)),
            StyleUSSpan::new(base_style, US::from(RIGHT_PARENTHESIS)),
        ]
    }

    /// Each [MdLineFragment] needs to be translated into a [StyleUSSpan] or [Vec] of
    /// [StyleUSSpan]s.
    ///
    /// 1. These are then rolled up into a [StyleUSSpanLine] by
    ///    [StyleUSSpanLine::from_fragments](StyleUSSpanLine::from_fragments) ...
    /// 2. ... which is then rolled up into [StyleUSSpanLines] by
    ///    [StyleUSSpanLine::from_block](StyleUSSpanLine::from_block).
    pub fn from_fragment(
        fragment: &MdLineFragment,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Vec<Self> {
        match fragment {
            MdLineFragment::Plain(plain_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default() + get_foreground_style(),
                US::from(*plain_text),
            )],

            MdLineFragment::Bold(bold_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default() + get_bold_style(),
                US::from(*bold_text),
            )],

            MdLineFragment::Italic(italic_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default() + get_italic_style(),
                US::from(*italic_text),
            )],

            MdLineFragment::BoldItalic(bitalic_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default() + get_bold_italic_style(),
                US::from(*bitalic_text),
            )],

            MdLineFragment::InlineCode(inline_code_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default() + get_inline_code_style(),
                MdLineFragment::InlineCode(inline_code_text).to_plain_text(),
            )],

            MdLineFragment::Link(link_data) => Self::format_hyperlink_data(
                link_data,
                maybe_current_box_computed_style,
                HyperlinkType::Link,
            ),

            MdLineFragment::Image(link_data) => Self::format_hyperlink_data(
                link_data,
                maybe_current_box_computed_style,
                HyperlinkType::Image,
            ),

            MdLineFragment::Checkbox(done) => {
                vec![if *done {
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_checkbox_checked_style(),
                        US::from(CHECKED_OUTPUT),
                    )
                } else {
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_checkbox_unchecked_style(),
                        US::from(UNCHECKED_OUTPUT),
                    )
                }]
            }
        }
    }
}

impl StyleUSSpanLine {
    pub fn pretty_print(&self) -> String {
        let mut it = vec![];
        for span in &self.items {
            let StyleUSSpan { style, text } = span;
            let line_text = format!("fragment[ {:?} , {:?} ]", text.string, style);
            it.push(line_text);
        }
        it.join("\n")
    }

    pub fn from_fragments(
        fragments_in_one_line: &FragmentsInOneLine,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut acc = vec![];

        for fragment in fragments_in_one_line {
            let vec_spans = StyleUSSpan::from_fragment(fragment, maybe_current_box_computed_style);
            acc.extend(vec_spans);
        }

        List { items: acc }
    }

    /// This is a sample [HeadingData] that needs to be converted into a [StyleUSSpanLine].
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
    pub fn from_heading_data(
        heading_data: &HeadingData,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut color_wheel = ColorWheel::from_heading_data(heading_data);
        let mut line = StyleUSSpanLine::default();

        let heading_level_span: StyleUSSpan = {
            let heading_level = heading_data.level.to_plain_text();
            let my_style = {
                maybe_current_box_computed_style.unwrap_or_default()
                    + style! {
                        attrib: [dim]
                    }
            };
            StyleUSSpan::new(my_style, heading_level)
        };

        let heading_text_span: StyleUSSpanLine = {
            let heading_text = heading_data.content.to_plain_text();
            let styled_texts = color_wheel.colorize_into_styled_texts(
                &heading_text,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(*maybe_current_box_computed_style),
            );
            StyleUSSpanLine::from(styled_texts)
        };

        line.items.push(heading_level_span);
        line.items.extend(heading_text_span.items);

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
            it.items.push(StyleUSSpan::new(*style, us.clone()));
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

        #[test]
        fn checkbox_unchecked() {
            let fragment = MdLineFragment::Checkbox(false);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq!(actual.len(), 1);

            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_checkbox_unchecked_style(),
                    US::from(UNCHECKED_OUTPUT)
                )
            );

            // println!("{}", List::from(actual).pretty_print());
        }

        #[test]
        fn checkbox_checked() {
            let fragment = MdLineFragment::Checkbox(true);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq!(actual.len(), 1);

            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_checkbox_checked_style(),
                    US::from(CHECKED_OUTPUT)
                )
            );

            // println!("{}", List::from(actual).pretty_print());
        }

        #[test]
        fn image() {
            let fragment = MdLineFragment::Image(HyperlinkData {
                text: "R3BL",
                url: "https://r3bl.com",
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq!(actual.len(), 6);

            // "!["
            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#c1b3d0"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("![")
                )
            );

            // Everything else is the same as the link() test below.
        }

        #[test]
        fn link() {
            let fragment = MdLineFragment::Link(HyperlinkData {
                text: "R3BL",
                url: "https://r3bl.com",
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq!(actual.len(), 6);

            // "["
            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#c1b3d0"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("[")
                )
            );

            // "Foobar"
            assert_eq2!(
                actual.get(1).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#4f86ed"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("R3BL")
                )
            );

            // "]"
            assert_eq2!(
                actual.get(2).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#c1b3d0"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("]")
                )
            );

            // "("
            assert_eq2!(
                actual.get(3).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#c1b3d0"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("(")
                )
            );

            // "https://r3bl.com"
            assert_eq2!(
                actual.get(4).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [underline]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#16adf3"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("https://r3bl.com")
                )
            );

            // ")"
            assert_eq2!(
                actual.get(5).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: TuiColor::Rgb(RgbValue::from_hex("#c1b3d0"))
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from(")")
                )
            );

            // println!("{}", List::from(actual).pretty_print());
        }

        #[test]
        fn inline_code() {
            let fragment = MdLineFragment::InlineCode("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);
            let expected = vec![StyleUSSpan::new(
                maybe_style.unwrap_or_default() + get_inline_code_style(),
                US::from("`Foobar`"),
            )];
            assert_eq2!(actual, expected);
        }

        #[test]
        fn italic() {
            let fragment = MdLineFragment::Italic("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);
            let expected = vec![StyleUSSpan::new(
                maybe_style.unwrap_or_default() + get_italic_style(),
                US::from("Foobar"),
            )];
            assert_eq2!(actual, expected);
        }

        #[test]
        fn bold() {
            let fragment = MdLineFragment::Bold("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);
            let expected = vec![StyleUSSpan::new(
                maybe_style.unwrap_or_default() + get_bold_style(),
                US::from("Foobar"),
            )];
            assert_eq2!(actual, expected);
        }

        #[test]
        fn plain() {
            let fragment = MdLineFragment::Plain("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);
            let expected = vec![StyleUSSpan::new(
                maybe_style.unwrap_or_default() + get_foreground_style(),
                US::from("Foobar"),
            )];
            assert_eq2!(actual, expected);
        }

        #[test]
        fn bold_italic() {
            let fragment = MdLineFragment::BoldItalic("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);
            let expected = vec![StyleUSSpan::new(
                maybe_style.unwrap_or_default() + get_bold_italic_style(),
                US::from("Foobar"),
            )];
            assert_eq2!(actual, expected);
        }
    }

    /// Test each variant of [MdBlockElement] is converted by
    /// [StyleUSSpanLines::from_block](StyleUSSpanLines::from_block).
    mod from_block {
        use super::*;

        // AI: 0.2. TEST from_block [ ] ul
        // AI: 0.2. TEST from_block [ ] ol
        // AI: 0.2. TEST from_block [ ] cb
        // AI: 0.2. TEST from_block [ ] title
        // AI: 0.2. TEST from_block [ ] tags

        #[test]
        fn text() {
            let text_block = MdBlockElement::Text(vec![MdLineFragment::Plain("Foobar")]);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines = StyleUSSpanLines::from_block(&text_block, &maybe_style);
            // println!("{}", lines.pretty_print());

            let line_0 = &lines.items[0];
            let span_0_in_line_0 = &line_0.items[0];
            let StyleUSSpan { style, text } = span_0_in_line_0;
            assert_eq2!(text.string, "Foobar");
            assert_eq2!(
                style,
                &(maybe_style.unwrap_or_default() + get_foreground_style())
            );
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
            let spans_in_line = &first_line.items;

            // There should be 7 spans in the line.
            assert_eq2!(spans_in_line.len(), 7);

            // First span is the heading level `# ` in dim w/ Red bg color, and no fg color.
            assert_eq2!(spans_in_line[0].style.dim, true);
            assert_eq2!(
                spans_in_line[0].style.color_bg.unwrap(),
                TuiColor::Basic(ANSIBasicColor::Red)
            );
            assert_eq2!(spans_in_line[0].style.color_fg.is_some(), false);
            assert_eq2!(spans_in_line[0].text.string, "# ");

            // The remainder of the spans are the heading text which are colorized with a color
            // wheel.
            for span in &spans_in_line[1..=6] {
                assert_eq2!(span.style.dim, false);
                assert_eq2!(
                    span.style.color_bg.unwrap(),
                    TuiColor::Basic(ANSIBasicColor::Red)
                );
                assert_eq2!(span.style.color_fg.is_some(), true);
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
                MdLineFragment::Link(HyperlinkData::new("pip", "https://pip.pypa.io/en/stable/")),
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
