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
//! This requires the conversion code to:
//! 1. Convert the syntect `Vec<SyntectStyle, &str>` into a `List<Style, UnicodeString>`.
//! 2. Then convert that into a `StyledText`.

use r3bl_rs_utils_core::*;

use crate::*;

type SyntectStyle = syntect::highlighting::Style;

pub fn from_syntect_to_tui(
    vec_styled_str: Vec<(SyntectStyle, &str)>,
) -> List<(Style, UnicodeString)> {
    let mut it: List<(Style, UnicodeString)> = vec_styled_str.into();

    // Remove the background color from each style in the theme.
    it.iter_mut().for_each(|(style, _)| style.remove_bg_color());

    it
}

impl From<Vec<(SyntectStyle, &str)>> for StyledTexts {
    fn from(value: Vec<(SyntectStyle, &str)>) -> Self { (&value).into() }
}

impl From<&Vec<(SyntectStyle, &str)>> for StyledTexts {
    fn from(styles: &Vec<(SyntectStyle, &str)>) -> Self {
        let mut styled_texts = StyledTexts::default();
        for (style, text) in styles {
            let my_style: Style = (*style).into();
            styled_texts.push(StyledText::new(text.to_string(), my_style));
        }
        styled_texts
    }
}

impl From<Vec<(SyntectStyle, &str)>> for List<(Style, UnicodeString)> {
    fn from(value: Vec<(SyntectStyle, &str)>) -> Self {
        pub fn from_vec_styled_str(
            vec_styled_str: &Vec<(SyntectStyle, &str)>,
        ) -> List<(Style, UnicodeString)> {
            let mut it: List<(Style, UnicodeString)> = Default::default();

            for (style, text) in vec_styled_str {
                let my_style: Style = (*style).into();
                let unicode_string: UnicodeString = (*text).into();
                it.push((my_style, unicode_string));
            }

            it
        }

        from_vec_styled_str(&value)
    }
}

impl List<(Style, UnicodeString)> {
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
        let line = UnicodeString::from(self.get_plain_text());
        line.clip(scroll_offset_col_index, max_display_col_count)
    }

    /// Clip the text (in one line) in this range: [ `start_col` .. `end_col` ]. Each line is
    /// represented as a [List] of ([Style], [UnicodeString])`s.
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
            { PatternMatcherStateMachine::new(plain_text_pattern, scroll_offset_col_index.into()) };

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
                list.push((*style, clipped_text_fragment.into()));
            }
        }

        StyledTexts::from(list)
    }
}

impl From<List<(Style, UnicodeString)>> for StyledTexts {
    fn from(styles: List<(Style, UnicodeString)>) -> Self {
        let mut styled_texts = StyledTexts::default();
        for (style, text) in styles.iter() {
            let my_style: Style = *style;
            styled_texts.push(StyledText::new(text.string.clone(), my_style));
        }
        styled_texts
    }
}
