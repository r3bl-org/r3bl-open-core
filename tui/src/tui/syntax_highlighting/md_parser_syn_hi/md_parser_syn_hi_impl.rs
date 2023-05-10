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
use syntect::{easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet};

use crate::{constants::*, *};

/// This is the main function that the [editor] uses this in order to display the markdown to the
/// user.It is responsible for converting:
/// - from a &[Vec] of [US] which comes from the [editor],
/// - into a [StyleUSSpanLines], which the [editor] will clip & render.
/// ## Arguments
/// - `editor_text` - The text that the user has typed into the editor.
/// - `current_box_computed_style` - The computed style of the box that the editor is in.
pub fn try_parse_and_highlight(
    editor_text_lines: &Vec<US>,
    maybe_current_box_computed_style: &Option<Style>,
    maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
) -> CommonResult<StyleUSSpanLines> {
    // Convert the editor text into a string.
    let editor_text_to_string = {
        let mut line_to_str_acc = Vec::<&str>::new();
        for line in editor_text_lines {
            line_to_str_acc.push(line.string.as_str());
            line_to_str_acc.push("\n");
        }
        line_to_str_acc.join("")
    };

    // Try and parse `editor_text_to_string` into a `Document`.
    match parse_markdown(&editor_text_to_string) {
        Ok((_, document)) => Ok(StyleUSSpanLines::from_document(
            &document,
            maybe_current_box_computed_style,
            maybe_syntect_tuple,
        )),
        Err(_) => CommonError::new_err_with_only_type(CommonErrorType::ParsingError),
    }
}

#[cfg(test)]
mod tests_try_parse_and_highlight {
    use super::*;

    #[test]
    fn from_vec_us() -> CommonResult<()> {
        let editor_text_lines = vec![US::new("Hello"), US::new("World")];
        let maybe_current_box_computed_style = Some(style! {
            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
        });

        let style_us_span_lines = try_parse_and_highlight(
            &editor_text_lines,
            &maybe_current_box_computed_style,
            None,
        )?;

        println!(
            "result: \n{}",
            ansi_term::Color::Cyan.paint(style_us_span_lines.pretty_print_debug())
        );

        assert_eq2!(editor_text_lines.len(), style_us_span_lines.len());
        let line_0 = &style_us_span_lines[0][0];
        let line_1 = &style_us_span_lines[1][0];
        assert_eq2!(editor_text_lines[0], line_0.text);
        assert_eq2!(editor_text_lines[1], line_1.text);

        assert_eq2!(
            line_0.style,
            maybe_current_box_computed_style.unwrap() + get_foreground_style()
        );
        assert_eq2!(
            line_1.style,
            maybe_current_box_computed_style.unwrap() + get_foreground_style()
        );

        Ok(())
    }
}

impl PrettyPrintDebug for StyleUSSpanLines {
    fn pretty_print_debug(&self) -> String {
        let mut it = vec![];

        for line in &self.items {
            it.push(line.pretty_print_debug())
        }

        it.join("\n")
    }
}

impl StyleUSSpanLines {
    pub fn from_document(
        document: &MdDocument,
        maybe_current_box_computed_style: &Option<Style>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();
        for block in document.iter() {
            let block_to_lines = StyleUSSpanLines::from_block(
                block,
                maybe_current_box_computed_style,
                maybe_syntect_tuple,
            );
            lines.items.extend(block_to_lines.items);
        }
        lines
    }

