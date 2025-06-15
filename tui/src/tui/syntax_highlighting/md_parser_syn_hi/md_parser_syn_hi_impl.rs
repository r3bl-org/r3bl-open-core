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

//! This module is responsible for converting a [MdDocument] into a [StyleUSSpanLines].

use smallvec::smallvec;
use syntect::{highlighting::Theme, parsing::SyntaxSet};

use super::create_color_wheel_from_heading_data;
use crate::{generate_ordered_list_item_bullet,
            generate_unordered_list_item_bullet,
            get_bold_style,
            get_checkbox_checked_style,
            get_checkbox_unchecked_style,
            get_code_block_content_style,
            get_code_block_lang_style,
            get_foreground_dim_style,
            get_foreground_style,
            get_inline_code_style,
            get_italic_style,
            get_link_text_style,
            get_link_url_style,
            get_list_bullet_style,
            join,
            new_style,
            parse_markdown,
            tui::{md_parser::constants::{AUTHORS,
                                         BACK_TICK,
                                         CHECKED_OUTPUT,
                                         DATE,
                                         LEFT_BRACKET,
                                         LEFT_IMAGE,
                                         LEFT_PARENTHESIS,
                                         NEW_LINE,
                                         RIGHT_BRACKET,
                                         RIGHT_IMAGE,
                                         RIGHT_PARENTHESIS,
                                         STAR,
                                         TAGS,
                                         TITLE,
                                         UNCHECKED_OUTPUT,
                                         UNDERSCORE},
                  md_parser_alt::AsStrSlice},
            CodeBlockLineContent,
            CodeBlockLines,
            CommonError,
            CommonErrorType,
            CommonResult,
            FragmentsInOneLine,
            GCString,
            GCStringExt,
            GradientGenerationPolicy,
            HeadingData,
            HyperlinkData,
            InlineString,
            Lines,
            List,
            MdDocument,
            MdElement,
            MdLineFragment,
            ParserByteCache,
            PrettyPrintDebug,
            StyleUSSpan,
            StyleUSSpanLine,
            StyleUSSpanLines,
            TextColorizationPolicy,
            TuiStyle,
            TuiStyledTexts};

/// This is the main function that the [crate::editor] uses this in order to display the
/// markdown to the user.It is responsible for converting:
/// - from a &[Vec] of [GCString] which comes from the [crate::editor],
/// - into a [StyleUSSpanLines], which the [crate::editor] will clip & render.
///
/// # Arguments
/// - `editor_text_lines` - The text that the user has typed into the editor.
/// - `current_box_computed_style` - The computed style of the box that the editor is in.
/// - `maybe_syntect_tuple` - The syntax set and theme that the editor should use to
///   highlight the text.
/// - `parser_byte_cache` - A cache that is used to store the byte array that results from
///   adding CRLF back into the document [crate::sizing::VecEditorContentLines]. This is
///   used to avoid re-allocating this struct every time the document is re-parsed, which
///   requires this byte array to be re-created with CRLF added to the document contents
///   (which may have changed).
pub fn try_parse_and_highlight(
    editor_text_lines: &[GCString],
    maybe_current_box_computed_style: &Option<TuiStyle>,
    maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    parser_byte_cache: Option<&mut ParserByteCache>,
) -> CommonResult<StyleUSSpanLines> {
    // XMARK: Parse markdown from editor and render it

    // PERF: This is a known performance bottleneck. The underlying storage mechanism for
    // content in the editor will have to change (from Vec<String>) for this to be
    // possible.

    // Convert the editor text into a InlineString (unfortunately requires allocating to
    // get the new lines back, since they're stripped out when loading content into the
    // editor buffer struct).

    let slice = AsStrSlice::from(editor_text_lines);

    let size_hint = editor_text_lines
        .iter()
        .map(|line| line.len().as_usize() + 1 /* for new line */)
        .sum();

    // Use the parser_byte_cache if it exists, otherwise create a new one with the
    // size_hint.
    let acc = match parser_byte_cache {
        // If the parser_byte_cache exists, we can write to it directly.
        Some(parser_byte_cache) => parser_byte_cache,
        // If it doesn't exist, we create a new one with the size hint.
        None => &mut ParserByteCache::with_capacity(size_hint),
    };

    slice.write_to_byte_cache_compat(size_hint, acc);
    let result_md_ast = parse_markdown(acc);

    // Try and parse `editor_text_to_string` into a `Document`.
    match result_md_ast {
        Ok((_remainder, document)) => Ok(StyleUSSpanLines::from_document(
            &document,
            maybe_current_box_computed_style,
            maybe_syntect_tuple,
        )),
        Err(_) => {
            CommonError::new_error_result_with_only_type(CommonErrorType::ParsingError)
        }
    }
}

