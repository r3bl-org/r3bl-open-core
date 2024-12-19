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

//! This module contains the intermediate types that are used in the process of converting
//! source to syntax highlighted text.
//!
//! These types are used for both:
//! 1. Syntect parser.
//! 2. md_parser_syn_hi, which is a custom R3BL highlighter for md_parser (custom R3BL
//!    Markdown parser).
//!
//! In both cases:
//! 1. The source document comes from an [crate::editor] component which is a [Vec] of
//!    [US] (unicode strings).
//! 2. This intermediate type is [clipped](StyleUSSpanLine::clip) to the visible area of
//!    the editor component (based on scroll state in viewport). And finally that is
//!    converted to a [r3bl_core::TuiStyledTexts].

use r3bl_core::{ch, tui_styled_text, ChUnit, TuiStyle, TuiStyledTexts};

use crate::{constants::{COLON, COMMA, SPACE},
            get_foreground_dim_style,
            get_metadata_tags_marker_style,
            get_metadata_tags_values_style,
            get_metadata_title_marker_style,
            get_metadata_title_value_style,
            CharacterMatchResult,
            List,
            PatternMatcherStateMachine,
            US};

/// Spans are chunks of a text that have an associated style. There are usually multiple spans in a
/// line of text.
#[derive(Default, Clone, PartialEq, Eq, Debug, size_of::SizeOf)]
pub struct StyleUSSpan {
    pub style: TuiStyle,
    pub text: US,
}

impl StyleUSSpan {
    pub fn new(style: TuiStyle, text: US) -> Self { Self { style, text } }
}

/// A line of text is made up of multiple [StyleUSSpan]s.
pub type StyleUSSpanLine = List<StyleUSSpan>;

/// A document is made up of multiple [StyleUSSpanLine]s.
pub type StyleUSSpanLines = List<StyleUSSpanLine>;

impl StyleUSSpanLine {
    /// Eg: "@tags: [tag1, tag2, tag3]"
    pub fn from_csvp(
        key: &str,
        tag_list: &List<&'_ str>,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut acc_line_output = StyleUSSpanLine::default();
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_tags_marker_style(),
            US::from(key),
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_foreground_dim_style(),
            US::from(format!("{COLON}{SPACE}")),
        );
        for (index, span) in tag_list.iter().enumerate() {
            acc_line_output += StyleUSSpan::new(
                maybe_current_box_computed_style.unwrap_or_default()
                    + get_metadata_tags_values_style(),
                US::from(*span),
            );
            // Not the last item in the iterator.
            if index != (tag_list.len() - 1) {
                acc_line_output += StyleUSSpan::new(
                    maybe_current_box_computed_style.unwrap_or_default()
                        + get_foreground_dim_style(),
                    US::from(format!("{COMMA}{SPACE}")),
                );
            }
        }

        acc_line_output
    }

    /// Eg: "@title: Something"
    pub fn from_kvp(
        key: &str,
        text: &str,
        maybe_current_box_computed_style: &Option<TuiStyle>,
    ) -> Self {
        let mut acc_line_output = StyleUSSpanLine::default();
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_title_marker_style(),
            US::from(key),
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_foreground_dim_style(),
            US::from(format!("{COLON}{SPACE}")),
        );
        acc_line_output += StyleUSSpan::new(
            maybe_current_box_computed_style.unwrap_or_default()
                + get_metadata_title_value_style(),
            US::from(text),
        );

        acc_line_output
    }

    /// This applies the given style to every single item in the list. It has the highest
    /// specificity.
    pub fn add_style(&mut self, style: TuiStyle) {
        for StyleUSSpan { style: s, text: _ } in self.iter_mut() {
            *s += style;
        }
    }

    /// Clip the text (in one line) in this range: [ `start_col` .. `end_col` ]. Each line is
    /// represented as a [List] of ([TuiStyle], [US])`s.
    pub fn clip(
        &self,
        scroll_offset_col_index: ChUnit,
        max_display_col_count: ChUnit,
    ) -> TuiStyledTexts {
        // Populated and returned at the end.
        let mut list: List<StyleUSSpan> = List::default();

        // Clip w/out syntax highlighting & store this as a pattern to match against.
        let plain_text_pattern: &str =
            &self.get_plain_text_clipped(scroll_offset_col_index, max_display_col_count);
        let mut matcher = PatternMatcherStateMachine::new(
            plain_text_pattern,
            Some(scroll_offset_col_index),
        );

        // Main loop over each `styled_text_segment` in the `List` (the list represents a single
        // line of text).
        for span in self.iter() {
            let StyleUSSpan {
                style,
                text: formatted_text_unicode_string,
            } = span;

            let mut clipped_text_fragment = String::new();

            for segment in formatted_text_unicode_string.iter() {
                for character in segment.string.chars() {
                    match matcher.match_next(character) {
                        CharacterMatchResult::Keep => {
                            clipped_text_fragment.push(character);
                            continue;
                        }
                        CharacterMatchResult::Reset => {
                            clipped_text_fragment.clear();
                            continue;
                        }
                        CharacterMatchResult::ResetAndKeep => {
                            clipped_text_fragment.clear();
                            clipped_text_fragment.push(character);
                            continue;
                        }
                        CharacterMatchResult::Finished => {
                            break;
                        }
                        CharacterMatchResult::Skip => {
                            continue;
                        }
                    }
                }
            }

            if !clipped_text_fragment.is_empty() {
                list.push(StyleUSSpan::new(*style, US::from(clipped_text_fragment)));
            }
        }

        TuiStyledTexts::from(list)
    }

    pub fn display_width(&self) -> ChUnit {
        let mut size = ch!(0);
        for StyleUSSpan {
            style: _,
            text: item,
        } in self.iter()
        {
            size += item.display_width;
        }
        size
    }

    pub fn get_plain_text(&self) -> String {
        let mut plain_text = String::new();
        for StyleUSSpan {
            style: _,
            text: item,
        } in self.iter()
        {
            plain_text.push_str(&item.string);
        }
        plain_text
    }

    /// Clip the content [scroll_offset.col .. max cols].
    pub fn get_plain_text_clipped(
        &self,
        scroll_offset_col_index: ChUnit,
        max_display_col_count: ChUnit,
    ) -> String {
        let line = US::from(self.get_plain_text());
        String::from(line.clip_to_width(scroll_offset_col_index, max_display_col_count))
    }
}

