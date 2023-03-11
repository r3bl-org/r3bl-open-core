/*
 *   Copyright (c) 2022 R3BL LLC
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

//! This module contains code for converting between syntect styled texts and tui styled texts.
//!
//! A [Vec] or [List] of styled text represents a single line of text in an editor component, which
//! is the output of a syntax highlighter (that takes plain text and returns the styled text).
//!
//! There is a major difference in doing this conversion which is:
//! - tui styled texts are styled unicode strings,
//! - while syntect styled texts are styled plain text strings.
//!
//! This requires the conversion code to perform the following steps:
//! 1. Convert the syntect [SyntectStyleStrFragmentLine] into a [StyleUSFragmentLine].
//! 2. Then convert [StyleUSFragmentLine] into a [StyledTexts].

use r3bl_rs_utils_core::*;

use crate::*;

pub type SyntectStyle = syntect::highlighting::Style;
/// Fragments are chunks of a text that have an associated style. There are usually multiple
/// fragments in a line of text.
pub type SyntectStyleStrFragment<'a> = (SyntectStyle, &'a str);
/// A line of text is made up of multiple [SyntectStyleStrFragment]s.
pub type SyntectStyleStrFragmentLine<'a> = Vec<SyntectStyleStrFragment<'a>>;

/// Fragments are chunks of a text that have an associated style. There are usually multiple
/// fragments in a line of text.
pub type StyleUSFragment = (Style, US);
/// A line of text is made up of multiple [StyleUSFragment]s.
pub type StyleUSFragmentLine = List<StyleUSFragment>;
/// A document is made up of multiple [StyleUSFragmentLine]s.
pub type StyleUSFragmentLines = List<StyleUSFragmentLine>;

pub fn from_syntect_to_tui(
    syntect_highlighted_line: SyntectStyleStrFragmentLine,
) -> StyleUSFragmentLine {
    let mut it = StyleUSFragmentLine::from(syntect_highlighted_line);

    // Remove the background color from each style in the theme.
    it.iter_mut().for_each(|(style, _)| style.remove_bg_color());

    it
}

mod syntect_support {
    use super::*;

    impl From<SyntectStyleStrFragmentLine<'_>> for StyledTexts {
        fn from(value: SyntectStyleStrFragmentLine) -> Self { StyledTexts::from(&value) }
    }

    impl From<&SyntectStyleStrFragmentLine<'_>> for StyledTexts {
        fn from(styles: &SyntectStyleStrFragmentLine) -> Self {
            let mut styled_texts = StyledTexts::default();
            for (style, text) in styles {
                let my_style = Style::from(*style);
                styled_texts.push(StyledText::new(text.to_string(), my_style));
            }
            styled_texts
        }
    }

    impl From<SyntectStyleStrFragmentLine<'_>> for StyleUSFragmentLine {
        fn from(value: SyntectStyleStrFragmentLine) -> Self {
            pub fn from_vec_styled_str(
                vec_styled_str: &SyntectStyleStrFragmentLine,
            ) -> StyleUSFragmentLine {
                let mut it: StyleUSFragmentLine = Default::default();

                for (style, text) in vec_styled_str {
                    let my_style = Style::from(*style);
                    let unicode_string = US::from(*text);
                    it.push((my_style, unicode_string));
                }

                it
            }

            from_vec_styled_str(&value)
        }
    }
}

mod style_us_support {
    use super::*;

    impl StyleUSFragmentLine {
        pub fn display_width(&self) -> ChUnit {
            let mut size = ch!(0);
            for (_, item) in self.iter() {
                size += item.display_width;
            }
            size
        }

        pub fn get_plain_text(&self) -> String {
            let mut plain_text = String::new();
            for (_, item) in self.iter() {
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
            String::from(line.clip(scroll_offset_col_index, max_display_col_count))
        }

        // BM: ▌3. START▐ clip() is the entry point
        /// Clip the text (in one line) in this range: [ `start_col` .. `end_col` ]. Each line is
        /// represented as a [List] of ([Style], [US])`s.
        pub fn clip(
            &self,
            scroll_offset_col_index: ChUnit,
            max_display_col_count: ChUnit,
        ) -> StyledTexts {
            // Populated and returned at the end.
            let mut list = List::default();

            // Clip w/out syntax highlighting & store this as a pattern to match against.
            let plain_text_pattern: &str =
                &self.get_plain_text_clipped(scroll_offset_col_index, max_display_col_count);
            let mut matcher =
                PatternMatcherStateMachine::new(plain_text_pattern, Some(scroll_offset_col_index));

            // Main loop over each `styled_text_segment` in the `List` (the list represents a single
            // line of text).
            for span in self.iter() {
                let (style, formatted_text_unicode_string) = span;

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
                    list.push((*style, US::from(clipped_text_fragment)));
                }
            }

            StyledTexts::from(list)
        }
    }

    impl From<StyleUSFragmentLine> for StyledTexts {
        fn from(styles: StyleUSFragmentLine) -> Self {
            let mut styled_texts = StyledTexts::default();
            for (style, text) in styles.iter() {
                let my_style: Style = *style;
                styled_texts.push(StyledText::new(text.string.clone(), my_style));
            }
            styled_texts
        }
    }
}