    /// Based on [ColorSupport::detect()](ColorSupport::detect()) & language we have the following:
    /// ```text
    /// |               | Truecolor      | ANSI           |
    /// |---------------|----------------|----------------|
    /// | Language Some | syntect        | fallback       |
    /// | Language None | fallback       | fallback       |
    /// ```
    ///
    /// Case 1: Fallback
    /// - 1st line        : "```": `get_foreground_dim_style()`, lang: `get_code_block_lang_style()`
    /// - 2nd line .. end : content: `get_inline_code_style()`
    /// - last line       : "```": `get_foreground_dim_style()`
    ///
    /// Case 2: Syntect
    /// - 1st line        : "```": `get_foreground_dim_style()`, lang: `get_code_block_lang_style()`
    /// - 2nd line .. end : use syntect to highlight
    /// - last line       : "```": `get_foreground_dim_style()`
    pub fn from_block_codeblock(
        code_block_lines: &CodeBlockLines,
        maybe_current_box_computed_style: &Option<Style>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        mod inner {
            use super::*;

            pub fn try_use_syntect(
                code_block_lines: &CodeBlockLines,
                maybe_current_box_computed_style: &Option<Style>,
                maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
            ) -> Option<StyleUSSpanLines> {
                let mut acc_lines_output = StyleUSSpanLines::default();

                // Process each line in code_block_lines.
                for code_block_line in code_block_lines.iter() {
                    let mut acc_line_output = StyleUSSpanLine::default();

                    let maybe_lang =
                        if let Some(code_block_line) = code_block_lines.items.first() {
                            code_block_line.language
                        } else {
                            None
                        };
                    let syntax_set = maybe_syntect_tuple?.0;
                    let lang = maybe_lang?;
                    let theme = maybe_syntect_tuple?.1;
                    let syntax_ref = try_get_syntax_ref(syntax_set, lang)?;

                    let mut highlighter = HighlightLines::new(syntax_ref, theme);

                    match code_block_line.content {
                        CodeBlockLineContent::Text(line_of_text) => {
                            let syntect_highlighted_line: Vec<(
                                syntect::highlighting::Style,
                                &str,
                            )> = highlighter
                                .highlight_line(line_of_text, syntax_set)
                                .ok()?;

                            let line_converted_to_tui: List<StyleUSSpan> =
                                syntect_to_styled_text_conversion::from_syntect_to_tui(
                                    syntect_highlighted_line,
                                );

                            acc_line_output += line_converted_to_tui;
                        }

                        // Same as fallback.
                        CodeBlockLineContent::StartTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                US::from(CODE_BLOCK_START_PARTIAL),
                            );
                            if let Some(language) = code_block_line.language {
                                acc_line_output += StyleUSSpan::new(
                                    maybe_current_box_computed_style.unwrap_or_default()
                                        + get_code_block_lang_style(),
                                    US::from(language),
                                );
                            }
                        }

                        // Same as fallback.
                        CodeBlockLineContent::EndTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                US::from(CODE_BLOCK_START_PARTIAL),
                            );
                        }
                    }

