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

//! This module contains the intermediate types that are used in the process of converting
//! the source to syntax-highlighted text.
//!
//! These types are used for both:
//! 1. Syntect parser.
//! 2. `md_parser_syn_hi`, which is a custom R3BL highlighter for `md_parser` (custom R3BL
//!    Markdown parser).
//!
//! In both cases:
//! 1. The source document comes from a [`crate::editor`] component, which is a [Vec] of
//!    [`GCStringOwned`] (Unicode strings).
//! 2. This intermediate type is [clipped](StyleUSSpanLine::clip) to the visible area of
//!    the editor component (based on scroll state in the viewport). And finally that is
//!    converted to a [`crate::TuiStyledTexts`].

use crate::{get_foreground_dim_style, get_metadata_tags_marker_style,
            get_metadata_tags_values_style, get_metadata_title_marker_style,
            get_metadata_title_value_style,
            md_parser::constants::{COLON, COMMA, SPACE},
            tiny_inline_string, tui_styled_text, width, CharacterMatchResult, ColIndex,
            ColWidth, GCStringOwned, InlineString, List,
            PatternMatcherStateMachine, ScrOfs, TuiStyle, TuiStyledTexts};

/// Spans are chunks of a text that have an associated style. There are usually multiple
/// spans in a line of text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct StyleUSSpan {
    pub style: TuiStyle,
    pub text_gcs: GCStringOwned,
}

impl Default for StyleUSSpan {
    fn default() -> Self {
        Self {
            style: TuiStyle::default(),
            text_gcs: "".into(),
        }
    }
}

impl StyleUSSpan {
    #[must_use]
    pub fn new(style: TuiStyle, arg_text: &str) -> Self {
        Self {
            style,
            text_gcs: arg_text.into(),
        }
    }
}

/// A line of text is made up of multiple [`StyleUSSpan`]s.
pub type StyleUSSpanLine = List<StyleUSSpan>;

/// A document is made up of multiple [`StyleUSSpanLine`]s.
pub type StyleUSSpanLines = List<StyleUSSpanLine>;

impl StyleUSSpanLine {
    /// Eg: "@tags: [tag1, tag2, tag3]"
    #[must_use]
    pub fn from_csvp(
        key: &str,
        tag_list: &List<&'_ str>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut acc_line_output = StyleUSSpanLine::default();
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_tags_marker_style(),
            key,
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_foreground_dim_style(),
            &tiny_inline_string!("{COLON}{SPACE}"),
        );
        for (index, span) in tag_list.iter().enumerate() {
            acc_line_output += StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default()
                    + get_metadata_tags_values_style(),
                span,
            );
            // Not the last item in the iterator.
            if index != (tag_list.len() - 1) {
                acc_line_output += StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    &tiny_inline_string!("{COMMA}{SPACE}"),
                );
            }
        }

        acc_line_output
    }

    /// Eg: "@title: Something"
    #[must_use]
    pub fn from_kvp(
        key: &str,
        text: &str,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut acc_line_output = StyleUSSpanLine::default();
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_title_marker_style(),
            key,
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_foreground_dim_style(),
            &tiny_inline_string!("{COLON}{SPACE}"),
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_title_value_style(),
            text,
        );

        acc_line_output
    }

    /// This applies the given style to every single item in the list. It has the highest
    /// specificity.
    pub fn add_style(&mut self, style: TuiStyle) {
        for StyleUSSpan { style: s, .. } in self.iter_mut() {
            *s += style;
        }
    }

    /// Clip the text (in one line) in this range: [ `start_col` .. `end_col` ]. Each line
    /// is represented as a [List] of ([`TuiStyle`], [`GCStringOwned`])'s.
    #[must_use]
    pub fn clip(
        &self,
        scr_ofs: ScrOfs,
        max_display_col_count: ColWidth,
    ) -> TuiStyledTexts {
        let scroll_offset_col_index = scr_ofs.col_index;

        // Populated and returned at the end.
        let mut list: List<StyleUSSpan> = List::default();

        // Clip w/out syntax highlighting & store this as a pattern to match against.
        let plain_text_pattern: &str =
            &self.get_plain_text_clipped(scroll_offset_col_index, max_display_col_count);
        let mut matcher = PatternMatcherStateMachine::new(
            plain_text_pattern,
            Some(scroll_offset_col_index),
        );

        // Main loop over each `styled_text_segment` in the `List` (the list represents a
        // single line of text).
        for span in self.iter() {
            let StyleUSSpan { style, text_gcs } = span;

            let mut clipped_text_fragment = InlineString::new();

            for seg_str in text_gcs {
                for character in seg_str.chars() {
                    match matcher.match_next(character) {
                        CharacterMatchResult::Keep => {
                            clipped_text_fragment.push(character);
                            /* continue */
                        }
                        CharacterMatchResult::Reset => {
                            clipped_text_fragment.clear();
                            /* continue */
                        }
                        CharacterMatchResult::ResetAndKeep => {
                            clipped_text_fragment.clear();
                            clipped_text_fragment.push(character);
                            /* continue */
                        }
                        CharacterMatchResult::Skip => { /* continue */ }
                        CharacterMatchResult::Finished => {
                            break;
                        }
                    }
                }
            }

            if !clipped_text_fragment.is_empty() {
                list.push(StyleUSSpan::new(*style, clipped_text_fragment.as_str()));
            }
        }

        TuiStyledTexts::from(list)
    }

    #[must_use]
    pub fn display_width(&self) -> ColWidth {
        let mut display_width = width(0);
        for span in self.iter() {
            display_width += span.text_gcs.display_width;
        }
        display_width
    }

    #[must_use]
    pub fn get_plain_text(&self) -> InlineString {
        let mut plain_text_acc = InlineString::new();
        for span in self.iter() {
            let str = span.text_gcs.as_ref();
            plain_text_acc.push_str(str);
        }
        plain_text_acc
    }

    /// Clip the content `[scroll_offset.col .. max cols]`.
    #[must_use]
    pub fn get_plain_text_clipped(
        &self,
        scroll_offset_col_index: ColIndex,
        max_display_col_count: ColWidth,
    ) -> InlineString {
        let line = self.get_plain_text();
        let line_gcs: GCStringOwned = line.into();
        let str = line_gcs.clip(scroll_offset_col_index, max_display_col_count);
        InlineString::from(str)
    }
}