#[cfg(test)]
mod tests_try_parse_and_highlight {
    use super::*;
    use crate::{assert_eq2, fg_cyan, throws, tui_color};

    #[test]
    fn from_vec_gcs() -> CommonResult<()> {
        throws!({
            let editor_text_lines = ["Hello", "World"].map(GCString::new);
            let current_box_computed_style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let style_us_span_lines = try_parse_and_highlight(
                &editor_text_lines,
                &Some(current_box_computed_style),
                None,
                None,
            )?;

            println!(
                "result: \n{}",
                fg_cyan(style_us_span_lines.pretty_print_debug())
            );

            assert_eq2!(editor_text_lines.len(), style_us_span_lines.len());
            let line_0 = &style_us_span_lines[0][0];
            let line_1 = &style_us_span_lines[1][0];
            assert_eq2!(editor_text_lines[0], line_0.text_gcs);
            assert_eq2!(editor_text_lines[1], line_1.text_gcs);

            assert_eq2!(
                line_0.style,
                current_box_computed_style + get_foreground_style()
            );
            assert_eq2!(
                line_1.style,
                current_box_computed_style + get_foreground_style()
            );
        });
    }
}

impl PrettyPrintDebug for StyleUSSpanLines {
    fn pretty_print_debug(&self) -> InlineString {
        join!(
            from: self.inner,
            each: line,
            delim: NEW_LINE,
            format: "{}",
            line.pretty_print_debug()
        )
    }
}

impl StyleUSSpanLines {
    pub fn from_document(
        document: &MdDocument<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();
        for block in document.iter() {
            let block_to_lines = StyleUSSpanLines::from_block(
                block,
                maybe_current_box_computed_style,
                maybe_syntect_tuple,
            );
            lines.inner.extend(block_to_lines.inner);
        }
        lines
    }