mod convert {
    use super::*;

    impl From<(&TuiStyle, &US)> for StyleUSSpan {
        fn from((style, text): (&TuiStyle, &US)) -> Self {
            Self::new(*style, text.clone())
        }
    }

    impl From<StyleUSSpanLine> for TuiStyledTexts {
        fn from(styles: StyleUSSpanLine) -> Self {
            let mut acc = TuiStyledTexts::default();
            for StyleUSSpan { style, text } in styles.iter() {
                acc += tui_styled_text!(@style: *style, @text: text.string.clone());
            }
            acc
        }
    }
}

/// Make sure that the code to clip styled text to a range [ start_col .. end_col ] works. The
/// list of styled unicode string represents a single line of text in an editor component.
#[cfg(test)]
mod tests_clip_styled_texts {
    use r3bl_core::{assert_eq2, ConvertToPlainText, RgbValue, TuiColor, UnicodeString};
    use r3bl_macro::tui_style;

    use super::*;
    use crate::{list, List};

    mod fixtures {
        use super::*;

        pub fn get_s1() -> TuiStyle {
            tui_style! {
              id: 1
              color_bg: TuiColor::Rgb (RgbValue{ red: 1, green: 1, blue: 1 })
            }
        }

        pub fn get_s2() -> TuiStyle {
            tui_style! {
              id: 2
              color_bg: TuiColor::Rgb(RgbValue{ red: 2, green: 2, blue: 2 })
            }
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
                StyleUSSpan::new(get_s1(), UnicodeString::from("first")),
                StyleUSSpan::new(get_s1(), UnicodeString::from(" ")),
                StyleUSSpan::new(get_s2(), UnicodeString::from("second"))
            }
        }
    }

    /// ```text
    /// BEFORE:
    ///    ┌→s1
    ///    │    ┌→s2
    ///    │    │┌→s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 2 .. 5 ].
    ///      ┌→s1
    ///      │  ┌→s2
    ///      │  │┌→s3
    /// R   ┌─────┐
    /// 0 fi│rst s│econd
    ///     └─────┘
    ///     C01234 5678901
    /// ```
    #[test]
    fn list_1_range_2_5() {
        use fixtures::*;

        assert_eq2!(get_list().len(), 3);

        let scroll_offset_col_index = ch!(2);
        let max_display_col_count = ch!(5);
        let expected_clipped_string = "rst s";

        // Equivalent no highlight version.
        {
            let line = TuiStyledTexts::from(get_list()).to_plain_text_us().string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 3);
            let lhs = clipped.to_plain_text_us().string;
            assert_eq2!(lhs, expected_clipped_string);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ┌→s1
    ///    │    ┌→s2
    ///    │    │┌→s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 0 .. 3 ].
    ///    ┌→s1
    ///    │     ┌→s2
    ///    │     │┌→s3
    /// R ┌───┐
    /// 0 │fir│st second
    ///   └───┘
    ///   C012 345678901
    /// ```
    #[test]
    fn list_1_range_0_3() {
        use fixtures::*;

        assert_eq2!(get_list().len(), 3);

        let scroll_offset_col_index = ch!(0);
        let max_display_col_count = ch!(3);
        let expected_clipped_string = "fir";

        // Equivalent no highlight version.
        {
            let line = TuiStyledTexts::from(fixtures::get_list())
                .to_plain_text_us()
                .string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped =
                fixtures::get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let left = clipped.to_plain_text_us().string;
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ┌→s1
    ///    │    ┌→s2
    ///    │    │┌→s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 0 .. 5 ].
    ///    ┌→s1
    ///    │     ┌→s2
    ///    │     │┌→s3
    /// R ┌─────┐
    /// 0 │first│ second
    ///   └─────┘
    ///   C01234 5678901
    /// ```
    #[test]
    fn list_1_range_0_5() {
        use fixtures::*;

        assert_eq2!(get_list().len(), 3);

        let scroll_offset_col_index = ch!(0);
        let max_display_col_count = ch!(5);
        let expected_clipped_string = "first";

        // Equivalent no highlight version.
        {
            let line = TuiStyledTexts::from(fixtures::get_list())
                .to_plain_text_us()
                .string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped =
                fixtures::get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let lhs = clipped.to_plain_text_us().string;
            let rhs = expected_clipped_string;
            assert_eq2!(lhs, rhs);
        }
    }

    /// ```text
    /// BEFORE:
    ///    ┌→s1
    ///    │    ┌→s2
    ///    │    │┌→s3
    ///    ▒▒▒▒▒█▒▒▒▒▒▒
    /// R ┌────────────┐
    /// 0 │first second│
    ///   └────────────┘
    ///   C012345678901
    ///
    /// AFTER: Cut [ 2 .. 8 ].
    ///      ┌→s1
    ///      │  ┌→s2
    ///      │  │┌→s3
    /// R   ┌────────┐
    /// 0 fi│rst seco│nd
    ///     └────────┘
    ///     C01234567 8901
    /// ```
    #[test]
    fn list_1_range_2_8() {
        use fixtures::*;

        assert_eq2!(get_list().len(), 3);

        let scroll_offset_col_index = ch!(2);
        let max_display_col_count = ch!(8);
        let expected_clipped_string = "rst seco";

        // Expected no highlight version.
        {
            let line = TuiStyledTexts::from(fixtures::get_list())
                .to_plain_text_us()
                .string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped =
                fixtures::get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 3);
            let left = clipped.to_plain_text_us().string;
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }

    #[test]
    fn list_2() {
        use fixtures::*;

        fn get_list() -> List<StyleUSSpan> {
            list! {
                StyleUSSpan::new(
                    get_s1(),
                    UnicodeString::from(
                        "01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234",
                    ),
                )
            }
        }

        let scroll_offset_col_index = ch!(1);
        let max_display_col_count = ch!(77);
        let expected_clipped_string =
                "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234";

        // BEFORE:
        // ┌→0                                                                              │
        // │                                                                           ┌→77 │
        // .............................................................................    │ viewport
        // 01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234
        //
        // AFTER:
        // ┌→0                                                                              │
        // │                                                                           ┌→77 │
        // .............................................................................    │ viewport
        // 1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 01234

        // Expected no highlight version.
        {
            let line = TuiStyledTexts::from(get_list()).to_plain_text_us().string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let lhs = clipped.to_plain_text_us().string;
            let rhs = expected_clipped_string;
            assert_eq2!(lhs, rhs);
        }
    }

    #[test]
    fn list_3() {
        use fixtures::*;

        fn get_list() -> List<StyleUSSpan> {
            list! {
                StyleUSSpan::new(
                    get_s1(),
                    UnicodeString::from(
                        "01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 0123456",
                    ),
                )
            }
        }

        let scroll_offset_col_index = ch!(1);
        let max_display_col_count = ch!(77);
        let expected_clipped_string =
                "1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 012345";

        // BEFORE:
        // ┌→0                                                                              │
        // │                                                                           ┌→77 │
        // .............................................................................    │ viewport
        // 01234567890 01234567890 01234567890 01234567890 01234567890 01234567890 0123456
        //
        // AFTER:
        // ┌→0                                                                              │
        // │                                                                           ┌→77 │
        // .............................................................................    │ viewport
        // 1234567890 01234567890 01234567890 01234567890 01234567890 01234567890 012345

        // Expected no highlight version.
        {
            let line = TuiStyledTexts::from(get_list()).to_plain_text_us().string;
            let line = UnicodeString::from(line);
            let truncated_line = line.truncate_start_by_n_col(scroll_offset_col_index);
            let truncated_line = UnicodeString::from(truncated_line);
            let truncated_line =
                truncated_line.truncate_end_to_fit_width(max_display_col_count);
            assert_eq2!(truncated_line, expected_clipped_string);
        }

        // clip version.
        {
            let clipped = get_list().clip(scroll_offset_col_index, max_display_col_count);
            // println!("{}", clipped.pretty_print_debug());
            assert_eq2!(clipped.len(), 1);
            let left = clipped.to_plain_text_us().string;
            let right = expected_clipped_string;
            assert_eq2!(left, right);
        }
    }
}