                    acc_lines_output += acc_line_output;
                }

                Some(acc_lines_output)
            }

            pub fn use_fallback(
                code_block_lines: &CodeBlockLines,
                maybe_current_box_computed_style: &Option<Style>,
            ) -> StyleUSSpanLines {
                let mut acc_lines_output = StyleUSSpanLines::default();

                // Process each line in code_block_lines.
                for code_block_line in code_block_lines.iter() {
                    let mut acc_line_output = StyleUSSpanLine::default();

                    match code_block_line.content {
                        CodeBlockLineContent::StartTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                US::from(CODE_BLOCK_START_PARTIAL),
                            );
                            if let Some(language) = code_block_line.language {
                                acc_line_output += StyleUSSpan::new(
                                    maybe_current_box_computed_style.unwrap_or_default()
                                        + get_code_block_lang_style(),
                                    US::from(language),
                                );
                            }
                        }

                        CodeBlockLineContent::EndTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                US::from(CODE_BLOCK_START_PARTIAL),
                            );
                        }

                        CodeBlockLineContent::Text(content) => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_code_block_content_style(),
                                US::from(content),
                            );
                        }
                    }

                    acc_lines_output += acc_line_output;
                }

                acc_lines_output
            }
        }

        match inner::try_use_syntect(
            code_block_lines,
            maybe_current_box_computed_style,
            maybe_syntect_tuple,
        ) {
            Some(syntect_output) => syntect_output,
            _ => inner::use_fallback(code_block_lines, maybe_current_box_computed_style),
        }
    }

    pub fn from_block_smart_list(
        input_ul_lines: &Lines,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut acc_lines_output = StyleUSSpanLines::default();

        // Process each line in ul_lines.
        for input_line in input_ul_lines.iter() {
            let mut acc_line_output = StyleUSSpanLine::default();

            let postfix_span_list = StyleUSSpanLine::from_fragments(
                input_line,
                maybe_current_box_computed_style,
            );

            acc_line_output += postfix_span_list;

            acc_lines_output += acc_line_output;
        }

        acc_lines_output
    }

    /// Each [MdBlockElement] needs to be translated into a line. The [MdBlockElement::CodeBlock] is
    /// the only block that needs to be translated into multiple lines. This is why the return type
    /// is a [StyleUSSpanLines] (and not a single line).
    pub fn from_block(
        block: &MdBlockElement,
        maybe_current_box_computed_style: &Option<Style>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();

        match block {
            MdBlockElement::Title(title) => {
                lines += StyleUSSpanLine::from_kvp(
                    TITLE,
                    title,
                    maybe_current_box_computed_style,
                );
            }
            MdBlockElement::Date(date) => {
                lines += StyleUSSpanLine::from_kvp(
                    DATE,
                    date,
                    maybe_current_box_computed_style,
                );
            }
            MdBlockElement::Tags(tags) => {
                lines += StyleUSSpanLine::from_csvp(
                    TAGS,
                    tags,
                    maybe_current_box_computed_style,
                );
            }
            MdBlockElement::Authors(authors) => {
                lines += StyleUSSpanLine::from_csvp(
                    AUTHORS,
                    authors,
                    maybe_current_box_computed_style,
                );
            }
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
            MdBlockElement::SmartList((list_lines, _bullet_kind, _indent)) => {
                lines += StyleUSSpanLines::from_block_smart_list(
                    list_lines,
                    maybe_current_box_computed_style,
                );
            }
            MdBlockElement::CodeBlock(code_block_lines) => {
                lines += StyleUSSpanLines::from_block_codeblock(
                    code_block_lines,
                    maybe_current_box_computed_style,
                    maybe_syntect_tuple,
                );
            }
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

        let base_style = maybe_current_box_computed_style.unwrap_or_default()
            + get_foreground_dim_style();

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
            MdLineFragment::OrderedListBullet {
                indent,
                number,
                is_first_line,
            } => {
                let bullet =
                    generate_ordered_list_item_bullet(indent, number, is_first_line);
                vec![StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_list_bullet_style(),
                    US::from(bullet),
                )]
            }

            MdLineFragment::UnorderedListBullet {
                indent,
                is_first_line,
            } => {
                let bullet = generate_unordered_list_item_bullet(indent, is_first_line);
                vec![StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_list_bullet_style(),
                    US::from(bullet),
                )]
            }

            MdLineFragment::Plain(plain_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default()
                    + get_foreground_style(),
                US::from(*plain_text),
            )],

            MdLineFragment::Bold(bold_text) => {
                vec![
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_foreground_dim_style(),
                        US::from(BOLD_1),
                    ),
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_bold_style(),
                        US::from(*bold_text),
                    ),
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_foreground_dim_style(),
                        US::from(BOLD_1),
                    ),
                ]
            }

            MdLineFragment::Italic(italic_text) => vec![
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(ITALIC_1),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_italic_style(),
                    US::from(*italic_text),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(ITALIC_1),
                ),
            ],

            MdLineFragment::BoldItalic(bitalic_text) => vec![
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(BITALIC_1),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_bold_italic_style(),
                    US::from(*bitalic_text),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(BITALIC_1),
                ),
            ],

            MdLineFragment::InlineCode(inline_code_text) => vec![
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(BACK_TICK),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_inline_code_style(),
                    US::from(*inline_code_text),
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(BACK_TICK),
                ),
            ],

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

