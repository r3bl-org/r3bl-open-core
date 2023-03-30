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
//! 1. Convert the syntect [SyntectStyleStrSpanLine] into a [StyleUSSpanLine].
//! 2. Then convert [StyleUSSpanLine] into a [StyledTexts].

use r3bl_rs_utils_core::*;
use syntect::parsing::SyntaxSet;

use crate::*;

pub type SyntectStyle = syntect::highlighting::Style;

/// Span are chunks of a text that have an associated style. There are usually multiple spans in a
/// line of text.
pub type SyntectStyleStrSpan<'a> = (SyntectStyle, &'a str);

/// A line of text is made up of multiple [SyntectStyleStrSpan]s.
pub type SyntectStyleStrSpanLine<'a> = Vec<SyntectStyleStrSpan<'a>>;

pub fn try_get_syntax_ref<'a>(
    syntax_set: &'a SyntaxSet,
    file_extension: &'a str,
) -> Option<&'a syntect::parsing::SyntaxReference> {
    syntax_set.find_syntax_by_extension(file_extension)
}

// AA: convert RGB to ANSI color: https://github.com/rhysd/rgb2ansi256
pub fn from_syntect_to_tui(syntect_highlighted_line: SyntectStyleStrSpanLine) -> StyleUSSpanLine {
    let mut it = StyleUSSpanLine::from(syntect_highlighted_line);

    // Remove the background color from each style in the theme.
    it.iter_mut()
        .for_each(|StyleUSSpan { style, text: _ }| style.remove_bg_color());

    it
}

mod syntect_support {
    use super::*;

    impl From<SyntectStyleStrSpanLine<'_>> for StyledTexts {
        fn from(value: SyntectStyleStrSpanLine) -> Self { StyledTexts::from(&value) }
    }

    impl From<&SyntectStyleStrSpanLine<'_>> for StyledTexts {
        fn from(syntect_styles: &SyntectStyleStrSpanLine) -> Self {
            let mut acc = StyledTexts::default();
            for (syntect_style, text) in syntect_styles {
                let my_style = Style::from(*syntect_style);
                acc += styled_text!(@style: my_style, @text: text.to_string());
            }
            acc
        }
    }

    impl From<SyntectStyleStrSpanLine<'_>> for StyleUSSpanLine {
        fn from(value: SyntectStyleStrSpanLine) -> Self {
            pub fn from_vec_styled_str(
                vec_styled_str: &SyntectStyleStrSpanLine,
            ) -> StyleUSSpanLine {
                let mut it: StyleUSSpanLine = Default::default();

                for (style, text) in vec_styled_str {
                    let my_style = Style::from(*style);
                    let unicode_string = US::from(*text);
                    it.push(StyleUSSpan::new(my_style, unicode_string));
                }

                it
            }

            from_vec_styled_str(&value)
        }
    }
}