    /// Based on [crate::global_color_support::detect] & language we have the
    /// following:
    /// ```text
    /// |               | Truecolor      | ANSI           |
    /// |---------------|----------------|----------------|
    /// | Language Some | syntect        | fallback       |
    /// | Language None | fallback       | fallback       |
    /// ```
    ///
    /// Case 1: Fallback
    /// - 1st line        : "```": `get_foreground_dim_style()`, lang:
    ///   `get_code_block_lang_style()`
    /// - 2nd line .. end : content: `get_inline_code_style()`
    /// - last line       : "```": `get_foreground_dim_style()`
    ///
    /// Case 2: Syntect
    /// - 1st line        : "```": `get_foreground_dim_style()`, lang:
    ///   `get_code_block_lang_style()`
    /// - 2nd line .. end : use syntect to highlight
    /// - last line       : "```": `get_foreground_dim_style()`
    pub fn from_block_codeblock(
        code_block_lines: &CodeBlockLines<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        mod inner {
            use syntect::easy::HighlightLines;

            use super::*;
            use crate::{convert_syntect_to_styled_text,
                        try_get_syntax_ref,
                        tui::constants::CODE_BLOCK_START_PARTIAL};

            pub fn try_use_syntect(
                code_block_lines: &CodeBlockLines<'_>,
                maybe_current_box_computed_style: &Option<TuiStyle>,
                maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
            ) -> Option<StyleUSSpanLines> {
                let mut acc_lines_output = StyleUSSpanLines::default();

                // Process each line in code_block_lines.
                for code_block_line in code_block_lines.iter() {
                    let mut acc_line_output = StyleUSSpanLine::default();

                    let maybe_lang =
                        if let Some(code_block_line) = code_block_lines.inner.first() {
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
                                convert_syntect_to_styled_text::convert_highlighted_line_from_syntect_to_tui(
                                    syntect_highlighted_line,
                                );

                            acc_line_output += line_converted_to_tui;
                        }

                        // Same as fallback.
                        CodeBlockLineContent::StartTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                CODE_BLOCK_START_PARTIAL,
                            );
                            if let Some(language) = code_block_line.language {
                                acc_line_output += StyleUSSpan::new(
                                    maybe_current_box_computed_style.unwrap_or_default()
                                        + get_code_block_lang_style(),
                                    language,
                                );
                            }
                        }

                        // Same as fallback.
                        CodeBlockLineContent::EndTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                CODE_BLOCK_START_PARTIAL,
                            );
                        }
                    }

                    acc_lines_output += acc_line_output;
                }

                Some(acc_lines_output)
            }

            pub fn use_fallback(
                code_block_lines: &CodeBlockLines<'_>,
                maybe_current_box_computed_style: &Option<TuiStyle>,
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
                                CODE_BLOCK_START_PARTIAL,
                            );
                            if let Some(language) = code_block_line.language {
                                acc_line_output += StyleUSSpan::new(
                                    maybe_current_box_computed_style.unwrap_or_default()
                                        + get_code_block_lang_style(),
                                    language,
                                );
                            }
                        }

                        CodeBlockLineContent::EndTag => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_foreground_dim_style(),
                                CODE_BLOCK_START_PARTIAL,
                            );
                        }

                        CodeBlockLineContent::Text(content) => {
                            acc_line_output += StyleUSSpan::new(
                                maybe_current_box_computed_style.unwrap_or_default()
                                    + get_code_block_content_style(),
                                content,
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
        input_ul_lines: &Lines<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
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

    /// Each [MdElement] needs to be translated into a line. The [MdElement::CodeBlock] is
    /// the only block that needs to be translated into multiple lines. This is why the
    /// return type is a [StyleUSSpanLines] (and not a single line).
    pub fn from_block(
        block: &MdElement<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
        maybe_syntect_tuple: Option<(&SyntaxSet, &Theme)>,
    ) -> Self {
        let mut lines = StyleUSSpanLines::default();

        match block {
            MdElement::Title(title) => {
                lines += StyleUSSpanLine::from_kvp(
                    TITLE,
                    title,
                    maybe_current_box_computed_style,
                );
            }
            MdElement::Date(date) => {
                lines += StyleUSSpanLine::from_kvp(
                    DATE,
                    date,
                    maybe_current_box_computed_style,
                );
            }
            MdElement::Tags(tags) => {
                lines += StyleUSSpanLine::from_csvp(
                    TAGS,
                    tags,
                    maybe_current_box_computed_style,
                );
            }
            MdElement::Authors(authors) => {
                lines += StyleUSSpanLine::from_csvp(
                    AUTHORS,
                    authors,
                    maybe_current_box_computed_style,
                );
            }
            MdElement::Heading(heading_data) => {
                lines.push(StyleUSSpanLine::from_heading_data(
                    heading_data,
                    maybe_current_box_computed_style,
                ));
            }
            MdElement::Text(fragments_in_one_line) => {
                lines.push(StyleUSSpanLine::from_fragments(
                    fragments_in_one_line,
                    maybe_current_box_computed_style,
                ))
            }
            MdElement::SmartList((list_lines, _bullet_kind, _indent)) => {
                lines += StyleUSSpanLines::from_block_smart_list(
                    list_lines,
                    maybe_current_box_computed_style,
                );
            }
            MdElement::CodeBlock(code_block_lines) => {
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
        link_data: &HyperlinkData<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
        hyperlink_type: HyperlinkType,
    ) -> Vec<Self> {
        let link_text = link_data.text;
        let link_url = link_data.url;

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
                match hyperlink_type {
                    HyperlinkType::Link => LEFT_BRACKET,
                    HyperlinkType::Image => LEFT_IMAGE,
                },
            ),
            StyleUSSpan::new(link_text_style, link_text),
            StyleUSSpan::new(
                base_style,
                match hyperlink_type {
                    HyperlinkType::Link => RIGHT_BRACKET,
                    HyperlinkType::Image => RIGHT_IMAGE,
                },
            ),
            // (link_url)
            StyleUSSpan::new(base_style, LEFT_PARENTHESIS),
            StyleUSSpan::new(link_url_style, link_url),
            StyleUSSpan::new(base_style, RIGHT_PARENTHESIS),
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
        fragment: &MdLineFragment<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
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
                    &bullet,
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
                    &bullet,
                )]
            }

            MdLineFragment::Plain(plain_text) => vec![StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default()
                    + get_foreground_style(),
                plain_text,
            )],

            MdLineFragment::Bold(bold_text) => {
                vec![
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_foreground_dim_style(),
                        STAR,
                    ),
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_bold_style(),
                        bold_text,
                    ),
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_foreground_dim_style(),
                        STAR,
                    ),
                ]
            }

            MdLineFragment::Italic(italic_text) => vec![
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    UNDERSCORE,
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_italic_style(),
                    italic_text,
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    UNDERSCORE,
                ),
            ],

            MdLineFragment::InlineCode(inline_code_text) => vec![
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    BACK_TICK,
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_inline_code_style(),
                    inline_code_text,
                ),
                StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    BACK_TICK,
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
                        CHECKED_OUTPUT,
                    )
                } else {
                    StyleUSSpan::new(
                        maybe_current_box_computed_style.unwrap_or_default()
                            + get_checkbox_unchecked_style(),
                        UNCHECKED_OUTPUT,
                    )
                }]
            }
        }
    }
}