mod convert {
    use super::{tui_styled_text, StyleUSSpan, StyleUSSpanLine, TuiStyle, TuiStyledTexts};

    impl From<(&TuiStyle, &str)> for StyleUSSpan {
        fn from((style, text): (&TuiStyle, &str)) -> Self { Self::new(*style, text) }
    }

    impl From<StyleUSSpanLine> for TuiStyledTexts {
        fn from(styles: StyleUSSpanLine) -> Self {
            let mut acc = TuiStyledTexts::default();
            for StyleUSSpan {
                style, text_gcs, ..
            } in styles.iter()
            {
                acc += tui_styled_text!(@style: *style, @text: text_gcs.string);
            }
            acc
        }
    }
}

/// Make sure that the code to clip styled text to a range [ `start_col` .`end_col`ol ]
/// works. The list of styled unicode string represents a single line of text in an editor
/// component.
#[cfg(test)]
mod tests_clip_styled_texts {
    use super::*;
    use crate::{assert_eq2, ch, col, list, row, scr_ofs, tui_color, ChUnitPrimitiveType,
                ConvertToPlainText, List};

    mod fixtures {
        use super::*;
        use crate::new_style;

        pub fn get_s1() -> TuiStyle {
            new_style!(
                id: {1}
                color_bg: {tui_color!(1, 1, 1)}
            )
        }

        pub fn get_s2() -> TuiStyle {
            new_style!(
                id: {2}
                color_bg: {tui_color!(2, 2, 2)}
            )
        }

        /// A struct containing the following pseudo HTML representation is returned.
        ///
        /// ```text
        /// <span style="s1">first</span>
        /// <span style="s1"> </span>
        /// <span style="s2">second</span>
        /// ```
        pub fn get_list() -> List<StyleUSSpan> {
            list! {
                StyleUSSpan::new(get_s1(), "first"),
                StyleUSSpan::new(get_s1(), " "),
                StyleUSSpan::new(get_s2(), "second"),
            }
        }
    }