impl PrettyPrintDebug for StyleUSSpanLine {
    fn pretty_print_debug(&self) -> String {
        let mut it = vec![];
        for span in &self.items {
            let StyleUSSpan { style, text } = span;
            let line_text = format!("fragment[ {:?} , {:?} ]", text.string, style);
            it.push(line_text);
        }
        it.join("\n")
    }
}

impl StyleUSSpanLine {
    pub fn from_fragments(
        fragments_in_one_line: &FragmentsInOneLine,
        maybe_current_box_computed_style: &Option<Style>,
    ) -> Self {
        let mut acc = vec![];

        for fragment in fragments_in_one_line.iter() {
            let vec_spans =
                StyleUSSpan::from_fragment(fragment, maybe_current_box_computed_style);
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
            let heading_level = US::from(heading_data.level.pretty_print_debug());
            let my_style = {
                maybe_current_box_computed_style.unwrap_or_default()
                    + style! {
                        attrib: [dim]
                    }
            };
            StyleUSSpan::new(my_style, heading_level)
        };

        let heading_text_span: StyleUSSpanLine = {
            let heading_text = UnicodeString::from(heading_data.text);
            let styled_texts = color_wheel.colorize_into_styled_texts(
                &heading_text,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(
                    *maybe_current_box_computed_style,
                ),
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
mod tests_style_us_span_lines_from {
    use r3bl_rs_utils_macro::style;

    use super::*;

    /// Test each [MdLineFragment] variant is converted by
    /// [StyleUSSpan::from_fragment](StyleUSSpan::from_fragment).
    mod from_fragment {
        use super::*;

        #[test]
        fn test_checkbox_unchecked() {
            let fragment = MdLineFragment::Checkbox(false);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq2!(actual.len(), 1);

            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_checkbox_unchecked_style(),
                    US::from(UNCHECKED_OUTPUT)
                )
            );

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_checkbox_checked() {
            let fragment = MdLineFragment::Checkbox(true);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq2!(actual.len(), 1);

            assert_eq2!(
                actual.get(0).unwrap(),
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_checkbox_checked_style(),
                    US::from(CHECKED_OUTPUT)
                )
            );

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_image() {
            let fragment = MdLineFragment::Image(HyperlinkData {
                text: "R3BL",
                url: "https://r3bl.com",
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq2!(actual.len(), 6);

            // "!["
            let actual = actual.get(0).unwrap();
            let actual_style_color_fg = actual
                .style
                .color_fg
                .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
            assert_eq2!(
                actual,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default()
                        + style! {
                            attrib: [dim]
                            color_fg: actual_style_color_fg
                            color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                        },
                    US::from("![")
                )
            );

            // Everything else is the same as the link() test below.
        }

        #[test]
        fn test_link() {
            let fragment = MdLineFragment::Link(HyperlinkData {
                text: "R3BL",
                url: "https://r3bl.com",
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            assert_eq2!(actual.len(), 6);

            // "["
            {
                let actual = actual.get(0).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                attrib: [dim]
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from("[")
                    )
                );
            }

            // "Foobar"
            {
                let actual = actual.get(1).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from("R3BL")
                    )
                )
            };

            // "]"
            {
                let actual = actual.get(2).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                attrib: [dim]
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from("]")
                    )
                );
            }

            // "("
            {
                let actual = actual.get(3).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                attrib: [dim]
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from("(")
                    )
                );
            }