impl PrettyPrintDebug for StyleUSSpanLine {
    fn pretty_print_debug(&self) -> InlineString {
        join!(
            from: self.inner,
            each: span,
            delim: NEW_LINE,
            format: "fragment[ {} , {:?} ]", &span.text_gcs.string, span.style
        )
    }
}

impl StyleUSSpanLine {
    pub fn from_fragments(
        fragments_in_one_line: &FragmentsInOneLine<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut acc = smallvec![];

        for fragment in fragments_in_one_line.iter() {
            let vec_spans =
                StyleUSSpan::from_fragment(fragment, maybe_current_box_computed_style);
            acc.extend(vec_spans);
        }

        List { inner: acc }
    }

    /// This is a sample [HeadingData] that needs to be converted into a
    /// [StyleUSSpanLine].
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
        heading_data: &HeadingData<'_>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut color_wheel = create_color_wheel_from_heading_data(heading_data);
        let mut acc_line = StyleUSSpanLine::default();
        let heading_level_span: StyleUSSpan = {
            let heading_level_string = heading_data.level.pretty_print_debug();
            let my_style = {
                maybe_current_box_computed_style.unwrap_or_default() + new_style!(dim)
            };
            StyleUSSpan::new(my_style, &heading_level_string)
        };

        let heading_text_span: StyleUSSpanLine = {
            let heading_text = heading_data.text;
            let heading_text_gcs = heading_text.grapheme_string();
            let styled_texts = color_wheel.colorize_into_styled_texts(
                &heading_text_gcs,
                GradientGenerationPolicy::ReuseExistingGradientAndResetIndex,
                TextColorizationPolicy::ColorEachCharacter(
                    *maybe_current_box_computed_style,
                ),
            );
            StyleUSSpanLine::from(styled_texts)
        };

        acc_line.inner.push(heading_level_span);
        acc_line.inner.extend(heading_text_span.inner);

        acc_line
    }
}

impl From<TuiStyledTexts> for StyleUSSpanLine {
    fn from(styled_texts: TuiStyledTexts) -> Self {
        let mut it = StyleUSSpanLine::default();
        // More info on `into_iter`: <https://users.rust-lang.org/t/move-value-from-an-iterator/46172>
        for styled_text in styled_texts.inner.into_iter() {
            let style = styled_text.get_style();
            let text = styled_text.get_text();
            it.inner.push(StyleUSSpan::new(*style, text));
        }
        it
    }
}

#[cfg(test)]
mod tests_style_us_span_lines_from {
    use miette::IntoDiagnostic as _;