    /// ```text
    /// BEFORE:
    ///    ╭s1
    ///    │    ╭s2
    ///    │    │╭s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 2 .. 5 ].
    ///      ╭s1
    ///      │  ╭s2
    ///      │  │╭s3
    /// R   ┌─────┐
    /// 0 fi│rst s│econd
    ///     └─────┘
    ///     C01234 5678901
    /// ```
    #[test]
    fn list_1_range_2_5() {
        assert_eq2!(fixtures::get_list().len(), 3);

        let scroll_offset_col_index = ch(2);
        let max_display_col_count = ch(5);
        let expected_clipped_string = "rst s";

        // Equivalent no highlight version.
        {
            let text = TuiStyledTexts::from(fixtures::get_list()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped =
                fixtures::get_list().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 3);
            let lhs = clipped.to_plain_text();
            assert_eq2!(lhs, expected_clipped_string);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ╭s1
    ///    │    ╭s2
    ///    │    │╭s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 0 .. 3 ].
    ///    ╭s1
    ///    │     ╭s2
    ///    │     │╭s3
    /// R ┌───┐
    /// 0 │fir│st second
    ///   └───┘
    ///   C012 345678901
    /// ```
    #[test]
    fn list_1_range_0_3() {
        assert_eq2!(fixtures::get_list().len(), 3);

        let scroll_offset_col_index = ch(0);
        let max_display_col_count = ch(3);
        let expected_clipped_string = "fir";

        // Equivalent no highlight version.
        {
            let text = TuiStyledTexts::from(fixtures::get_list()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped =
                fixtures::get_list().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let left = clipped.to_plain_text();
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ╭s1
    ///    │    ╭s2
    ///    │    │╭s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 0 .. 5 ].
    ///    ╭s1
    ///    │     ╭s2
    ///    │     │╭s3
    /// R ┌─────┐
    /// 0 │first│ second
    ///   └─────┘
    ///   C01234 5678901
    /// ```
    #[test]
    fn list_1_range_0_5() {
        assert_eq2!(fixtures::get_list().len(), 3);

        let scroll_offset_col_index = ch(0);
        let max_display_col_count = ch(5);
        let expected_clipped_string = "first";

        // Equivalent no highlight version.
        {
            let text = TuiStyledTexts::from(fixtures::get_list()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped =
                fixtures::get_list().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let lhs = clipped.to_plain_text();
            let rhs = expected_clipped_string;
            assert_eq2!(lhs, rhs);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ╭s1
    ///    │    ╭s2
    ///    │    │╭s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 2 .. 8 ].
    ///      ╭s1
    ///      │  ╭s2
    ///      │  │╭s3
    /// R   ┌────────┐
    /// 0 fi│rst seco│nd
    ///     └────────┘
    ///     C01234567 8901
    /// ```
    #[test]
    fn list_1_range_2_8() {
        assert_eq2!(fixtures::get_list().len(), 3);

        let scroll_offset_col_index = ch(2);
        let max_display_col_count = ch(8);
        let expected_clipped_string = "rst seco";

        // Expected no highlight version.
        {
            let text = TuiStyledTexts::from(fixtures::get_list()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped =
                fixtures::get_list().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 3);
            let left = clipped.to_plain_text();
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }

    #[test]
    fn list_2() {
        fn get_list_alt() -> List<StyleUSSpan> {
            list! {
                StyleUSSpan::new(
                    fixtures::get_s1(),
                    "01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234",
                )
            }
        }

        let scroll_offset_col_index = ch(1);
        let max_display_col_count = ch(77);
        let expected_clipped_string =
                "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234";

        // BEFORE:
        // ╭0
        // │ │
        // ╭77 │ .................................................................
        // ............   │ viewport 01234567890 01234567890 01234567890
        // 01234567890 01234567890 01234567890 01234
        //
        // AFTER:
        // ╭0
        // │ │
        // ╭77 │ .................................................................
        // ............   │ viewport 1234567890 01234567890 01234567890
        // 01234567890 01234567890 01234567890 01234

        // Expected no highlight version.
        {
            let text = TuiStyledTexts::from(get_list_alt()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped = get_list_alt().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let lhs = clipped.to_plain_text();
            let rhs = expected_clipped_string;
            assert_eq2!(lhs, rhs);
        }
    }

    #[test]
    fn list_3() {
        fn get_list_alt() -> List<StyleUSSpan> {
            list! {
                StyleUSSpan::new(
                    fixtures::get_s1(),
                    "01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 0123456",
                )
            }
        }

        let scroll_offset_col_index = ch(1);
        let max_display_col_count = ch(77);
        let expected_clipped_string =
                "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 012345";

        // BEFORE:
        // ╭0
        // │ │
        // ╭77 │ .................................................................
        // ............   │ viewport 01234567890 01234567890 01234567890
        // 01234567890 01234567890 01234567890 0123456
        //
        // AFTER:
        // ╭0
        // │ │
        // ╭77 │ .................................................................
        // ............   │ viewport 1234567890 01234567890 01234567890
        // 01234567890 01234567890 01234567890 012345

        // Expected no highlight version.
        {
            let text = TuiStyledTexts::from(get_list_alt()).to_plain_text();
            let text_gcs: GCStringOwned = text.into();

            let trunc_1_str = text_gcs.trunc_start_by(width(*scroll_offset_col_index));
            let trunc_1_gcs: GCStringOwned = trunc_1_str.into();

            let trunc_2_str = trunc_1_gcs.trunc_end_to_fit(width(*max_display_col_count));
            assert_eq2!(trunc_2_str, expected_clipped_string);
        }

        // clip version.
        {
            let scr_ofs = scr_ofs(
                /* just need this col_index */
                col(*scroll_offset_col_index) +
                /* row is not used, so set it to an improbable value */
                row(ChUnitPrimitiveType::MAX),
            );
            let clipped = get_list_alt().clip(scr_ofs, width(*max_display_col_count));
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let left = clipped.to_plain_text();
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }
}