            // "https://r3bl.com"
            {
                let actual = actual.get(4).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                attrib: [underline]
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from("https://r3bl.com")
                    )
                );
            }

            // ")"
            {
                let actual = actual.get(5).unwrap();
                let actual_style_color_fg = actual
                    .style
                    .color_fg
                    .unwrap_or(TuiColor::Basic(ANSIBasicColor::White));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        maybe_style.unwrap_or_default()
                            + style! {
                                attrib: [dim]
                                color_fg: actual_style_color_fg
                                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
                            },
                        US::from(")")
                    )
                );
            }

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_inline_code() {
            let fragment = MdLineFragment::InlineCode("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("`"),
                )
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_inline_code_style(),
                    US::from("Foobar"),
                )
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("`"),
                )
            );
        }

        #[test]
        fn test_italic() {
            let fragment = MdLineFragment::Italic("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("*"),
                )
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_italic_style(),
                    US::from("Foobar"),
                )
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("*"),
                )
            );
        }

        #[test]
        fn test_bold() {
            let fragment = MdLineFragment::Bold("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("**"),
                )
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_bold_style(),
                    US::from("Foobar"),
                )
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("**"),
                )
            );
        }

        #[test]
        fn test_bold_italic() {
            let fragment = MdLineFragment::BoldItalic("Foobar");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let actual = StyleUSSpan::from_fragment(&fragment, &maybe_style);

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("***"),
                )
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_bold_italic_style(),
                    US::from("Foobar"),
                )
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("***"),
                )
            );
        }

        #[test]
        fn test_plain() {
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
    }

    /// Test each variant of [MdBlockElement] is converted by
    /// [StyleUSSpanLines::from_block](StyleUSSpanLines::from_block).
    mod from_block {
        use super::*;

        #[test]
        fn test_block_metadata_tags() -> Result<(), ()> {
            let tags = MdBlockElement::Tags(list!["tag1", "tag2", "tag3"]);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let lines = StyleUSSpanLines::from_block(&tags, &maybe_style, None);
            let line_0 = &lines.items[0];
            let mut iter = line_0.items.iter();

            // println!("{}", line_0..pretty_print_debug());
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_tags_marker_style(),
                    US::from("@tags"),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from(": "),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_tags_values_style(),
                    US::from("tag1"),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from(", "),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_tags_values_style(),
                    US::from("tag2"),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from(", "),
                )
            );
            assert_eq2!(
                iter.next().ok_or(())?,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_tags_values_style(),
                    US::from("tag3"),
                )
            );

            Ok(())
        }

        #[test]
        fn test_block_metadata_title() {
            let title = MdBlockElement::Title("Something");
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let lines = StyleUSSpanLines::from_block(&title, &maybe_style, None);
            // println!("{}", lines..pretty_print_debug());

            let line_0 = &lines.items[0];

            let span_0 = &line_0.items[0];
            assert_eq2!(
                span_0,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_title_marker_style(),
                    US::from("@title"),
                )
            );

            let span_1 = &line_0.items[1];
            assert_eq2!(
                span_1,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from(": "),
                )
            );

            let span_2 = &line_0.items[2];
            assert_eq2!(
                span_2,
                &StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_metadata_title_value_style(),
                    US::from("Something"),
                )
            );
        }

        #[test]
        fn test_block_codeblock() {
            let codeblock_block = MdBlockElement::CodeBlock(list!(
                CodeBlockLine {
                    language: Some("ts"),
                    content: CodeBlockLineContent::StartTag
                },
                CodeBlockLine {
                    language: Some("ts"),
                    content: CodeBlockLineContent::Text("let a = 1;")
                },
                CodeBlockLine {
                    language: Some("ts"),
                    content: CodeBlockLineContent::EndTag
                },
            ));

            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines =
                StyleUSSpanLines::from_block(&codeblock_block, &maybe_style, None);

            let line_0 = &lines.items[0];
            // println!("{}", line_0..pretty_print_debug());
            assert_eq2!(
                line_0.items[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("```"),
                )
            );
            assert_eq2!(
                line_0.items[1],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_code_block_lang_style(),
                    US::from("ts"),
                )
            );

            let line_1 = &lines.items[1];
            // println!("{}", line_1..pretty_print_debug());
            assert_eq2!(
                line_1.items[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_code_block_content_style(),
                    US::from("let a = 1;"),
                )
            );

            let line_2 = &lines.items[2];
            // println!("{}", line_2..pretty_print_debug());
            assert_eq2!(
                line_2.items[0],
                StyleUSSpan::new(
                    maybe_style.unwrap_or_default() + get_foreground_dim_style(),
                    US::from("```"),
                )
            );
        }

        #[test]
        fn test_block_ol() -> CommonResult<()> {
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let (remainder, doc) = parse_markdown("100. Foo\n200. Bar\n")?;
            assert_eq2!(remainder, "");

            let ol_block_1 = &doc[0];
            {
                // println!("{:#?}", ol_block_1);
                let lines = StyleUSSpanLines::from_block(ol_block_1, &maybe_style, None);

                let line_0 = &lines.items[0];
                // println!("{}", line_0..pretty_print_debug());
                assert_eq2!(
                    line_0.items[0],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_list_bullet_style(),
                        US::from("100.│")
                    )
                );
                assert_eq2!(
                    line_0.items[1],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_foreground_style(),
                        US::from("Foo"),
                    )
                );
            }

            let ol_block_2 = &doc[1];
            {
                // println!("{:#?}", ol_block_2);
                let lines = StyleUSSpanLines::from_block(ol_block_2, &maybe_style, None);

                let line_0 = &lines.items[0];
                // println!("{}", line_0..pretty_print_debug());
                assert_eq2!(
                    line_0.items[0],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_list_bullet_style(),
                        US::from("200.│")
                    )
                );
                assert_eq2!(
                    line_0.items[1],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_foreground_style(),
                        US::from("Bar"),
                    )
                );
            }

            Ok(())
        }

        #[test]
        fn test_block_ul() -> CommonResult<()> {
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });
            let (_, doc) = parse_markdown("- Foo\n- Bar\n")?;
            println!("{}", ansi_term::Color::Cyan.paint(format!("{:#?}", doc)));

            // First smart list.
            {
                let ul_block_0 = &doc[0];
                let lines = StyleUSSpanLines::from_block(ul_block_0, &maybe_style, None);
                let line_0 = &lines.items[0];
                assert_eq2!(
                    line_0.items[0],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_list_bullet_style(),
                        US::from("─┤")
                    )
                );
                assert_eq2!(
                    line_0.items[1],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_foreground_style(),
                        US::from("Foo")
                    )
                );
            }

            // Second smart list.
            {
                let ul_block_1 = &doc[1];
                let lines = StyleUSSpanLines::from_block(ul_block_1, &maybe_style, None);
                let line_0 = &lines.items[0];
                assert_eq2!(
                    line_0.items[0],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_list_bullet_style(),
                        US::from("─┤")
                    )
                );
                assert_eq2!(
                    line_0.items[1],
                    StyleUSSpan::new(
                        maybe_style.unwrap_or_default() + get_foreground_style(),
                        US::from("Bar")
                    )
                );
            }

            Ok(())
        }

        #[test]
        fn test_block_text() {
            let text_block = MdBlockElement::Text(list![MdLineFragment::Plain("Foobar")]);
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines = StyleUSSpanLines::from_block(&text_block, &maybe_style, None);
            // println!("{}", lines..pretty_print_debug());

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
        fn test_block_heading() {
            let heading_block = MdBlockElement::Heading(HeadingData {
                level: HeadingLevel::Heading1,
                text: "Foobar",
            });
            let maybe_style = Some(style! {
                color_bg: TuiColor::Basic(ANSIBasicColor::Red)
            });

            let lines = StyleUSSpanLines::from_block(&heading_block, &maybe_style, None);
            // println!("{}", lines..pretty_print_debug());

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
}