    use super::*;
    use crate::{assert_eq2,
                fg_cyan,
                get_metadata_tags_marker_style,
                get_metadata_tags_values_style,
                get_metadata_title_marker_style,
                get_metadata_title_value_style,
                list,
                throws,
                tui_color,
                CodeBlockLine,
                HeadingLevel};

    /// Test each [MdLineFragment] variant is converted by
    /// [StyleUSSpan::from_fragment](StyleUSSpan::from_fragment).
    mod from_fragment {
        use super::*;

        #[test]
        fn test_checkbox_unchecked() {
            let fragment = MdLineFragment::Checkbox(false);
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            assert_eq2!(actual.len(), 1);

            assert_eq2!(
                actual.first().unwrap(),
                &StyleUSSpan::new(
                    style + get_checkbox_unchecked_style(),
                    UNCHECKED_OUTPUT,
                )
            );

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_checkbox_checked() {
            let fragment = MdLineFragment::Checkbox(true);
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            assert_eq2!(actual.len(), 1);

            assert_eq2!(
                actual.first().unwrap(),
                &StyleUSSpan::new(style + get_checkbox_checked_style(), CHECKED_OUTPUT,)
            );

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_image() {
            let fragment = MdLineFragment::Image(HyperlinkData {
                text: "R3BL",
                url: "https://r3bl.com",
            });
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            assert_eq2!(actual.len(), 6);

            // "!["
            let actual = actual.first().unwrap();
            let actual_style_color_fg =
                actual.style.color_fg.unwrap_or(tui_color!(white));
            assert_eq2!(
                actual,
                &StyleUSSpan::new(
                    style
                        + new_style!(
                            dim
                            color_fg: {actual_style_color_fg}
                            color_bg: {tui_color!(red)}
                        ),
                    "![",
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
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            assert_eq2!(actual.len(), 6);

            // "["
            {
                let actual = actual.first().unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                dim
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        "[",
                    )
                );
            }

            // "Foobar"
            {
                let actual = actual.get(1).unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        "R3BL",
                    )
                )
            };

            // "]"
            {
                let actual = actual.get(2).unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                dim
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        "]",
                    )
                );
            }

