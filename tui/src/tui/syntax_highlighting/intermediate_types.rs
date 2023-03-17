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

//! This module contains the intermediate types that are used in the process of converting source to
//! syntax highlighted text.
//!
//! These types are used for both:
//! 1. Syntect parser.
//! 2. md_parser_syn_hi, which is a custom R3BL highlighter for md_parser (custom R3BL Markdown
//!    parser).
//!
//! In either case, the source comes from an [crate::editor] component which is a [Vec] of [US]
//! (unicode strings).
//!
//! In either case, this intermediate type is [clipped](StyleUSSpanLine::clip) to the visible area
//! of the editor component (scroll in viewport). And finally that is converted to a
//! [crate::StyledTexts].

use r3bl_rs_utils_core::*;

use crate::*;

/// Spans are chunks of a text that have an associated style. There are usually multiple spans in a
/// line of text.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleUSSpan {
    pub style: Style,
    pub text: US,
}

mod style_us_span_impl {
    use super::*;

    impl StyleUSSpan {
        pub fn new(style: Style, text: US) -> Self { Self { style, text } }
    }

    impl From<(&Style, &US)> for StyleUSSpan {
        fn from((style, text): (&Style, &US)) -> Self { Self::new(*style, text.clone()) }
    }
}

/// A line of text is made up of multiple [StyleUSSpan]s.
pub type StyleUSSpanLine = List<StyleUSSpan>;

/// A document is made up of multiple [StyleUSSpanLine]s.
pub type StyleUSSpanLines = List<StyleUSSpanLine>;

impl StyleUSSpanLine {
    // BM: ▌3. START▐ clip() is the entry point
    /// Clip the text (in one line) in this range: [ `start_col` .. `end_col` ]. Each line is
    /// represented as a [List] of ([Style], [US])`s.
    pub fn clip(
        &self,
        scroll_offset_col_index: ChUnit,
        max_display_col_count: ChUnit,
    ) -> StyledTexts {
        // Populated and returned at the end.
        let mut list: List<StyleUSSpan> = List::default();

        // Clip w/out syntax highlighting & store this as a pattern to match against.
        let plain_text_pattern: &str =
            &self.get_plain_text_clipped(scroll_offset_col_index, max_display_col_count);
        let mut matcher =
            PatternMatcherStateMachine::new(plain_text_pattern, Some(scroll_offset_col_index));

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

        StyledTexts::from(list)
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
        String::from(line.clip(scroll_offset_col_index, max_display_col_count))
    }
}

impl From<StyleUSSpanLine> for StyledTexts {
    fn from(styles: StyleUSSpanLine) -> Self {
        let mut styled_texts = StyledTexts::default();
        for StyleUSSpan { style, text } in styles.iter() {
            styled_texts.push(StyledText::new(text.string.clone(), *style));
        }
        styled_texts
    }
}