            // "("
            {
                let actual = actual.get(3).unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                dim
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        "(",
                    )
                );
            }

            // "https://r3bl.com"
            {
                let actual = actual.get(4).unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                underline
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        "https://r3bl.com",
                    )
                );
            }

            // ")"
            {
                let actual = actual.get(5).unwrap();
                let actual_style_color_fg =
                    actual.style.color_fg.unwrap_or(tui_color!(white));
                assert_eq2!(
                    actual,
                    &StyleUSSpan::new(
                        style
                            + new_style!(
                                dim
                                color_fg: {actual_style_color_fg}
                                color_bg: {tui_color!(red)}
                            ),
                        ")",
                    )
                );
            }

            // println!("{}", List::from(actual)..pretty_print_debug());
        }

        #[test]
        fn test_inline_code() {
            let fragment = MdLineFragment::InlineCode("Foobar");
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(style + get_foreground_dim_style(), "`",)
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(style + get_inline_code_style(), "Foobar",)
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(style + get_foreground_dim_style(), "`",)
            );
        }

        #[test]
        fn test_italic() {
            let fragment = MdLineFragment::Italic("Foobar");
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(style + get_foreground_dim_style(), "_",)
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(style + get_italic_style(), "Foobar",)
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(style + get_foreground_dim_style(), "_",)
            );
        }

        #[test]
        fn test_bold() {
            let fragment = MdLineFragment::Bold("Foobar");
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));

            // println!("{}", List::from(actual.clone())..pretty_print_debug());

            assert_eq2!(
                actual[0],
                StyleUSSpan::new(style + get_foreground_dim_style(), "*",)
            );
            assert_eq2!(
                actual[1],
                StyleUSSpan::new(style + get_bold_style(), "Foobar",)
            );
            assert_eq2!(
                actual[2],
                StyleUSSpan::new(style + get_foreground_dim_style(), "*",)
            );
        }

        #[test]
        fn test_plain() {
            let fragment = MdLineFragment::Plain("Foobar");
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let actual = StyleUSSpan::from_fragment(&fragment, &Some(style));
            let expected =
                vec![StyleUSSpan::new(style + get_foreground_style(), "Foobar")];
            assert_eq2!(actual, expected);
        }
    }

    /// Test each variant of [MdBlockElement] is converted by
    /// [StyleUSSpanLines::from_block](StyleUSSpanLines::from_block).
    mod from_block {
        use super::*;

        #[test]
        fn test_block_metadata_tags() -> Result<(), ()> {
            throws!({
                let tags = MdElement::Tags(list!["tag1", "tag2", "tag3"]);
                let style = new_style!(
                    color_bg: {tui_color!(red)}
                );
                let lines = StyleUSSpanLines::from_block(&tags, &Some(style), None);
                let line_0 = &lines.inner[0];
                let mut iter = line_0.inner.iter();

                // println!("{}", line_0..pretty_print_debug());
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_metadata_tags_marker_style(), "@tags",)
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_foreground_dim_style(), ": ")
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_metadata_tags_values_style(), "tag1",)
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_foreground_dim_style(), ", ")
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_metadata_tags_values_style(), "tag2",)
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_foreground_dim_style(), ", ")
                );
                assert_eq2!(
                    iter.next().ok_or(())?,
                    &StyleUSSpan::new(style + get_metadata_tags_values_style(), "tag3",)
                );
            });
        }

        #[test]
        fn test_block_metadata_title() {
            let title = MdElement::Title("Something");
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );
            let lines = StyleUSSpanLines::from_block(&title, &Some(style), None);
            // println!("{}", lines..pretty_print_debug());

            let line_0 = &lines.inner[0];

            let span_0 = &line_0.inner[0];
            assert_eq2!(
                span_0,
                &StyleUSSpan::new(style + get_metadata_title_marker_style(), "@title",)
            );

            let span_1 = &line_0.inner[1];
            assert_eq2!(
                span_1,
                &StyleUSSpan::new(style + get_foreground_dim_style(), ": ")
            );

            let span_2 = &line_0.inner[2];
            assert_eq2!(
                span_2,
                &StyleUSSpan::new(style + get_metadata_title_value_style(), "Something",)
            );
        }

        #[test]
        fn test_block_codeblock() {
            let codeblock_block = MdElement::CodeBlock(list!(
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

            let style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let lines =
                StyleUSSpanLines::from_block(&codeblock_block, &Some(style), None);

            let line_0 = &lines.inner[0];
            // println!("{}", line_0..pretty_print_debug());
            assert_eq2!(
                line_0.inner[0],
                StyleUSSpan::new(style + get_foreground_dim_style(), "```",)
            );
            assert_eq2!(
                line_0.inner[1],
                StyleUSSpan::new(style + get_code_block_lang_style(), "ts",)
            );

            let line_1 = &lines.inner[1];
            // println!("{}", line_1..pretty_print_debug());
            assert_eq2!(
                line_1.inner[0],
                StyleUSSpan::new(style + get_code_block_content_style(), "let a = 1;",)
            );

            let line_2 = &lines.inner[2];
            // println!("{}", line_2..pretty_print_debug());
            assert_eq2!(
                line_2.inner[0],
                StyleUSSpan::new(style + get_foreground_dim_style(), "```",)
            );
        }

        #[test]
        fn test_block_ol() -> CommonResult<()> {
            throws!({
                let style = new_style!(
                    color_bg: {tui_color!(red)}
                );
                let (remainder, doc) =
                    parse_markdown("100. Foo\n200. Bar\n").into_diagnostic()?;
                assert_eq2!(remainder, "");

                let ol_block_1 = &doc[0];
                {
                    // println!("{:#?}", ol_block_1);
                    let lines =
                        StyleUSSpanLines::from_block(ol_block_1, &Some(style), None);

                    let line_0 = &lines.inner[0];
                    // println!("{}", line_0..pretty_print_debug());
                    assert_eq2!(
                        line_0.inner[0],
                        StyleUSSpan::new(style + get_list_bullet_style(), "100.│",)
                    );
                    assert_eq2!(
                        line_0.inner[1],
                        StyleUSSpan::new(style + get_foreground_style(), "Foo",)
                    );
                }

                let ol_block_2 = &doc[1];
                {
                    // println!("{:#?}", ol_block_2);
                    let lines =
                        StyleUSSpanLines::from_block(ol_block_2, &Some(style), None);

                    let line_0 = &lines.inner[0];
                    // println!("{}", line_0..pretty_print_debug());
                    assert_eq2!(
                        line_0.inner[0],
                        StyleUSSpan::new(style + get_list_bullet_style(), "200.│",)
                    );
                    assert_eq2!(
                        line_0.inner[1],
                        StyleUSSpan::new(style + get_foreground_style(), "Bar",)
                    );
                }
            });
        }

        #[test]
        fn test_block_ul() -> CommonResult<()> {
            throws!({
                let style = new_style!(
                    color_bg: {tui_color!(red)}
                );
                let (_, doc) = parse_markdown("- Foo\n- Bar\n").into_diagnostic()?;
                println!("{}", fg_cyan(format!("{doc:#?}")));

                // First smart list.
                {
                    let ul_block_0 = &doc[0];
                    let lines =
                        StyleUSSpanLines::from_block(ul_block_0, &Some(style), None);
                    let line_0 = &lines.inner[0];
                    assert_eq2!(
                        line_0.inner[0],
                        StyleUSSpan::new(style + get_list_bullet_style(), "─┤",)
                    );
                    assert_eq2!(
                        line_0.inner[1],
                        StyleUSSpan::new(style + get_foreground_style(), "Foo",)
                    );
                }

                // Second smart list.
                {
                    let ul_block_1 = &doc[1];
                    let lines =
                        StyleUSSpanLines::from_block(ul_block_1, &Some(style), None);
                    let line_0 = &lines.inner[0];
                    assert_eq2!(
                        line_0.inner[0],
                        StyleUSSpan::new(style + get_list_bullet_style(), "─┤",)
                    );
                    assert_eq2!(
                        line_0.inner[1],
                        StyleUSSpan::new(style + get_foreground_style(), "Bar",)
                    );
                }
            });
        }

        #[test]
        fn test_block_text() {
            let text_block = MdElement::Text(list![MdLineFragment::Plain("Foobar")]);
            let style = new_style!(
                color_bg: {tui_color!(red)}
            );

            let lines = StyleUSSpanLines::from_block(&text_block, &Some(style), None);
            // println!("{}", lines..pretty_print_debug());

            let line_0 = &lines.inner[0];
            let span_0_in_line_0 = &line_0.inner[0];
            let StyleUSSpan {
                style, text_gcs, ..
            } = span_0_in_line_0;
            assert_eq2!(text_gcs.as_ref(), "Foobar");
            assert_eq2!(style, &(*style + get_foreground_style()));
        }

        #[test]
        fn test_block_heading() {
            let heading_block = MdElement::Heading(HeadingData {
                level: HeadingLevel { level: 1 },
                text: "Foobar",
            });
            let maybe_style = Some(new_style!(
                color_bg: {tui_color!(red)}
            ));

            let lines = StyleUSSpanLines::from_block(&heading_block, &maybe_style, None);
            // println!("{}", lines..pretty_print_debug());

            // There should just be 1 line.
            assert_eq2!(lines.inner.len(), 1);
            let first_line = &lines.inner[0];
            let spans_in_line = &first_line.inner;

            // There should be 7 spans in the line.
            assert_eq2!(spans_in_line.len(), 7);

            // First span is the heading level `# ` in dim w/ Red bg color, and no fg
            // color.
            assert!(spans_in_line[0].style.dim.is_some());
            assert_eq2!(spans_in_line[0].style.color_bg.unwrap(), tui_color!(red));
            assert_eq2!(spans_in_line[0].style.color_fg.is_some(), false);
            assert_eq2!(spans_in_line[0].text_gcs.as_ref(), "# ");

            // The remainder of the spans are the heading text which are colorized with a
            // color wheel.
            for span in &spans_in_line[1..=6] {
                assert!(span.style.dim.is_none());
                assert_eq2!(span.style.color_bg.unwrap(), tui_color!(red));
                assert_eq2!(span.style.color_fg.is_some(), true);
            }
        }
    }
}
